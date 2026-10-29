//! The fetch pipeline (external design §3.2; RFC 012): validation, the
//! URL privacy check, strict mode, the egress policy, concurrency permits,
//! robots.txt, the request with manual redirects, the status, the media
//! type, the streaming byte bound, charset decoding, bounded parsing,
//! extraction, Markdown rendering, content cleaning and the window
//! (REQ-PAGE-001). Nothing is kept between calls except the robots.txt
//! cache and the strict-mode set.

use crate::html::charset;
use crate::html::dom;
pub mod extract;
pub mod markdown;
pub mod robots;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;
use url::Url;

use crate::CancellationToken;
use crate::boundary;
use crate::config::SecretDetection;
use crate::egress::policy::{Denial, Host, Target};
use crate::egress::resolver::{JudgingResolver, Lookup, SystemLookup};
use crate::egress::strict::ReturnedUrls;
use crate::egress::{self, AgentClientSettings, BodyError, ClientError, EgressPolicy};
use crate::limits::Limits;
use crate::model::{
    CallError, DetailReason, FetchRequest, FetchResponse, Notice, NoticeCode, Outcome, ToolError,
};
use crate::privacy::{self, Sensitive, Verdict};
use crate::search::normalize::{self, comparison_key};
use crate::telemetry::Counters;

/// How much of the head of a body is examined to tell HTML from text when
/// the server sent no type (REQ-PAGE-005).
const SNIFF_BYTES: usize = 1024;

/// The longest media type repeated in an error (external data, bounded).
const MEDIA_TYPE_MAX_CHARS: usize = 100;

/// How a request of the fetch path failed to be sent (RFC 019 D2). The
/// caller's answer is the same for both: the site could not be reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendFailure {
    /// No address, a refused or dropped connection, a timeout, a TLS
    /// failure: whatever the network said.
    Unreachable,
    /// The resolver judged every address of the name and the policy
    /// refused them all (REQ-SEC-007). Known to the operator's log only.
    Refused(Denial),
}

/// Classifies the error of a `send()` in the fetch path (the page request,
/// each redirect hop, the robots.txt read) by walking its source chain for
/// the resolver's refusal. A refusal is logged here, once, as `refused`
/// with the denial's name and no host; the caller gets the answer of an
/// unreachable site, so that the error kind is not an oracle for internal
/// names. Call it once per failed send.
pub(crate) fn classify_send_error(error: &reqwest::Error) -> SendFailure {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(current) = source {
        if let Some(refused) = current.downcast_ref::<crate::egress::resolver::Refused>() {
            return match refused {
                crate::egress::resolver::Refused::Policy(denial) => {
                    tracing::warn!(
                        event = "refused",
                        tool = "web_fetch",
                        reason = denial.reason().as_str(),
                        denial = denial.as_str(),
                        "a name was refused by the egress policy at connect time; the caller was told the site could not be reached"
                    );
                    SendFailure::Refused(*denial)
                }
                crate::egress::resolver::Refused::NoAddress => SendFailure::Unreachable,
            };
        }
        source = current.source();
    }
    SendFailure::Unreachable
}

/// What a document is parsed as (REQ-FETCH-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Media {
    /// HTML or XHTML: parsed, extracted, rendered.
    Html,
    /// Plain text or Markdown: the text is the content.
    Text,
}

/// What the pure stages produce for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The raw title: `<title>`, else the first `h1`, else empty.
    pub title: String,
    /// The rendered Markdown (or the text itself).
    pub content: String,
    /// Whether the node or depth bound cut the document.
    pub partial: bool,
}

/// Decodes, parses, extracts and renders one body: the stages after the
/// network, pure and synchronous, bounded by LIM-F10 and LIM-F11.
pub fn render_document(
    bytes: &[u8],
    media: Media,
    charset: Option<&str>,
    limits: &Limits,
) -> Document {
    let decoded = charset::decode(bytes, charset);
    match media {
        Media::Text => Document {
            title: String::new(),
            content: decoded.text,
            partial: false,
        },
        Media::Html => {
            let tree = dom::parse(&decoded.text, limits.html_max_nodes);
            let extraction = extract::extract(&tree, limits.html_max_depth);
            let rendered = markdown::render(
                &tree,
                extraction.root,
                &extraction.excluded,
                markdown::Options {
                    max_depth: limits.html_max_depth as usize,
                    max_url_chars: limits.url_max_chars as usize,
                },
            );
            Document {
                title: extraction.title,
                content: rendered.markdown,
                partial: tree.truncated() || extraction.depth_truncated || rendered.depth_truncated,
            }
        }
    }
}

