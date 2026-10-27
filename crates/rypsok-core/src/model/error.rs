//! The error object of the contract (external design §4; REQ-ERR-002,
//! REQ-ERR-004). Messages are built from templates with parameter names and
//! numbers only, never with a value from a request, a provider or a page,
//! and are bounded by LIM-I4.

use serde::{Deserialize, Serialize};

use crate::limits::defaults::MESSAGE_MAX_CHARS;

/// The kind of an error (REQ-ERR-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// A parameter fails validation.
    InvalidRequest,
    /// Policy refuses the call.
    Denied,
    /// Nothing could be returned now; the caller may retry.
    Unavailable,
    /// A fetch was redirected to another host.
    Redirected,
    /// The content cannot be extracted.
    UnsupportedContent,
    /// The response exceeded a byte limit.
    TooLarge,
}

impl ErrorKind {
    /// The snake-case name, as serialized.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::InvalidRequest => "invalid_request",
            ErrorKind::Denied => "denied",
            ErrorKind::Unavailable => "unavailable",
            ErrorKind::Redirected => "redirected",
            ErrorKind::UnsupportedContent => "unsupported_content",
            ErrorKind::TooLarge => "too_large",
        }
    }
}

/// The reason inside `detail` for `denied`, `unavailable` and
/// `unsupported_content` errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DetailReason {
    /// Denied: the egress policy.
    Egress,
    /// Denied: robots.txt.
    RobotsTxt,
    /// Denied: the URL scheme.
    Scheme,
    /// Denied: strict mode.
    StrictMode,
    /// Denied: the URL carries credentials.
    CredentialInUrl,
    /// Denied: the query looks like it carries a secret.
    SecretInQuery,
    /// Denied: the URL looks like it carries a secret.
    SecretInUrl,
    /// Unavailable: no engine is enabled for the category.
    NoEngine,
    /// Unavailable: every engine failed.
    AllFailed,
    /// Unavailable: the tool's rate limit.
    RateLimited,
    /// Unavailable: a network failure.
    Network,
    /// Unavailable: the deadline passed with nothing to return.
    Deadline,
    /// Unavailable: the target answered 4xx or 5xx.
    HttpStatus,
    /// Unavailable: the concurrency limit.
    Busy,
    /// Unsupported content: the media type.
    MediaType,
    /// Unsupported content: nothing could be extracted.
    ExtractionFailed,
}

impl DetailReason {
    /// The snake-case name, as serialized.
    pub fn as_str(self) -> &'static str {
        match self {
            DetailReason::Egress => "egress",
            DetailReason::RobotsTxt => "robots_txt",
            DetailReason::Scheme => "scheme",
            DetailReason::StrictMode => "strict_mode",
            DetailReason::CredentialInUrl => "credential_in_url",
            DetailReason::SecretInQuery => "secret_in_query",
            DetailReason::SecretInUrl => "secret_in_url",
            DetailReason::NoEngine => "no_engine",
            DetailReason::AllFailed => "all_failed",
            DetailReason::RateLimited => "rate_limited",
            DetailReason::Network => "network",
            DetailReason::Deadline => "deadline",
            DetailReason::HttpStatus => "http_status",
            DetailReason::Busy => "busy",
            DetailReason::MediaType => "media_type",
            DetailReason::ExtractionFailed => "extraction_failed",
        }
    }
}

/// The kind of credential the secret check found (RFC 009).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SecretKind {
    /// An AWS access key identifier.
    AwsAccessKey,
    /// A GitHub token.
    GithubToken,
    /// A Slack token.
    SlackToken,
    /// A Google API key.
    GoogleApiKey,
    /// A Stripe key.
    StripeKey,
    /// An OpenAI key.
    OpenaiKey,
    /// A PEM private key.
    PrivateKey,
    /// A JSON Web Token.
    Jwt,
    /// A bearer authorization header.
    BearerHeader,
    /// Credentials in a URL's userinfo.
    BasicAuthUrl,
}

