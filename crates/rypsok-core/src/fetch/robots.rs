//! robots.txt (RFC 012; REQ-CMP-005, D-08): one entry per origin kept for
//! LIM-F12 in a cache bounded to LIM-F15 entries, read through the agent
//! client under the same policy as the page, bounded to LIM-F13 bytes, and
//! matched for the product token `rypsok` and `*` by Google's parser. As
//! RFC 9309 says: 4xx allows everything and a 5xx answer disallows
//! everything (kept for one hour so that an outage is retried). A site
//! that cannot be reached at all gives no verdict and nothing is cached:
//! the page read would fail the same way, and a transport failure is
//! never stated as a rule (RFC 019 D2).

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use tokio::time::Instant;
use url::Url;

use crate::egress::{EgressPolicy, Host, Target};
use crate::sync;

/// The product token robots.txt groups are matched for.
pub const PRODUCT_TOKEN: &str = "rypsok";

/// What a robots.txt said, or what its absence or failure means.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    /// The rules, to be matched per URL.
    Rules(String),
    /// No rules apply (absent, not text, or empty).
    AllowAll,
    /// The server answered with an error: nothing may be fetched (RFC 9309
    /// §2.3.1.4).
    DisallowAll,
}

#[derive(Debug, Clone)]
struct Entry {
    verdict: Verdict,
    fetched: Instant,
    ttl: Duration,
}

/// The decision for one URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The page may be fetched.
    Allowed,
    /// The page is disallowed for rypsok.
    Disallowed,
    /// The site could not be reached, whatever the reason (no address, a
    /// refused connection, a timeout, a connect-time policy refusal): no
    /// verdict, nothing cached (RFC 019 D2).
    Unreachable,
}

/// A robots.txt read that ended in a transport failure.
#[derive(Debug, Clone, Copy)]
struct Unreachable;

/// What a robots.txt read needs.
#[derive(Debug, Clone, Copy)]
pub struct ReadLimits {
    /// LIM-F13.
    pub max_bytes: u64,
    /// LIM-F7, for same-origin redirects of the robots.txt itself.
    pub max_redirects: u8,
}

#[derive(Debug, Default)]
struct Entries {
    map: HashMap<String, Entry>,
    order: VecDeque<String>,
}

/// The cache.
#[derive(Debug)]
pub struct Robots {
    ttl: Duration,
    error_ttl: Duration,
    capacity: usize,
    entries: Mutex<Entries>,
}

impl Robots {
    /// A cache keeping entries for `ttl` (LIM-F12), failures for a
    /// twenty-fourth of it, at most `capacity` origins (LIM-F15).
    pub fn new(ttl: Duration, capacity: u32) -> Self {
        Robots {
            ttl,
            error_ttl: ttl / 24,
            capacity: usize::try_from(capacity).unwrap_or(usize::MAX).max(1),
            entries: Mutex::new(Entries::default()),
        }
    }

    /// Decides whether `target` may be fetched, reading the origin's
    /// robots.txt through `client` when the cache has no fresh entry.
    pub async fn check(
        &self,
        client: &reqwest::Client,
        policy: &EgressPolicy,
        target: &Target,
        limits: ReadLimits,
    ) -> Decision {
        let origin = origin_of(&target.url);
        let verdict = match self.fresh(&origin) {
            Some(verdict) => verdict,
            None => match read(client, policy, &origin, limits).await {
                Ok(verdict) => {
                    self.store(origin.clone(), verdict.clone());
                    verdict
                }
                Err(Unreachable) => return Decision::Unreachable,
            },
        };
        decide(&verdict, &target.url)
    }

    /// How many origins are cached.
    pub fn len(&self) -> usize {
        sync::lock(&self.entries).map.len()
    }

    /// Whether nothing is cached.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn fresh(&self, origin: &str) -> Option<Verdict> {
        let entries = sync::lock(&self.entries);
        entries
            .map
            .get(origin)
            .filter(|e| e.fetched.elapsed() < e.ttl)
            .map(|e| e.verdict.clone())
    }

    fn store(&self, origin: String, verdict: Verdict) {
        let ttl = if verdict == Verdict::DisallowAll {
            self.error_ttl
        } else {
            self.ttl
        };
        let mut entries = sync::lock(&self.entries);
        if !entries.map.contains_key(&origin) {
            while entries.map.len() >= self.capacity {
                match entries.order.pop_front() {
                    Some(oldest) => {
                        entries.map.remove(&oldest);
                    }
                    None => break,
                }
            }
            entries.order.push_back(origin.clone());
        }
        entries.map.insert(
            origin,
            Entry {
                verdict,
                fetched: Instant::now(),
                ttl,
            },
        );
    }
}

/// What a verdict means for one URL: the rules are matched for the product
/// token and `*`.
fn decide(verdict: &Verdict, url: &Url) -> Decision {
    match verdict {
        Verdict::AllowAll => Decision::Allowed,
        Verdict::DisallowAll => Decision::Disallowed,
        Verdict::Rules(body) => {
            let mut matcher = robotstxt::DefaultMatcher::default();
            if matcher.one_agent_allowed_by_robots(body, PRODUCT_TOKEN, url.as_str()) {
                Decision::Allowed
            } else {
                Decision::Disallowed
            }
        }
    }
}

