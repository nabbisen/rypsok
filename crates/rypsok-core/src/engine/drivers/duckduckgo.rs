//! The DuckDuckGo HTML driver (RFC 015): a scraping driver for the
//! human-facing endpoint `html.duckduckgo.com/html/`. It runs only with
//! `[scraping] enabled = true` (REQ-ENG-013), sends the standard
//! User-Agent and nothing that resembles a browser (REQ-ENG-014), reads a
//! bounded response, parses it under the shared parsing permits with the
//! bounded DOM, and reports a challenge as `blocked` (REQ-ENG-015,
//! REQ-ENG-009). The admission record is
//! `docs/src/maintainers/engines/duckduckgo.md`; the markers and
//! parameters below are its facts, side by side.

use std::sync::Arc;

use reqwest::header::{HeaderMap, HeaderValue};
use tokio::sync::Semaphore;

use super::{DriverBuildError, DriverContext, debug_provider_text, is_loopback, retry_after};
use crate::CancellationToken;
use crate::config::SafeSearch;
use crate::egress::{self, BodyError};
use crate::engine::scrape::{self, Marker, Page, Selector, Shape};
use crate::engine::{
    Driver, EngineError, EngineErrorClass, EngineProperties, EngineRequest, EngineResponse,
    LanguageSupport, TimeRangeSupport, UserAgentFormat,
};
use crate::html::{charset, dom};
use crate::model::TimeRange;

/// The driver type, as in the configuration's `type` key.
pub const KIND: &str = "duckduckgo";

/// The provider's base; `base_override` replaces it in tests.
const BASE: &str = "https://html.duckduckgo.com/";

/// The admission properties (the record's rows "Rate limit",
/// "Concurrency", "Storage", "Attribution", "Response bound").
pub const PROPERTIES: EngineProperties = EngineProperties {
    max_query_chars: 500,
    max_query_words: None,
    languages: LanguageSupport::All,
    time_ranges: TimeRangeSupport::All,
    requests_per_minute: 10,
    max_in_flight: 1,
    cacheable: false,
    attribution_required: false,
    user_agent: UserAgentFormat::Standard,
    max_response_bytes: 262_144,
    scraping: true,
};

/// The page's shape (the record's rows "Result", "No results",
/// "Challenge").
pub const SHAPE: Shape = Shape {
    result: Selector::class("div", "result"),
    link: Selector::class("a", "result__a"),
    title: Selector::class("a", "result__a"),
    snippet: Selector::class("a", "result__snippet"),
    no_results: Marker::Element(Selector::class("div", "no-results")),
    challenge: &[
        Marker::Text("bots use DuckDuckGo too"),
        Marker::Element(Selector {
            tag: None,
            class: Some("anomaly-modal__title"),
            attr: None,
        }),
        Marker::Element(Selector {
            tag: Some("form"),
            class: None,
            attr: Some(("action", "/html/anomaly")),
        }),
    ],
};

/// The driver: one client under the engine profile, the provider's base
/// (or a test override), and the shared parsing permits.
#[derive(Debug)]
pub struct DuckDuckGo {
    client: reqwest::Client,
    base: url::Url,
    parsing: Arc<Semaphore>,
    html_max_nodes: u32,
}

impl DuckDuckGo {
    /// Builds the driver; the table declares no keys.
    pub fn new(context: &DriverContext) -> Result<Self, DriverBuildError> {
        let base = match &context.base_override {
            Some(base) => base.clone(),
            None => url::Url::parse(BASE).expect("a constant URL"),
        };
        // The only header beside the standard User-Agent: what the page
        // is. Nothing a browser would add (REQ-ENG-014).
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/html"));
        let mut settings = context.client_settings(headers);
        if is_loopback(&base) {
            settings.https_only = false;
        }
        let client = egress::engine_client(&settings)?;
        Ok(DuckDuckGo {
            client,
            base,
            parsing: Arc::clone(&context.parsing),
            html_max_nodes: context.html_max_nodes,
        })
    }

