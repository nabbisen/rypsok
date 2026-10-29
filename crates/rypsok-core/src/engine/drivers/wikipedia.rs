//! The Wikipedia driver: the MediaWiki Action API search list of the
//! language edition, keyless (admission record:
//! `docs/src/maintainers/engines/wikipedia.md`).

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

use super::{DriverBuildError, DriverContext, debug_provider_text, is_loopback, retry_after};
use crate::CancellationToken;
use crate::config::EngineTable;
use crate::egress::{self, BodyError};
use crate::engine::{
    Driver, EngineError, EngineErrorClass, EngineProperties, EngineRequest, EngineResponse,
    LanguageSupport, RawResult, TimeRangeSupport, UserAgentFormat,
};
use crate::model::Date;

/// The driver type name.
pub const KIND: &str = "wikipedia";

/// The properties from the admission record.
pub const PROPERTIES: EngineProperties = EngineProperties {
    max_query_chars: 250,
    max_query_words: None,
    languages: LanguageSupport::All,
    time_ranges: TimeRangeSupport::None,
    requests_per_minute: 100,
    max_in_flight: 1,
    cacheable: true,
    attribution_required: false,
    user_agent: UserAgentFormat::Standard,
    max_response_bytes: 1_048_576,
    scraping: false,
};

/// The Wikipedia driver.
#[derive(Debug)]
pub struct Wikipedia {
    client: reqwest::Client,
    base_override: Option<url::Url>,
}

impl Wikipedia {
    /// Builds the driver with its client. A `base_url` key replaces the
    /// public endpoint (a private mirror or a loopback test server); plain
    /// HTTP is admitted for a loopback base only.
    pub fn new(table: &EngineTable, context: &DriverContext) -> Result<Self, DriverBuildError> {
        let base_override = match (table.key_str("base_url"), &context.base_override) {
            (Some(text), _) => {
                let url = url::Url::parse(text).map_err(|_| {
                    DriverBuildError::Key("base_url".into(), "must be an absolute URL".into())
                })?;
                if url.scheme() != "https" && !is_loopback(&url) {
                    return Err(DriverBuildError::Key(
                        "base_url".into(),
                        "must be https unless it is a loopback address".into(),
                    ));
                }
                Some(url)
            }
            (None, base) => base.clone(),
        };
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("application/json"));
        let mut settings = context.client_settings(headers);
        if base_override.as_ref().is_some_and(is_loopback) {
            settings.https_only = false;
        }
        let client = egress::engine_client(&settings)?;
        Ok(Wikipedia {
            client,
            base_override,
        })
    }

    fn endpoint(&self, language: &str) -> Result<url::Url, EngineError> {
        let base = match &self.base_override {
            Some(base) => base.clone(),
            None => url::Url::parse(&format!("https://{language}.wikipedia.org/"))
                .map_err(|_| EngineError::new(EngineErrorClass::RequestFailure))?,
        };
        base.join("w/api.php")
            .map_err(|_| EngineError::new(EngineErrorClass::RequestFailure))
    }

    fn page_url(&self, language: &str, title: &str) -> Option<String> {
        let base = match &self.base_override {
            Some(base) => base.clone(),
            None => url::Url::parse(&format!("https://{language}.wikipedia.org/")).ok()?,
        };
        let mut url = base.join("wiki/").ok()?;
        url.path_segments_mut()
            .ok()?
            .pop_if_empty()
            .push(&title.replace(' ', "_"));
        Some(url.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    #[serde(default)]
    query: Option<Query>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Query {
    #[serde(default)]
    search: Vec<Hit>,
}

#[derive(Debug, Deserialize)]
struct Hit {
    title: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    timestamp: Option<String>,
}

#[async_trait::async_trait]
impl Driver for Wikipedia {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn properties(&self) -> &EngineProperties {
        &PROPERTIES
    }

    async fn search(
        &self,
        req: &EngineRequest,
        cancel: CancellationToken,
    ) -> Result<EngineResponse, EngineError> {
        let language = req.language.primary().to_owned();
        let endpoint = self.endpoint(&language)?;
        let limit = req.max_results.clamp(1, 50).to_string();
        let request = self.client.get(endpoint).query(&[
            ("action", "query"),
            ("list", "search"),
            ("srsearch", req.query.expose()),
            ("srlimit", limit.as_str()),
            ("srprop", "snippet|timestamp"),
            ("format", "json"),
        ]);
        let send = request.send();
        let mut response = tokio::select! {
            _ = cancel.cancelled() => return Err(EngineError::new(EngineErrorClass::RequestFailure)),
            sent = send => sent.map_err(classify_transport)?,
        };
        let status = response.status().as_u16();
        let body = tokio::select! {
            _ = cancel.cancelled() => return Err(EngineError::new(EngineErrorClass::RequestFailure)),
            read = egress::read_body(&mut response, PROPERTIES.max_response_bytes) => read.map_err(|e| match e {
                BodyError::TooLarge => EngineError::new(EngineErrorClass::ParseFailure),
                BodyError::Transport => EngineError::new(EngineErrorClass::RequestFailure),
            })?,
        };
        match status {
            200 => {}
            429 => {
                return Err(EngineError {
                    class: EngineErrorClass::RateLimited,
                    retry_after: retry_after(response.headers(), "retry-after"),
                });
            }
            403 => {
                debug_provider_text(KIND, status, &body);
                return Err(EngineError::new(EngineErrorClass::Blocked));
            }
            _ => {
                debug_provider_text(KIND, status, &body);
                return Err(EngineError::new(EngineErrorClass::RequestFailure));
            }
        }
        let parsed: ApiResponse = serde_json::from_slice(&body).map_err(|_| {
            debug_provider_text(KIND, status, &body);
            EngineError::new(EngineErrorClass::ParseFailure)
        })?;
        if parsed.error.is_some() {
            debug_provider_text(KIND, status, &body);
            return Err(EngineError::new(EngineErrorClass::RequestFailure));
        }
        let hits = parsed.query.map(|q| q.search).unwrap_or_default();
        let results = hits
            .into_iter()
            .filter_map(|hit| {
                let url = self.page_url(&language, &hit.title)?;
                let date = hit
                    .timestamp
                    .as_deref()
                    .and_then(|t| Date::new(t.get(..10)?).ok());
                Some(RawResult {
                    date,
                    ..RawResult::new(hit.title, url, hit.snippet)
                })
            })
            .collect();
        Ok(EngineResponse { results })
    }
}

fn classify_transport(error: reqwest::Error) -> EngineError {
    if error.is_timeout() {
        EngineError::new(EngineErrorClass::Timeout)
    } else {
        EngineError::new(EngineErrorClass::RequestFailure)
    }
}

#[cfg(test)]
mod tests;
