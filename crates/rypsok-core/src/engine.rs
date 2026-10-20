//! The engine abstraction, the registry and the admission properties
//! (external design §11; RFC 006).
//!
//! A [`Driver`] turns an [`EngineRequest`] into one provider request and
//! parses the answer into raw results or a classified [`EngineError`]
//! (REQ-ENG-001, REQ-ENG-006). Every driver declares its
//! [`EngineProperties`]: what its admission record allows and what it can
//! honor (REQ-ENG-010). The [`registry::Registry`] is built once from the
//! configuration and owns one engine per table, with its limiter and its
//! HTTP client.

pub mod drivers;
pub mod limiter;
pub mod registry;

use std::time::Duration;

use crate::CancellationToken;
use crate::config::SafeSearch;
use crate::model::{Date, Language, Reason, TimeRange};
use crate::privacy::Sensitive;

/// What one engine is asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineRequest {
    /// The query, validated and checked for secrets. A driver reads it
    /// with `expose()`; it cannot reach a log field (REQ-PRIV-004).
    pub query: Sensitive,
    /// Results wanted from this engine (LIM-S3 or fewer).
    pub max_results: u8,
    /// The language of the results.
    pub language: Language,
    /// The region, two uppercase letters.
    pub region: String,
    /// The safe-search level.
    pub safe_search: SafeSearch,
    /// The period, when one was asked for.
    pub time_range: Option<TimeRange>,
}

/// One result as a provider returned it: raw text that has not passed the
/// content boundary (REQ-ENG-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawResult {
    /// The title, raw.
    pub title: String,
    /// The URL as the provider gave it, unwrapped from any redirect wrapper.
    pub url: String,
    /// The snippet, raw.
    pub snippet: String,
    /// The date, when the provider gave one.
    pub date: Option<Date>,
}

/// What an engine answered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EngineResponse {
    /// The raw results in the provider's order.
    pub results: Vec<RawResult>,
}

/// The classes of REQ-ENG-006.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineErrorClass {
    /// The request timed out.
    Timeout,
    /// The provider rate-limited the request.
    RateLimited,
    /// A CAPTCHA or anti-bot response.
    Blocked,
    /// The response did not parse.
    ParseFailure,
    /// The credential was refused.
    AuthFailure,
    /// A network error or a 5xx status.
    RequestFailure,
}

impl EngineErrorClass {
    /// The notice reason that names this class.
    pub fn reason(self) -> Reason {
        match self {
            EngineErrorClass::Timeout => Reason::Timeout,
            EngineErrorClass::RateLimited => Reason::RateLimited,
            EngineErrorClass::Blocked => Reason::Blocked,
            EngineErrorClass::ParseFailure => Reason::ParseFailure,
            EngineErrorClass::AuthFailure => Reason::AuthFailure,
            EngineErrorClass::RequestFailure => Reason::RequestFailure,
        }
    }

    /// The serialized name, for logs.
    pub fn as_str(self) -> &'static str {
        match self {
            EngineErrorClass::Timeout => "timeout",
            EngineErrorClass::RateLimited => "rate_limited",
            EngineErrorClass::Blocked => "blocked",
            EngineErrorClass::ParseFailure => "parse_failure",
            EngineErrorClass::AuthFailure => "auth_failure",
            EngineErrorClass::RequestFailure => "request_failure",
        }
    }
}

/// A classified failure. It carries no provider text (REQ-SEC-016); a
/// driver logs such text at `debug` only, truncated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineError {
    /// The class.
    pub class: EngineErrorClass,
    /// When the provider said to retry, bounded by the caller (LIM-B4).
    pub retry_after: Option<Duration>,
}

impl EngineError {
    /// An error of a class without a retry hint.
    pub fn new(class: EngineErrorClass) -> Self {
        EngineError {
            class,
            retry_after: None,
        }
    }
}

/// Which languages a driver honors (REQ-SEARCH-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageSupport {
    /// Every language.
    All,
    /// Only these.
    Only(Vec<Language>),
    /// The engine ignores language; it is skipped when one other than the
    /// default is asked for.
    None,
}

/// Whether a driver honors `time_range` (REQ-SEARCH-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRangeSupport {
    /// Every value.
    All,
    /// None: the engine is skipped when a period is asked for.
    None,
}

/// The User-Agent a driver sends (REQ-CMP-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserAgentFormat {
    /// The standard rypsok User-Agent.
    Standard,
    /// A provider-mandated format, with `{standard}` replaced by the
    /// standard User-Agent.
    Custom(&'static str),
}

/// What the admission record allows and what the driver can honor
/// (REQ-ENG-010, REQ-CMP-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineProperties {
    /// Longest query the provider accepts, in characters.
    pub max_query_chars: u32,
    /// Most words the provider accepts, when it counts words.
    pub max_query_words: Option<u32>,
    /// Languages honored.
    pub languages: LanguageSupport,
    /// Periods honored.
    pub time_ranges: TimeRangeSupport,
    /// Requests per minute the admission record allows (LIM-C9).
    pub requests_per_minute: u32,
    /// Requests in flight at most (at or below LIM-C2).
    pub max_in_flight: u8,
    /// Whether results may be cached (milestone M5).
    pub cacheable: bool,
    /// Whether results must carry the engine identifier (REQ-CMP-003).
    pub attribution_required: bool,
    /// The User-Agent format.
    pub user_agent: UserAgentFormat,
    /// Most bytes read from one response.
    pub max_response_bytes: u64,
}

impl EngineProperties {
    /// Why the engine cannot serve this request, if it cannot
    /// (REQ-ENG-007): the request is checked before any limiter.
    pub fn skip_reason(
        &self,
        query: &str,
        language: &Language,
        default_language: &Language,
        time_range: Option<TimeRange>,
    ) -> Option<Reason> {
        if query.chars().count() > self.max_query_chars as usize {
            return Some(Reason::QueryTooLong);
        }
        if let Some(max_words) = self.max_query_words
            && query.split_whitespace().count() > max_words as usize
        {
            return Some(Reason::QueryTooLong);
        }
        let language_ok = match &self.languages {
            LanguageSupport::All => true,
            LanguageSupport::Only(list) => list.iter().any(|l| l.primary() == language.primary()),
            LanguageSupport::None => language == default_language,
        };
        if !language_ok {
            return Some(Reason::Language);
        }
        if time_range.is_some() && self.time_ranges == TimeRangeSupport::None {
            return Some(Reason::TimeRange);
        }
        None
    }
}

/// A driver: one per engine type, built once with its HTTP client.
#[async_trait::async_trait]
pub trait Driver: Send + Sync + std::fmt::Debug {
    /// The driver type, as in the configuration's `type` key.
    fn kind(&self) -> &'static str;
    /// The admission properties.
    fn properties(&self) -> &EngineProperties;
    /// Performs one provider request. Cancellation aborts the request.
    async fn search(
        &self,
        req: &EngineRequest,
        cancel: CancellationToken,
    ) -> Result<EngineResponse, EngineError>;
}

#[cfg(test)]
mod tests;