impl SecretKind {
    /// A short English name for messages.
    pub fn describe(self) -> &'static str {
        match self {
            SecretKind::AwsAccessKey => "an AWS access key",
            SecretKind::GithubToken => "a github token",
            SecretKind::SlackToken => "a slack token",
            SecretKind::GoogleApiKey => "a google api key",
            SecretKind::StripeKey => "a stripe key",
            SecretKind::OpenaiKey => "an openai key",
            SecretKind::PrivateKey => "a private key",
            SecretKind::Jwt => "a json web token",
            SecretKind::BearerHeader => "a bearer token",
            SecretKind::BasicAuthUrl => "credentials in a url",
        }
    }
}

/// The `detail` object: fixed keys, each present only when the kind
/// defines it (external design §4).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[non_exhaustive]
pub struct ErrorDetail {
    /// The parameter that failed validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter: Option<String>,
    /// The smallest accepted value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    /// The largest accepted value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    /// The accepted values of an enumerated parameter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed: Option<Vec<String>>,
    /// Why the call was refused or failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<DetailReason>,
    /// The kind of secret found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SecretKind>,
    /// The character offset of the first finding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    /// When the caller may retry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    /// The HTTP status a target answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Where a fetch was redirected to (external data, bounded by LIM-F9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// The media type a target served (external data, bounded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    /// The byte limit that was exceeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_bytes: Option<u64>,
}

/// A tool execution error (REQ-ERR-002, REQ-ERR-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[non_exhaustive]
pub struct ToolError {
    /// The kind.
    pub kind: ErrorKind,
    /// What the caller can change, in at most LIM-I4 characters.
    pub message: String,
    /// Fixed keys that depend on the kind. Boxed so that the error stays
    /// small on the success path.
    pub detail: Box<ErrorDetail>,
}

/// How a call ends when it does not end with a response.
///
/// A host returns `Tool` to its caller as the error object (external
/// design §4) and returns nothing for `Cancelled`: the caller asked for
/// the call to stop, nothing of the call keeps running, and there is no
/// outcome to report (REQ-MCP-007). A cancelled call is never dressed up
/// as a deadline or any other error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError {
    /// The error object to return.
    Tool(ToolError),
    /// The caller cancelled the call; there is nothing to return.
    Cancelled,
}

impl From<ToolError> for CallError {
    fn from(error: ToolError) -> Self {
        CallError::Tool(error)
    }
}

impl CallError {
    /// The error object, when the call ended with one.
    pub fn tool(self) -> Option<ToolError> {
        match self {
            CallError::Tool(error) => Some(error),
            CallError::Cancelled => None,
        }
    }
}

/// The error as it is rendered: `{"error": …}` (external design §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ErrorEnvelope {
    /// The error.
    pub error: ToolError,
}

impl ToolError {
    fn new(kind: ErrorKind, message: String, detail: ErrorDetail) -> Self {
        debug_assert!(
            message.chars().count() <= MESSAGE_MAX_CHARS as usize,
            "message exceeds LIM-I4"
        );
        ToolError {
            kind,
            message,
            detail: Box::new(detail),
        }
    }

    /// The rendering of external design §4.
    pub fn envelope(self) -> ErrorEnvelope {
        ErrorEnvelope { error: self }
    }

    /// `invalid_request`: an integer parameter outside its range.
    pub fn out_of_range(parameter: &'static str, min: u32, max: u32) -> Self {
        Self::new(
            ErrorKind::InvalidRequest,
            format!("{parameter} must be between {min} and {max}"),
            ErrorDetail {
                parameter: Some(parameter.to_owned()),
                min: Some(min),
                max: Some(max),
                ..Default::default()
            },
        )
    }

    /// `invalid_request`: an enumerated parameter outside its vocabulary.
    pub fn not_allowed(parameter: &'static str, allowed: Vec<String>) -> Self {
        let list = allowed.join(", ");
        Self::new(
            ErrorKind::InvalidRequest,
            format!("{parameter} must be one of: {list}"),
            ErrorDetail {
                parameter: Some(parameter.to_owned()),
                allowed: Some(allowed),
                ..Default::default()
            },
        )
    }

