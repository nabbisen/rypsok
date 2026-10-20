//! Request, result, outcome, notice and error types of the external
//! contract (external design §3 and §4; RFC 003).
//!
//! Every structure that carries external text keeps it in a field distinct
//! from every field rypsok asserts (REQ-CTX-002). Every vocabulary is a
//! closed enumeration whose serialized names are the strings of the
//! external design (REQ-SEC-016). Requests refuse unknown fields
//! (REQ-MCP-004).

use std::fmt;

use serde::{Deserialize, Serialize};

pub mod error;
pub mod ids;
pub mod outcome;

pub use error::{DetailReason, ErrorDetail, ErrorEnvelope, ErrorKind, SecretKind, ToolError};
pub use ids::{Date, EngineId, Language};
pub use outcome::{Decision, decide};

/// The category of a search: which engines are asked (REQ-SEARCH-002).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// General web search; the default.
    General,
    /// News.
    News,
    /// Scientific and reference sources.
    Science,
    /// Source code and developer documentation.
    Code,
}

impl Category {
    /// Every category, in vocabulary order.
    pub const ALL: [Category; 4] = [
        Category::General,
        Category::News,
        Category::Science,
        Category::Code,
    ];

    /// The serialized name.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::General => "general",
            Category::News => "news",
            Category::Science => "science",
            Category::Code => "code",
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The period a search is restricted to (REQ-SEARCH-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    /// The last day.
    Day,
    /// The last week.
    Week,
    /// The last month.
    Month,
    /// The last year.
    Year,
}

impl TimeRange {
    /// Every value, in vocabulary order.
    pub const ALL: [TimeRange; 4] = [
        TimeRange::Day,
        TimeRange::Week,
        TimeRange::Month,
        TimeRange::Year,
    ];

    /// The serialized name.
    pub fn as_str(self) -> &'static str {
        match self {
            TimeRange::Day => "day",
            TimeRange::Week => "week",
            TimeRange::Month => "month",
            TimeRange::Year => "year",
        }
    }
}

/// A `web_search` request as a caller sends it (external design §3.1).
///
/// Optional parameters that are absent take the operator's defaults inside
/// the core, so that every host gets the same defaults (REQ-CFG-003).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    /// What to search for (REQ-SEARCH-001).
    pub query: String,
    /// Which engines to ask; `general` when absent (REQ-SEARCH-002).
    #[serde(default)]
    pub category: Option<Category>,
    /// How many results at most; LIM-S2 when absent (REQ-SEARCH-003).
    #[serde(default)]
    pub max_results: Option<u8>,
    /// Language of the results; the operator's default when absent (REQ-SEARCH-004).
    #[serde(default)]
    pub language: Option<Language>,
    /// Only results from this period (REQ-SEARCH-005).
    #[serde(default)]
    pub time_range: Option<TimeRange>,
}

impl SearchRequest {
    /// A request with only the query set; every other parameter takes its default.
    pub fn new(query: impl Into<String>) -> Self {
        SearchRequest {
            query: query.into(),
            category: None,
            max_results: None,
            language: None,
            time_range: None,
        }
    }
}

/// A `web_fetch` request (external design §3.2; milestone M3).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FetchRequest {
    /// The page to read.
    pub url: String,
    /// Characters to return at most; LIM-F1 when absent.
    #[serde(default)]
    pub max_chars: Option<u32>,
    /// Continue from this character; 0 when absent.
    #[serde(default)]
    pub start_char: Option<u32>,
}

/// The outcome of a successful call (REQ-ERR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Every engine answered and results remain.
    Ok,
    /// Results remain, but at least one engine failed.
    Partial,
    /// At least one engine answered and no result remains.
    Empty,
}

/// A notice code (external design §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NoticeCode {
    /// An engine failed; `reason` is its error class.
    EngineUnavailable,
    /// An engine was not asked; `reason` says why.
    EngineSkipped,
    /// Snippets were cut to LIM-S9; `count` says how many.
    SnippetTruncated,
    /// Titles were cut to LIM-S10; `count` says how many.
    TitleTruncated,
    /// Invisible or control characters were removed; `count` says how many.
    CharactersRemoved,
    /// Results carry instruction-like text; `count` says how many.
    InstructionLikeContent,
    /// Results were dropped; `reason` says why and `count` how many.
    ResultsDropped,
    /// Credentials in the query were masked (mask mode only); `count` says how many.
    CredentialMasked,
    /// A fetched document was malformed and extracted in part (`web_fetch` only).
    ExtractionPartial,
}