/// What a final (not redirecting) answer to the robots.txt request says,
/// before its body is read.
enum Answer {
    /// The verdict does not depend on the body.
    Verdict(Verdict),
    /// A text answer: the body is read, up to the bound, and is the rules.
    ReadBody,
}

/// The status and the media type of a final answer as RFC 9309 asks:
/// 4xx allows everything, anything else that is not a success disallows
/// everything, a success that is not text allows everything.
fn answer_for(status: reqwest::StatusCode, content_type: Option<&str>) -> Answer {
    if status.is_client_error() {
        return Answer::Verdict(Verdict::AllowAll);
    }
    if !status.is_success() {
        return Answer::Verdict(Verdict::DisallowAll);
    }
    let is_text =
        content_type.is_none_or(|t| t.trim_start().to_ascii_lowercase().starts_with("text/"));
    if is_text {
        Answer::ReadBody
    } else {
        Answer::Verdict(Verdict::AllowAll)
    }
}

/// The rules a body says, read up to `max_bytes` (a longer body is cut
/// there, as RFC 9309 allows a crawler to parse only a prefix).
fn rules_of(body: &[u8], max_bytes: u64) -> Verdict {
    let room = usize::try_from(max_bytes).unwrap_or(usize::MAX);
    Verdict::Rules(String::from_utf8_lossy(&body[..body.len().min(room)]).into_owned())
}

/// The decision for `url` from a final answer: the status, the media type
/// and the body of a robots.txt, with the body read to `max_bytes`. `None`
/// for a redirection, which the reader follows and never decides on. Public
/// for the fuzz targets; not part of the stable surface.
#[doc(hidden)]
pub fn decision_for_answer(
    status: u16,
    content_type: Option<&str>,
    body: &[u8],
    max_bytes: u64,
    url: &Url,
) -> Option<Decision> {
    let status = reqwest::StatusCode::from_u16(status).ok()?;
    if status.is_redirection() {
        return None;
    }
    let verdict = match answer_for(status, content_type) {
        Answer::Verdict(verdict) => verdict,
        Answer::ReadBody => rules_of(body, max_bytes),
    };
    Some(decide(&verdict, url))
}

/// `scheme://host[:port]` of a URL, with the host in its canonical form
/// (RFC 019 D1): `x.example` and `x.example.` share one cache entry.
pub fn origin_of(url: &Url) -> String {
    let host = match url.host() {
        Some(url::Host::Domain(name)) => match Host::from_name(name) {
            Ok(Host::Name(canonical)) => canonical,
            _ => name.to_owned(),
        },
        _ => url.host_str().unwrap_or("").to_owned(),
    };
    match url.port() {
        Some(port) => format!("{}://{host}:{port}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    }
}

/// Reads `<origin>/robots.txt` and turns the outcome into a verdict, or
/// reports that the site could not be reached.
async fn read(
    client: &reqwest::Client,
    policy: &EgressPolicy,
    origin: &str,
    limits: ReadLimits,
) -> Result<Verdict, Unreachable> {
    let Ok(mut url) = Url::parse(&format!("{origin}/robots.txt")) else {
        return Ok(Verdict::DisallowAll);
    };
    let mut hops = 0u8;
    loop {
        let Ok(target) = policy.judge_url(&url) else {
            return Ok(Verdict::DisallowAll);
        };
        let response = match client.get(target.url.clone()).send().await {
            Ok(response) => response,
            Err(error) => {
                // One classification for every send in the fetch path;
                // the caller learns nothing that tells a refused name from
                // an unreachable one (RFC 019 D2).
                super::classify_send_error(&error);
                return Err(Unreachable);
            }
        };
        let status = response.status();
        if status.is_redirection() {
            let Some(location) = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|l| url.join(l).ok())
            else {
                return Ok(Verdict::DisallowAll);
            };
            hops += 1;
            // Followed across origins too, as RFC 9309 asks; every hop
            // passes the policy at the top of the loop.
            if hops > limits.max_redirects {
                return Ok(Verdict::DisallowAll);
            }
            url = location;
            continue;
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        match answer_for(status, content_type.as_deref()) {
            Answer::Verdict(verdict) => return Ok(verdict),
            // A connection that fails while the body is read is a transport
            // failure like any other (RFC 019 D2).
            Answer::ReadBody => {
                return match read_prefix(response, limits.max_bytes).await {
                    Some(body) => Ok(rules_of(&body, limits.max_bytes)),
                    None => Err(Unreachable),
                };
            }
        }
    }
}

/// Reads at most `max_bytes` of the body; a longer body is cut there, as
/// RFC 9309 allows a crawler to parse only a prefix.
async fn read_prefix(mut response: reqwest::Response, max_bytes: u64) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        let room = (max_bytes as usize).saturating_sub(body.len());
        if room == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..chunk.len().min(room)]);
    }
    Some(body)
}

#[cfg(test)]
mod tests;
