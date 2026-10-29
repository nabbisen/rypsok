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
pub mod scrape;

use std::time::Duration;

use crate::CancellationToken;
use crate::boundary::Provenance;
use crate::config::SafeSearch;
use crate::model::{Date, Language, Reason, TimeRange};
use crate::privacy::Sensitive;

/// What one engine is asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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

impl EngineRequest {
    /// A request for `query` in `language` with the defaults a driver test
    /// wants: LIM-S3 results, region `US`, moderate safe search, no
    /// period. Set the public fields afterwards.
    pub fn new(query: &str, language: Language) -> Self {
        EngineRequest {
            query: Sensitive::new(query.to_owned()),
            max_results: crate::limits::defaults::RESULTS_PER_ENGINE,
            language,
            region: "US".to_owned(),
            safe_search: SafeSearch::Moderate,
            time_range: None,
        }
    }
}

/// One result as a provider returned it: text that has not passed the
/// content boundary (REQ-ENG-012). The text is raw (markup and entities
/// may be in it) unless the producer marked it [`decoded`](Self::decoded);
/// the mark travels with the result, through the result cache, and chooses
/// the cleaner (RFC 019 D3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RawResult {
    /// The title, as the provider gave it.
    pub title: String,
    /// The URL as the provider gave it, unwrapped from any redirect wrapper.
    pub url: String,
    /// The snippet, as the provider gave it.
    pub snippet: String,
    /// The date, when the provider gave one.
    pub date: Option<Date>,
    /// Where the text comes from; private, set by `new` and `decoded`.
    provenance: Provenance,
}

impl RawResult {
    /// A result without a date.
    ///
    /// ```
    /// use rypsok_core::engine::RawResult;
    /// use rypsok_core::model::Date;
    ///
    /// let result = RawResult::new("Title", "https://example.org/", "snippet")
    ///     .with_date(Date::new("2026-10-27").unwrap());
    /// assert_eq!(result.url, "https://example.org/");
    /// assert!(result.date.is_some());
    /// ```
    pub fn new(
        title: impl Into<String>,
        url: impl Into<String>,
        snippet: impl Into<String>,
    ) -> Self {
        RawResult {
            title: title.into(),
            url: url.into(),
            snippet: snippet.into(),
            date: None,
            provenance: Provenance::Raw,
        }
    }

    /// The same result marked as decoded: its title and snippet were taken
    /// from a parsed document, so no markup or entities remain in them and
    /// `<T>` is text. A result is raw unless its producer says otherwise,
    /// which errs on the safe side: raw text is cleaned as markup.
    #[must_use]
    pub fn decoded(mut self) -> Self {
        self.provenance = Provenance::Decoded;
        self
    }

    /// Where the text comes from, which chooses its cleaner.
    pub fn provenance(&self) -> Provenance {
        self.provenance
    }

    /// The same result with a date.
    #[must_use]
    pub fn with_date(mut self, date: Date) -> Self {
        self.date = Some(date);
        self
    }
}

/// What an engine answered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct EngineResponse {
    /// The raw results in the provider's order.
    pub results: Vec<RawResult>,
}

/// The classes of REQ-ENG-006.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
pub enum TimeRangeSupport {
    /// Every value.
    All,
    /// None: the engine is skipped when a period is asked for.
    None,
}

/// The User-Agent a driver sends (REQ-CMP-004).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
    /// Whether the driver reads a human-facing interface (RFC 015): such
    /// an engine runs only with `[scraping] enabled = true` (REQ-ENG-013).
    pub scraping: bool,
}

impl EngineResponse {
    /// An answer with these results.
    pub fn new(results: Vec<RawResult>) -> Self {
        EngineResponse { results }
    }
}

impl EngineProperties {
    /// Properties with the values a driver author must decide and
    /// conservative defaults for the rest: every language, no period,
    /// not cacheable, no attribution, the standard User-Agent, a 1 MiB
    /// response bound, no scraping. The `with_*` setters change them.
    ///
    /// ```
    /// use rypsok_core::engine::{EngineProperties, TimeRangeSupport};
    ///
    /// let properties = EngineProperties::new(250, 100, 1)
    ///     .with_time_ranges(TimeRangeSupport::All)
    ///     .with_cacheable(true);
    /// assert_eq!(properties.max_query_chars, 250);
    /// assert!(properties.cacheable);
    /// assert!(!properties.scraping);
    /// ```
    pub fn new(max_query_chars: u32, requests_per_minute: u32, max_in_flight: u8) -> Self {
        EngineProperties {
            max_query_chars,
            max_query_words: None,
            languages: LanguageSupport::All,
            time_ranges: TimeRangeSupport::None,
            requests_per_minute,
            max_in_flight,
            cacheable: false,
            attribution_required: false,
            user_agent: UserAgentFormat::Standard,
            max_response_bytes: 1_048_576,
            scraping: false,
        }
    }

    /// Most words the provider accepts.
    #[must_use]
    pub fn with_max_query_words(mut self, words: u32) -> Self {
        self.max_query_words = Some(words);
        self
    }

    /// The languages honored.
    #[must_use]
    pub fn with_languages(mut self, languages: LanguageSupport) -> Self {
        self.languages = languages;
        self
    }

    /// The periods honored.
    #[must_use]
    pub fn with_time_ranges(mut self, time_ranges: TimeRangeSupport) -> Self {
        self.time_ranges = time_ranges;
        self
    }

    /// Whether the admission record allows storage (RFC 014).
    #[must_use]
    pub fn with_cacheable(mut self, cacheable: bool) -> Self {
        self.cacheable = cacheable;
        self
    }

    /// Whether results must name the engine (REQ-CMP-003).
    #[must_use]
    pub fn with_attribution_required(mut self, required: bool) -> Self {
        self.attribution_required = required;
        self
    }

    /// The User-Agent format the provider requires.
    #[must_use]
    pub fn with_user_agent(mut self, format: UserAgentFormat) -> Self {
        self.user_agent = format;
        self
    }

    /// Most bytes read from one response.
    #[must_use]
    pub fn with_max_response_bytes(mut self, bytes: u64) -> Self {
        self.max_response_bytes = bytes;
        self
    }

    /// Whether the driver reads a human-facing interface (RFC 015).
    #[must_use]
    pub fn with_scraping(mut self, scraping: bool) -> Self {
        self.scraping = scraping;
        self
    }

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