    /// The target of a result link: the provider's redirect wrapper
    /// (`//duckduckgo.com/l/?uddg=<target>&rut=…`) unwrapped, or an
    /// absolute `http(s)` link as it is (REQ-ENG-001).
    pub fn unwrap_link(href: &str) -> Option<String> {
        let absolute = if let Some(rest) = href.strip_prefix("//") {
            format!("https://{rest}")
        } else {
            href.to_owned()
        };
        let url = url::Url::parse(&absolute).ok()?;
        if !matches!(url.scheme(), "http" | "https") {
            return None;
        }
        if url.path().starts_with("/l/")
            && url
                .host_str()
                .is_some_and(|h| h == "duckduckgo.com" || h.ends_with(".duckduckgo.com"))
        {
            let target = url
                .query_pairs()
                .find(|(k, _)| k == "uddg")
                .map(|(_, v)| v.into_owned())?;
            let parsed = url::Url::parse(&target).ok()?;
            return matches!(parsed.scheme(), "http" | "https").then_some(target);
        }
        Some(absolute)
    }
}

/// The provider's region-language code (the record's row "Region").
fn region_code(region: &str, language: &str) -> String {
    format!(
        "{}-{}",
        region.to_ascii_lowercase(),
        language.to_ascii_lowercase()
    )
}

fn safe_search_code(level: SafeSearch) -> &'static str {
    match level {
        SafeSearch::Strict => "1",
        SafeSearch::Moderate => "-1",
        SafeSearch::Off => "-2",
    }
}

fn time_range_code(range: TimeRange) -> &'static str {
    match range {
        TimeRange::Day => "d",
        TimeRange::Week => "w",
        TimeRange::Month => "m",
        TimeRange::Year => "y",
    }
}

#[async_trait::async_trait]
impl Driver for DuckDuckGo {
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
        let endpoint = self
            .base
            .join("html/")
            .map_err(|_| EngineError::new(EngineErrorClass::RequestFailure))?;
        let mut params: Vec<(&str, String)> = vec![
            ("q", req.query.expose().to_owned()),
            ("kl", region_code(&req.region, req.language.primary())),
            ("kp", safe_search_code(req.safe_search).to_owned()),
        ];
        if let Some(range) = req.time_range {
            params.push(("df", time_range_code(range).to_owned()));
        }
        let send = self.client.get(endpoint).query(&params).send();
        let mut response = tokio::select! {
            _ = cancel.cancelled() => return Err(EngineError::new(EngineErrorClass::RequestFailure)),
            sent = send => sent.map_err(classify_transport)?,
        };
        let status = response.status().as_u16();
        let declared = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .and_then(charset::charset_parameter)
            .map(str::to_owned);
        let body = tokio::select! {
            _ = cancel.cancelled() => return Err(EngineError::new(EngineErrorClass::RequestFailure)),
            read = egress::read_body(&mut response, PROPERTIES.max_response_bytes) => read.map_err(|e| match e {
                BodyError::TooLarge => EngineError::new(EngineErrorClass::ParseFailure),
                BodyError::Transport => EngineError::new(EngineErrorClass::RequestFailure),
            })?,
        };
        match status {
            200 => {}
            429 | 503 => {
                return Err(EngineError {
                    class: EngineErrorClass::RateLimited,
                    retry_after: retry_after(response.headers(), "retry-after"),
                });
            }
            403 => {
                debug_provider_text(KIND, status, &body);
                return Err(EngineError::new(EngineErrorClass::Blocked));
            }
            // A redirect is never followed (REQ-ENG-015); anything else is
            // the provider's problem.
            _ => {
                debug_provider_text(KIND, status, &body);
                return Err(EngineError::new(EngineErrorClass::RequestFailure));
            }
        }

        // Parsing off the runtime under LIM-C5 (REQ-ENG-015).
        let permit = tokio::select! {
            _ = cancel.cancelled() => return Err(EngineError::new(EngineErrorClass::RequestFailure)),
            acquired = Arc::clone(&self.parsing).acquire_owned() => acquired.map_err(|_| EngineError::new(EngineErrorClass::RequestFailure))?,
        };
        let max_nodes = self.html_max_nodes;
        let max_results = usize::from(req.max_results);
        let page = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let decoded = charset::decode(&body, declared.as_deref());
            let tree = dom::parse(&decoded.text, max_nodes);
            scrape::read(&tree, &SHAPE, max_results, &DuckDuckGo::unwrap_link)
        })
        .await
        .map_err(|_| EngineError::new(EngineErrorClass::ParseFailure))?;
        match page {
            Page::Results(results) => Ok(EngineResponse { results }),
            Page::NoResults => Ok(EngineResponse::default()),
            Page::Challenge => {
                tracing::debug!(
                    event = "provider_challenge",
                    driver = KIND,
                    "the provider answered a challenge page"
                );
                Err(EngineError::new(EngineErrorClass::Blocked))
            }
            Page::Drifted => Err(EngineError::new(EngineErrorClass::ParseFailure)),
        }
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