/// The long-lived parts of the fetch pipeline: the agent client, the
/// robots.txt cache and the concurrency permits (REQ-SEC-012).
#[derive(Debug)]
pub struct FetchService {
    client: reqwest::Client,
    robots: robots::Robots,
    global: Arc<Semaphore>,
    parsing: Arc<Semaphore>,
    hosts: Mutex<HashMap<String, Arc<Semaphore>>>,
    fetches_in_flight: usize,
}

impl FetchService {
    /// Builds the client under the agent-directed profile and the permits
    /// from the limits.
    pub fn new(
        settings: &AgentClientSettings,
        policy: Arc<EgressPolicy>,
        limits: &Limits,
    ) -> Result<Self, ClientError> {
        Self::with_lookup(settings, policy, limits, SystemLookup)
    }

    /// The same over any name lookup (tests script the answers; the
    /// policy still judges every address).
    pub fn with_lookup<L: Lookup>(
        settings: &AgentClientSettings,
        policy: Arc<EgressPolicy>,
        limits: &Limits,
        lookup: L,
    ) -> Result<Self, ClientError> {
        let client = egress::agent_client(
            settings,
            Arc::new(JudgingResolver::with_lookup(policy, lookup)),
        )?;
        Ok(FetchService {
            client,
            robots: robots::Robots::new(
                Duration::from_secs(u64::from(limits.robots_ttl_s)),
                limits.robots_cache_entries,
            ),
            global: Arc::new(Semaphore::new(limits.fetches_in_flight as usize)),
            parsing: Arc::new(Semaphore::new(limits.parsing_in_flight as usize)),
            hosts: Mutex::new(HashMap::new()),
            fetches_in_flight: limits.fetches_in_flight as usize,
        })
    }

    /// The same service parsing under permits shared with the scraping
    /// drivers, so that LIM-C5 bounds the process (RFC 015).
    #[must_use]
    pub fn with_parsing(mut self, parsing: Arc<Semaphore>) -> Self {
        self.parsing = parsing;
        self
    }

    /// The robots.txt cache.
    pub fn robots(&self) -> &robots::Robots {
        &self.robots
    }

    /// The per-host permit (LIM-C4: one fetch per host at a time). Idle
    /// entries are pruned when the map outgrows the global limit.
    fn host_permit(&self, host: &str) -> Arc<Semaphore> {
        let mut hosts = self.hosts.lock().expect("hosts lock");
        if hosts.len() > self.fetches_in_flight {
            hosts.retain(|_, s| Arc::strong_count(s) > 1);
        }
        Arc::clone(
            hosts
                .entry(host.to_owned())
                .or_insert_with(|| Arc::new(Semaphore::new(1))),
        )
    }
}

/// What the pipeline needs from the process.
pub struct Pipeline<'a> {
    /// The limits.
    pub limits: &'a Limits,
    /// The agent-directed policy (RFC 011).
    pub policy: &'a EgressPolicy,
    /// The client, cache and permits.
    pub service: &'a FetchService,
    /// The URLs `web_search` returned, for strict mode (REQ-SEC-021).
    pub returned: &'a ReturnedUrls,
    /// `[fetch].strict_mode`.
    pub strict_mode: bool,
    /// `[fetch].robots_txt`.
    pub robots_txt: bool,
    /// `[fetch].follow_cross_host_redirects`.
    pub follow_cross_host_redirects: bool,
    /// The secret-detection mode.
    pub secret_detection: SecretDetection,
    /// The counters.
    pub counters: &'a Counters,
}

