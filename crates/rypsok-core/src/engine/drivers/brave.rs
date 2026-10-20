//! The Brave Search driver: the Web Search and News Search endpoints,
//! keyed by `X-Subscription-Token` (admission record:
//! `docs/src/maintainers/engines/brave.md`).

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

use super::{DriverBuildError, DriverContext, debug_provider_text, retry_after};
use crate::CancellationToken;
use crate::config::{EngineTable, SafeSearch, Secret};
use crate::egress::{self, BodyError};
use crate::engine::{
    Driver, EngineError, EngineErrorClass, EngineProperties, EngineRequest, EngineResponse,
    LanguageSupport, RawResult, TimeRangeSupport, UserAgentFormat,
};
use crate::model::{Date, TimeRange};

/// The driver type name.
pub const KIND: &str = "brave";

/// The properties from the admission record.
pub const PROPERTIES: EngineProperties = EngineProperties {
    max_query_chars: 600,
    max_query_words: Some(75),
    languages: LanguageSupport::All,
    time_ranges: TimeRangeSupport::All,
    requests_per_minute: 60,
    max_in_flight: 2,
    cacheable: false,
    attribution_required: false,
    user_agent: UserAgentFormat::Standard,
    max_response_bytes: 1_048_576,
};

/// Which endpoint an engine queries (`endpoint` key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endpoint {
    /// `/res/v1/web/search`.
    Web,
    /// `/res/v1/news/search`.
    News,
}

/// The Brave driver.
#[derive(Debug)]
pub struct Brave {
    client: reqwest::Client,
    endpoint: Endpoint,
    base_override: Option<url::Url>,
}

impl Brave {
    /// Builds the driver with its client; the credential goes on the
    /// client, never on a request, so that it reaches one origin only.
    pub fn new(
        table: &EngineTable,
        secret: Option<&Secret<String>>,
        context: &DriverContext,
    ) -> Result<Self, DriverBuildError> {
        let endpoint = match table.key_str("endpoint") {
            None | Some("web") => Endpoint::Web,
            Some("news") => Endpoint::News,
            Some(_) => {
                return Err(DriverBuildError::Key(
                    "endpoint".into(),
                    "must be web or news".into(),
                ));
            }
        };
        let secret = secret.ok_or_else(|| {
            DriverBuildError::Key("api_key_env".into(), "a credential is required".into())
        })?;
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("application/json"));
        let mut token = HeaderValue::from_str(secret.expose_secret()).map_err(|_| {
            DriverBuildError::Key(
                "api_key_env".into(),
                "the credential is not a valid header value".into(),
            )
        })?;
        token.set_sensitive(true);
        headers.insert("x-subscription-token", token);
        let client = egress::engine_client(&context.client_settings(headers))?;
        Ok(Brave {
            client,
            endpoint,
            base_override: context.base_override.clone(),
        })
    }

    fn url(&self) -> Result<url::Url, EngineError> {
        let base = match &self.base_override {
            Some(base) => base.clone(),
            None => url::Url::parse("https://api.search.brave.com/")
                .map_err(|_| EngineError::new(EngineErrorClass::RequestFailure))?,
        };
        let path = match self.endpoint {
            Endpoint::Web => "res/v1/web/search",
            Endpoint::News => "res/v1/news/search",
        };
        base.join(path)
            .map_err(|_| EngineError::new(EngineErrorClass::RequestFailure))
    }
}

#[derive(Debug, Deserialize)]
struct WebResponse {
    #[serde(default)]
    web: Option<ResultList>,
}

#[derive(Debug, Deserialize)]
struct NewsResponse {
    #[serde(default)]
    results: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct ResultList {
    #[serde(default)]
    results: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    title: String,
    url: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    page_age: Option<String>,
}

fn freshness(range: TimeRange) -> &'static str {
    match range {
        TimeRange::Day => "pd",
        TimeRange::Week => "pw",
        TimeRange::Month => "pm",
        TimeRange::Year => "py",
    }
}

fn safesearch(level: SafeSearch) -> &'static str {
    match level {
        SafeSearch::Off => "off",
        SafeSearch::Moderate => "moderate",
        SafeSearch::Strict => "strict",
    }
}

#[async_trait::async_trait]
impl Driver for Brave {
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
        let count = req.max_results.clamp(1, 20).to_string();
        let mut query: Vec<(&str, &str)> = vec![
            ("q", req.query.as_str()),
            ("count", count.as_str()),
            ("search_lang", req.language.primary()),
            ("country", req.region.as_str()),
            ("safesearch", safesearch(req.safe_search)),
        ];
        if let Some(range) = req.time_range {
            query.push(("freshness", freshness(range)));
        }
        let send = self.client.get(self.url()?).query(&query).send();
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
                let headers = response.headers();
                let wait = retry_after(headers, "retry-after")
                    .or_else(|| retry_after(headers, "x-ratelimit-reset"));
                return Err(EngineError {
                    class: EngineErrorClass::RateLimited,
                    retry_after: wait,
                });
            }
            401 | 403 => {
                debug_provider_text(KIND, status, &body);
                return Err(EngineError::new(EngineErrorClass::AuthFailure));
            }
            _ => {
                debug_provider_text(KIND, status, &body);
                return Err(EngineError::new(EngineErrorClass::RequestFailure));
            }
        }
        let items = match self.endpoint {
            Endpoint::Web => serde_json::from_slice::<WebResponse>(&body)
                .map(|r| r.web.map(|w| w.results).unwrap_or_default()),
            Endpoint::News => serde_json::from_slice::<NewsResponse>(&body).map(|r| r.results),
        }
        .map_err(|_| {
            debug_provider_text(KIND, status, &body);
            EngineError::new(EngineErrorClass::ParseFailure)
        })?;
        let results = items
            .into_iter()
            .map(|item| RawResult {
                title: item.title,
                url: item.url,
                snippet: item.description,
                date: item
                    .page_age
                    .as_deref()
                    .and_then(|t| Date::new(t.get(..10)?).ok()),
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