/// A reason carried by a notice: engine error classes and skip or drop
/// reasons share one closed vocabulary (external design §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Engine error class: the request timed out (REQ-ENG-006).
    Timeout,
    /// Engine error class, or skip reason: the provider's or the engine's rate limit.
    RateLimited,
    /// Engine error class: a CAPTCHA or anti-bot response.
    Blocked,
    /// Engine error class: the response did not parse.
    ParseFailure,
    /// Engine error class: the credential was refused.
    AuthFailure,
    /// Engine error class: a network error or a 5xx status.
    RequestFailure,
    /// Skip reason: the engine cannot honor `time_range`.
    TimeRange,
    /// Skip reason: the engine cannot honor `language`.
    Language,
    /// Skip reason: the query exceeds the engine's limit.
    QueryTooLong,
    /// Skip reason: the engine is in cooldown.
    Cooldown,
    /// Drop reason: the result carried instruction-like text.
    InstructionLikeContent,
    /// Drop reason: the URL exceeds LIM-F9.
    UrlTooLong,
    /// Drop reason: the URL has no `http` or `https` scheme or carries credentials.
    UrlInvalid,
}

/// A notice: something rypsok did or could not do, from a fixed vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Notice {
    /// What happened.
    pub code: NoticeCode,
    /// The engine concerned, when one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<EngineId>,
    /// Why, when the code carries a reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// How many results or characters were affected, when the code counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

impl Notice {
    /// A notice with only a code.
    pub fn new(code: NoticeCode) -> Self {
        Notice {
            code,
            engine: None,
            reason: None,
            count: None,
        }
    }

    /// A notice about an engine with a reason.
    pub fn engine(code: NoticeCode, engine: EngineId, reason: Reason) -> Self {
        Notice {
            code,
            engine: Some(engine),
            reason: Some(reason),
            count: None,
        }
    }

    /// A notice with a count.
    pub fn counted(code: NoticeCode, count: u32) -> Self {
        Notice {
            code,
            engine: None,
            reason: None,
            count: Some(count),
        }
    }

    /// A notice with a reason and a count.
    pub fn counted_with_reason(code: NoticeCode, reason: Reason, count: u32) -> Self {
        Notice {
            code,
            engine: None,
            reason: Some(reason),
            count: Some(count),
        }
    }
}

/// One result of `web_search`. The title, URL, snippet and date are external
/// data, cleaned and bounded by the content boundary (REQ-CTX-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchResult {
    /// The page title, cleaned and bounded by LIM-S10.
    pub title: String,
    /// The engine's URL with tracking parameters removed.
    pub url: String,
    /// The snippet, cleaned and bounded by LIM-S9.
    pub snippet: String,
    /// The date an engine supplied, when one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<Date>,
    /// The engines that returned this result, when attribution is on or required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engines: Option<Vec<EngineId>>,
    /// Proof that the external fields passed the content boundary. The
    /// field is crate-private, so a result is built only by
    /// `boundary::make_result` (RFC 005).
    #[serde(skip)]
    #[doc(hidden)]
    pub(crate) cleaned: Cleaned,
}

/// Marker of text that passed the content boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, schemars::JsonSchema)]
#[doc(hidden)]
pub struct Cleaned;

/// The response of `web_search` (external design §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchResponse {
    /// The outcome.
    pub outcome: Outcome,
    /// What rypsok did or could not do.
    pub notices: Vec<Notice>,
    /// The results, at most `max_results`.
    pub results: Vec<SearchResult>,
}

/// The response of `web_fetch` (external design §3.2; milestone M3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FetchResponse {
    /// The outcome.
    pub outcome: Outcome,
    /// What rypsok did or could not do.
    pub notices: Vec<Notice>,
    /// The URL that was read, after same-host redirects.
    pub url: String,
    /// The page title, cleaned and bounded by LIM-S10.
    pub title: String,
    /// The content, as Markdown, bounded by `max_chars`.
    pub content: String,
    /// Whether the content was cut.
    pub truncated: bool,
    /// Where to continue when it was cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_start_char: Option<u32>,
}

/// The state of an engine's circuit (REQ-STATUS-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EngineState {
    /// Disabled by configuration or inactive for lack of a credential.
    Disabled,
    /// In use.
    Closed,
    /// Skipped during a cooldown.
    Open,
    /// One probe request in flight.
    HalfOpen,
}

/// The state of one engine as `engine_status` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EngineStatus {
    /// The configured identifier.
    pub id: EngineId,
    /// The circuit state.
    pub state: EngineState,
    /// Median latency of the last LIM-S11 completed requests, or null.
    pub latency_ms: Option<u32>,
}

/// The response of `engine_status` (external design §3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StatusResponse {
    /// Always `ok`.
    pub outcome: Outcome,
    /// Always empty.
    pub notices: Vec<Notice>,
    /// One entry per configured engine, in configuration order.
    pub engines: Vec<EngineStatus>,
}

#[cfg(test)]
mod tests;