/// Runs one `web_fetch` call under LIM-F5. A cancelled call ends as
/// [`CallError::Cancelled`] and whatever was in flight is dropped.
pub async fn run(
    p: &Pipeline<'_>,
    req: FetchRequest,
    cancel: CancellationToken,
) -> Result<FetchResponse, CallError> {
    let start = Instant::now();
    tokio::select! {
        _ = cancel.cancelled() => {
            p.counters.call("web_fetch", "cancelled");
            tracing::info!(event = "call", tool = "web_fetch", outcome = "cancelled", latency_ms = start.elapsed().as_millis() as u64, "call cancelled by the caller");
            Err(CallError::Cancelled)
        }
        outcome = run_inner(p, req, start) => {
            match outcome {
                Ok(response) => {
                    p.counters.call("web_fetch", "ok");
                    tracing::info!(event = "call", tool = "web_fetch", outcome = "ok", latency_ms = start.elapsed().as_millis() as u64, count = response.content.chars().count(), "call ended");
                    Ok(response)
                }
                Err(error) => {
                    let kind = error.kind.as_str();
                    if error.kind == crate::model::ErrorKind::Denied {
                        p.counters.refused("web_fetch", error.detail.reason.map_or("denied", DetailReason::as_str));
                    } else {
                        p.counters.call("web_fetch", kind);
                    }
                    tracing::info!(event = "call", tool = "web_fetch", outcome = kind, reason = error.detail.reason.map(DetailReason::as_str), latency_ms = start.elapsed().as_millis() as u64, "call ended");
                    Err(CallError::Tool(error))
                }
            }
        }
    }
}

/// What the network phase delivers to the parse phase.
struct Fetched {
    url: Url,
    body: Vec<u8>,
    media: Media,
    charset: Option<String>,
}

async fn run_inner(
    p: &Pipeline<'_>,
    req: FetchRequest,
    start: Instant,
) -> Result<FetchResponse, ToolError> {
    let limits = p.limits;
    let network_deadline = start + Duration::from_millis(u64::from(limits.fetch_deadline_ms));
    let call_deadline = start + Duration::from_millis(u64::from(limits.fetch_call_max_ms()));

    // Validation, in parameter order (REQ-FETCH-001, -003, -005).
    let url_text = Sensitive::new(req.url.trim());
    if url_text.chars() == 0 {
        return Err(ToolError::missing("url"));
    }
    if url_text.chars() > limits.url_max_chars as usize {
        return Err(ToolError::wrong_type(
            "url",
            "an absolute http or https URL of at most LIM-F9 characters",
        ));
    }
    let max_chars = req.max_chars.unwrap_or(limits.fetch_max_chars);
    if !(500..=limits.fetch_max_chars_ceiling).contains(&max_chars) {
        return Err(ToolError::out_of_range(
            "max_chars",
            500,
            limits.fetch_max_chars_ceiling,
        ));
    }
    let start_char = req.start_char.unwrap_or(0);

    // The privacy check of the whole URL (REQ-PRIV-002); mask mode cannot
    // apply to a URL and behaves as refuse.
    let mode = match p.secret_detection {
        SecretDetection::Mask => SecretDetection::Refuse,
        other => other,
    };
    match privacy::check(url_text.expose(), mode) {
        Verdict::Clean | Verdict::Masked { .. } => {}
        Verdict::Refuse(finding) => {
            tracing::warn!(event = "refused", tool = "web_fetch", reason = "secret_in_url", kind = ?finding.kind, "call refused");
            return Err(ToolError::secret_in_url(finding.kind, finding.start));
        }
        Verdict::Warned(count) => {
            p.counters.warned("web_fetch", "secret_in_url");
            tracing::warn!(
                event = "secret_in_url",
                tool = "web_fetch",
                count,
                "URL carries what looks like a credential; proceeding (warn mode)"
            );
        }
    }

    // Scheme (REQ-FETCH-010, D-18): http is retried over https unless the
    // operator allowed plain http.
    let mut url = Url::parse(url_text.expose())
        .map_err(|_| ToolError::wrong_type("url", "an absolute http or https URL"))?;
    let upgraded = url.scheme() == "http" && !p.policy.allows_http();
    if upgraded && url.set_scheme("https").is_err() {
        return Err(ToolError::denied(DetailReason::Scheme));
    }

    // Strict mode (REQ-SEC-021).
    if p.strict_mode {
        let host = match url.host() {
            Some(url::Host::Domain(name)) => Host::from_name(name).map_err(refused)?,
            Some(url::Host::Ipv4(ip)) => Host::Ip(ip.into()),
            Some(url::Host::Ipv6(ip)) => Host::Ip(ip.into()),
            None => return Err(refused(Denial::HostName)),
        };
        // The key of the URL as the search pipeline would have returned it:
        // tracking parameters removed first (RFC 008).
        let key = normalize::returned_url(url.as_str(), limits.url_max_chars)
            .map(|returned| comparison_key(&returned))
            .unwrap_or_else(|_| comparison_key(url.as_str()));
        if !(p.returned.contains(&key) || p.policy.allowed_by_rule(&host)) {
            tracing::warn!(
                event = "refused",
                tool = "web_fetch",
                reason = "strict_mode",
                "call refused"
            );
            return Err(ToolError::denied(DetailReason::StrictMode));
        }
    }

    // The egress policy before any I/O (RFC 011).
    let target = p.policy.judge_url(&url).map_err(refused)?;
    let host_key = match &target.host {
        Host::Name(name) => name.clone(),
        Host::Ip(ip) => ip.to_string(),
    };

    // Concurrency (REQ-SEC-012): the process-wide and the per-host permit.
    let _global = acquire(&p.service.global, network_deadline).await?;
    let host_permit = p.service.host_permit(&host_key);
    let _host = acquire(&host_permit, network_deadline).await?;

    // The network phase under LIM-F4.
    let fetched =
        match tokio::time::timeout_at(network_deadline, network(p, target, upgraded)).await {
            Ok(result) => result?,
            Err(_) => return Err(ToolError::deadline()),
        };
    drop(_host);
    drop(_global);

    // The parse phase off the I/O path (REQ-PAGE-008), under LIM-C5 and
    // the call deadline; a panic inside ends the call, not the process.
    let _parsing = acquire(&p.service.parsing, call_deadline).await?;
    let limits_for_task = limits.clone();
    let media = fetched.media;
    let charset = fetched.charset.clone();
    let body = fetched.body;
    let task = tokio::task::spawn_blocking(move || {
        render_document(&body, media, charset.as_deref(), &limits_for_task)
    });
    let document = match tokio::time::timeout_at(call_deadline, task).await {
        Ok(Ok(document)) => document,
        Ok(Err(join_error)) => {
            tracing::error!(
                event = "extraction_panic",
                panicked = join_error.is_panic(),
                "extraction failed; the call ends"
            );
            return Err(ToolError::extraction_failed());
        }
        Err(_) => return Err(ToolError::deadline()),
    };
    drop(_parsing);
    if media == Media::Html && document.content.trim().is_empty() {
        return Err(ToolError::extraction_failed());
    }

    // The boundary (RFC 005, RFC 012) and the window.
    let mut notices = Vec::new();
    let title = boundary::clean_decoded_text(&document.title, limits.title_max_chars);
    if title.truncated {
        notices.push(Notice::counted(NoticeCode::TitleTruncated, 1));
    }
    let content = boundary::clean_content(&document.content, start_char, max_chars)
        .map_err(|e| ToolError::start_beyond_end(e.total_chars))?;
    let removed = title.removed + content.removed;
    if removed > 0 {
        notices.push(Notice::counted(NoticeCode::CharactersRemoved, removed));
        p.counters.characters_removed(u64::from(removed));
    }
    if document.partial {
        notices.push(Notice::new(NoticeCode::ExtractionPartial));
    }
    if boundary::has_instruction_like_pattern(&content.text)
        || boundary::has_instruction_like_pattern(&title.text)
    {
        notices.push(Notice::counted(NoticeCode::InstructionLikeContent, 1));
    }
    tracing::debug!(event = "fetched", host = %host_key, chars = content.total_chars, "page read");
    Ok(FetchResponse {
        outcome: Outcome::Ok,
        notices,
        url: fetched.url.to_string(),
        title: title.text,
        content: content.text,
        truncated: content.truncated,
        next_start_char: content.next_start_char,
    })
}

