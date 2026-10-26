//! robots.txt (RFC 012; REQ-CMP-005, D-08): one entry per origin kept for
//! LIM-F12 in a cache bounded to LIM-F15 entries, read through the agent
//! client under the same policy as the page, bounded to LIM-F13 bytes, and
//! matched for the product token `rypsok` and `*` by Google's parser. As
//! RFC 9309 says: 4xx allows everything, 5xx and unreachable disallow
//! everything (kept for one hour so that an outage is retried).

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use tokio::time::Instant;
use url::Url;

use crate::egress::{EgressPolicy, Target};

/// The product token robots.txt groups are matched for.
pub const PRODUCT_TOKEN: &str = "rypsok";

/// What a robots.txt said, or what its absence or failure means.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    /// The rules, to be matched per URL.
    Rules(String),
    /// No rules apply (absent, not text, or empty).
    AllowAll,
    /// Unreachable: nothing may be fetched (RFC 9309 §2.3.1.4).
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
}

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
            None => {
                let verdict = read(client, policy, &origin, limits).await;
                self.store(origin.clone(), verdict.clone());
                verdict
            }
        };
        match verdict {
            Verdict::AllowAll => Decision::Allowed,
            Verdict::DisallowAll => Decision::Disallowed,
            Verdict::Rules(body) => {
                let mut matcher = robotstxt::DefaultMatcher::default();
                if matcher.one_agent_allowed_by_robots(&body, PRODUCT_TOKEN, target.url.as_str()) {
                    Decision::Allowed
                } else {
                    Decision::Disallowed
                }
            }
        }
    }

    /// How many origins are cached.
    pub fn len(&self) -> usize {
        self.entries.lock().expect("robots lock").map.len()
    }

    /// Whether nothing is cached.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn fresh(&self, origin: &str) -> Option<Verdict> {
        let entries = self.entries.lock().expect("robots lock");
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
        let mut entries = self.entries.lock().expect("robots lock");
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

/// `scheme://host[:port]` of a URL.
pub fn origin_of(url: &Url) -> String {
    match url.port() {
        Some(port) => format!("{}://{}:{port}", url.scheme(), url.host_str().unwrap_or("")),
        None => format!("{}://{}", url.scheme(), url.host_str().unwrap_or("")),
    }
}

/// Reads `<origin>/robots.txt` and turns the outcome into a verdict.
async fn read(
    client: &reqwest::Client,
    policy: &EgressPolicy,
    origin: &str,
    limits: ReadLimits,
) -> Verdict {
    let Ok(mut url) = Url::parse(&format!("{origin}/robots.txt")) else {
        return Verdict::DisallowAll;
    };
    let mut hops = 0u8;
    loop {
        let Ok(target) = policy.judge_url(&url) else {
            return Verdict::DisallowAll;
        };
        let response = match client.get(target.url.clone()).send().await {
            Ok(response) => response,
            Err(_) => {
                tracing::debug!(event = "robots_unreachable", "robots.txt could not be read");
                return Verdict::DisallowAll;
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
                return Verdict::DisallowAll;
            };
            hops += 1;
            // Followed across origins too, as RFC 9309 asks; every hop
            // passes the policy at the top of the loop.
            if hops > limits.max_redirects {
                return Verdict::DisallowAll;
            }
            url = location;
            continue;
        }
        if status.is_client_error() {
            return Verdict::AllowAll;
        }
        if !status.is_success() {
            return Verdict::DisallowAll;
        }
        let is_text = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_none_or(|t| t.trim_start().to_ascii_lowercase().starts_with("text/"));
        if !is_text {
            return Verdict::AllowAll;
        }
        return match read_prefix(response, limits.max_bytes).await {
            Some(body) => Verdict::Rules(String::from_utf8_lossy(&body).into_owned()),
            None => Verdict::DisallowAll,
        };
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