    /// `invalid_request`: a required parameter is absent.
    pub fn missing(parameter: &'static str) -> Self {
        Self::new(
            ErrorKind::InvalidRequest,
            format!("{parameter} is required"),
            ErrorDetail {
                parameter: Some(parameter.to_owned()),
                ..Default::default()
            },
        )
    }

    /// `invalid_request`: a parameter has the wrong JSON type.
    pub fn wrong_type(parameter: &'static str, expected: &'static str) -> Self {
        Self::new(
            ErrorKind::InvalidRequest,
            format!("{parameter} must be {expected}"),
            ErrorDetail {
                parameter: Some(parameter.to_owned()),
                ..Default::default()
            },
        )
    }

    /// `invalid_request`: an argument that is not a parameter of the tool.
    /// The unknown name is caller text and is not repeated; the accepted
    /// names are listed instead.
    pub fn unknown_parameter(allowed: &[&'static str]) -> Self {
        let allowed: Vec<String> = allowed.iter().map(|s| (*s).to_owned()).collect();
        let list = allowed.join(", ");
        Self::new(
            ErrorKind::InvalidRequest,
            format!("unknown parameter; the parameters are: {list}"),
            ErrorDetail {
                allowed: Some(allowed),
                ..Default::default()
            },
        )
    }

    /// `invalid_request`: the query is empty after trimming or too long.
    pub fn query_length(max: u32) -> Self {
        Self::new(
            ErrorKind::InvalidRequest,
            format!("query must be between 1 and {max} characters after trimming"),
            ErrorDetail {
                parameter: Some("query".to_owned()),
                min: Some(1),
                max: Some(max),
                ..Default::default()
            },
        )
    }

    /// `denied`: the secret check found a credential in the query.
    pub fn secret_in_query(kind: SecretKind, position: u32) -> Self {
        Self::new(
            ErrorKind::Denied,
            format!(
                "query contains what looks like {} at character {position}; remove it and retry",
                kind.describe()
            ),
            ErrorDetail {
                reason: Some(DetailReason::SecretInQuery),
                kind: Some(kind),
                position: Some(position),
                ..Default::default()
            },
        )
    }

    /// `denied`: the secret check found a credential in the URL.
    pub fn secret_in_url(kind: SecretKind, position: u32) -> Self {
        Self::new(
            ErrorKind::Denied,
            format!(
                "url contains what looks like {} at character {position}; remove it and retry",
                kind.describe()
            ),
            ErrorDetail {
                reason: Some(DetailReason::SecretInUrl),
                kind: Some(kind),
                position: Some(position),
                ..Default::default()
            },
        )
    }

    /// `unavailable`: no engine is enabled for the category.
    pub fn no_engine(category: super::Category) -> Self {
        Self::new(
            ErrorKind::Unavailable,
            format!("no engine is enabled for category \"{category}\""),
            ErrorDetail {
                reason: Some(DetailReason::NoEngine),
                ..Default::default()
            },
        )
    }

    /// `unavailable`: every queried engine failed.
    pub fn all_failed(retry_after_ms: Option<u64>) -> Self {
        Self::new(
            ErrorKind::Unavailable,
            "every engine failed; retry later".to_owned(),
            ErrorDetail {
                reason: Some(DetailReason::AllFailed),
                retry_after_ms,
                ..Default::default()
            },
        )
    }

    /// `unavailable`: the deadline passed with nothing to return.
    pub fn deadline() -> Self {
        Self::new(
            ErrorKind::Unavailable,
            "no engine answered before the deadline; retry later".to_owned(),
            ErrorDetail {
                reason: Some(DetailReason::Deadline),
                ..Default::default()
            },
        )
    }

    /// `unavailable`: the tool's own rate limit was reached.
    pub fn rate_limited(retry_after_ms: u64) -> Self {
        Self::new(
            ErrorKind::Unavailable,
            format!("rate limit of this tool reached; retry after {retry_after_ms} ms"),
            ErrorDetail {
                reason: Some(DetailReason::RateLimited),
                retry_after_ms: Some(retry_after_ms),
                ..Default::default()
            },
        )
    }

    /// `denied` for the fetch policies (external design §4): a fixed
    /// message per reason; never a URL, a host or an address.
    pub fn denied(reason: DetailReason) -> Self {
        let message = match reason {
            DetailReason::Egress => "the destination is not allowed by the egress policy",
            DetailReason::RobotsTxt => "the site's robots.txt disallows this page for rypsok",
            DetailReason::Scheme => {
                "the http URL was retried over https and that request failed; plain http is not allowed here"
            }
            DetailReason::StrictMode => {
                "strict mode: only URLs that web_search returned in this process, or that the operator listed, can be fetched"
            }
            DetailReason::CredentialInUrl => "the URL carries credentials; remove them",
            _ => "the call was refused by policy",
        };
        Self::new(
            ErrorKind::Denied,
            message.to_owned(),
            ErrorDetail {
                reason: Some(reason),
                ..Default::default()
            },
        )
    }

    /// `unavailable` / `network`: the page could not be retrieved.
    pub fn network() -> Self {
        Self::new(
            ErrorKind::Unavailable,
            "the page could not be retrieved; retry later".to_owned(),
            ErrorDetail {
                reason: Some(DetailReason::Network),
                ..Default::default()
            },
        )
    }

    /// `unavailable` / `http_status`: the target answered 4xx or 5xx; the
    /// body is never returned (REQ-FETCH-011).
    pub fn http_status(status: u16) -> Self {
        Self::new(
            ErrorKind::Unavailable,
            format!("the site answered HTTP {status}; the page was not read"),
            ErrorDetail {
                reason: Some(DetailReason::HttpStatus),
                status: Some(status),
                ..Default::default()
            },
        )
    }

    /// `redirected`: the page moved to another host that passes the
    /// policy; `location` is external data bounded by the caller
    /// (REQ-FETCH-006).
    pub fn redirected(location: String) -> Self {
        Self::new(
            ErrorKind::Redirected,
            "the page moved to another host; see detail.location".to_owned(),
            ErrorDetail {
                location: Some(location),
                ..Default::default()
            },
        )
    }

    /// `unsupported_content` / `media_type`: the type cannot be extracted;
    /// `media_type` is external data bounded by the caller (REQ-FETCH-008).
    pub fn unsupported_media_type(media_type: String) -> Self {
        Self::new(
            ErrorKind::UnsupportedContent,
            "the content type cannot be extracted; see detail.media_type".to_owned(),
            ErrorDetail {
                reason: Some(DetailReason::MediaType),
                media_type: Some(media_type),
                ..Default::default()
            },
        )
    }

    /// `unsupported_content` / `extraction_failed`: nothing could be
    /// extracted (REQ-PAGE-006).
    pub fn extraction_failed() -> Self {
        Self::new(
            ErrorKind::UnsupportedContent,
            "nothing could be extracted from the page".to_owned(),
            ErrorDetail {
                reason: Some(DetailReason::ExtractionFailed),
                ..Default::default()
            },
        )
    }

    /// `too_large`: the response exceeded the byte limit (REQ-PAGE-003).
    pub fn too_large(limit_bytes: u64) -> Self {
        Self::new(
            ErrorKind::TooLarge,
            format!("the response exceeds the limit of {limit_bytes} bytes"),
            ErrorDetail {
                limit_bytes: Some(limit_bytes),
                ..Default::default()
            },
        )
    }

    /// `invalid_request`: `start_char` is at or beyond the end of the
    /// content; the message states the length (REQ-FETCH-005).
    pub fn start_beyond_end(total_chars: u32) -> Self {
        Self::new(
            ErrorKind::InvalidRequest,
            format!("start_char is beyond the end: the content has {total_chars} characters"),
            ErrorDetail {
                parameter: Some("start_char".to_owned()),
                min: Some(0),
                max: Some(total_chars.saturating_sub(1)),
                ..Default::default()
            },
        )
    }

    /// `unavailable`: the concurrency limit was reached.
    pub fn busy() -> Self {
        Self::new(
            ErrorKind::Unavailable,
            "too many calls in flight; retry shortly".to_owned(),
            ErrorDetail {
                reason: Some(DetailReason::Busy),
                ..Default::default()
            },
        )
    }
}