/// A permit before the deadline, or `busy` (REQ-SEC-012).
async fn acquire(
    semaphore: &Arc<Semaphore>,
    deadline: Instant,
) -> Result<OwnedSemaphorePermit, ToolError> {
    match tokio::time::timeout_at(deadline, Arc::clone(semaphore).acquire_owned()).await {
        Ok(Ok(permit)) => Ok(permit),
        _ => Err(ToolError::busy()),
    }
}

/// A refusal by the policy: logged with its denial name, never with the
/// URL, the host or the address.
fn refused(denial: Denial) -> ToolError {
    tracing::warn!(
        event = "refused",
        tool = "web_fetch",
        reason = denial.reason().as_str(),
        denial = denial.as_str(),
        "call refused"
    );
    ToolError::denied(denial.reason())
}

/// robots.txt, request, redirects, status, media type and the streamed body
/// (REQ-CMP-005, REQ-FETCH-006, REQ-FETCH-011, REQ-PAGE-003 to -005).
async fn network(
    p: &Pipeline<'_>,
    mut target: Target,
    upgraded: bool,
) -> Result<Fetched, ToolError> {
    let limits = p.limits;
    let read_limits = robots::ReadLimits {
        max_bytes: limits.robots_max_bytes,
        max_redirects: limits.fetch_max_redirects,
    };
    if p.robots_txt {
        let decision = p
            .service
            .robots
            .check(&p.service.client, p.policy, &target, read_limits)
            .await;
        robots_gate(decision)?;
    }
    let mut hops = 0u8;
    loop {
        let mut response = match p.service.client.get(target.url.clone()).send().await {
            Ok(response) => response,
            Err(error) => {
                let failure = classify_send_error(&error);
                if failure == SendFailure::Unreachable && error.is_timeout() {
                    return Err(ToolError::deadline());
                }
                if upgraded && hops == 0 {
                    tracing::warn!(
                        event = "refused",
                        tool = "web_fetch",
                        reason = "scheme",
                        "the https retry of an http URL failed"
                    );
                    return Err(ToolError::denied(DetailReason::Scheme));
                }
                // A refused name and an unreachable one answer alike.
                return Err(ToolError::network());
            }
        };
        let status = response.status();
        if status.is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|l| target.url.join(l).ok())
                .ok_or_else(ToolError::network)?;
            if location.as_str().chars().count() > limits.url_max_chars as usize {
                return Err(ToolError::network());
            }
            // Every target passes the same profile (REQ-SEC-008); a refused
            // one is not disclosed.
            let next = p.policy.judge_url(&location).map_err(refused)?;
            let same_host = next.host == target.host && next.port == target.port;
            if !same_host && !p.follow_cross_host_redirects {
                return Err(ToolError::redirected(location.to_string()));
            }
            hops += 1;
            if hops > limits.fetch_max_redirects {
                return Err(ToolError::network());
            }
            if !same_host && p.robots_txt {
                let decision = p
                    .service
                    .robots
                    .check(&p.service.client, p.policy, &next, read_limits)
                    .await;
                robots_gate(decision)?;
            }
            target = next;
            continue;
        }
        if status.is_client_error() || status.is_server_error() {
            // The body is never read, parsed or returned (REQ-FETCH-011).
            return Err(ToolError::http_status(status.as_u16()));
        }
        if !status.is_success() {
            return Err(ToolError::network());
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let (essence, charset) = match &content_type {
            Some(value) => (
                value
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase(),
                charset::charset_parameter(value).map(str::to_owned),
            ),
            None => (String::new(), None),
        };
        let declared = match essence.as_str() {
            "text/html" | "application/xhtml+xml" => Some(Media::Html),
            "text/plain" | "text/markdown" => Some(Media::Text),
            "" => None,
            other => {
                return Err(ToolError::unsupported_media_type(bounded_media_type(other)));
            }
        };
        let body = match egress::read_body(&mut response, limits.fetch_max_bytes).await {
            Ok(body) => body,
            Err(BodyError::TooLarge) => return Err(ToolError::too_large(limits.fetch_max_bytes)),
            Err(BodyError::Transport) => return Err(ToolError::network()),
        };
        let media = declared.unwrap_or_else(|| sniff(&body));
        return Ok(Fetched {
            url: target.url,
            body,
            media,
            charset,
        });
    }
}

/// What a robots.txt decision means for the call: disallowed is `denied`
/// with the reason `robots_txt`; a site that could not be reached is the
/// same `unavailable` answer as the page read would give (RFC 019 D2).
fn robots_gate(decision: robots::Decision) -> Result<(), ToolError> {
    match decision {
        robots::Decision::Allowed => Ok(()),
        robots::Decision::Disallowed => {
            tracing::warn!(
                event = "refused",
                tool = "web_fetch",
                reason = "robots_txt",
                "call refused"
            );
            Err(ToolError::denied(DetailReason::RobotsTxt))
        }
        robots::Decision::Unreachable => Err(ToolError::network()),
    }
}

/// HTML or text, from the first bytes, when the server sent no type.
fn sniff(body: &[u8]) -> Media {
    let head: String =
        String::from_utf8_lossy(&body[..body.len().min(SNIFF_BYTES)]).to_ascii_lowercase();
    if ["<!doctype html", "<html", "<head", "<body"]
        .iter()
        .any(|marker| head.contains(marker))
    {
        Media::Html
    } else {
        Media::Text
    }
}

/// A media type as it may be repeated in an error: ASCII token characters
/// only, bounded.
fn bounded_media_type(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_graphic())
        .take(MEDIA_TYPE_MAX_CHARS)
        .collect()
}

#[cfg(test)]
mod tests;
