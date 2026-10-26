//! The egress component and its two profiles: the engine profile for
//! provider requests (milestone M1; REQ-SEC-017) and the agent-directed
//! profile for fetches (milestone M3; RFC 011) (external design §8).
//!
//! The engine profile builds one HTTP client per engine: no redirects, no
//! environment proxies, HTTPS only unless the operator admitted a plain
//! endpoint, a fixed identifying User-Agent, bounded timeouts, and the
//! credential header set on the client so that it can reach one origin
//! only. Bodies are read with a byte bound after decompression.
//!
//! The agent-directed profile builds one client for every fetch and
//! robots.txt read: the [`policy`] judges URLs before I/O and addresses at
//! connect time through the [`resolver`]; no redirects, no proxy, no
//! cookies, GET only; [`strict`] narrows destinations to what rypsok
//! returned.

pub mod policy;
pub mod resolver;
pub mod strict;

use std::sync::{Arc, Once};
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};

pub use policy::{Denial, EgressPolicy, Host, Rule, Target};
pub use resolver::JudgingResolver;
pub use strict::ReturnedUrls;

/// What the engine profile needs to build a client.
#[derive(Debug, Clone)]
pub struct EngineClientSettings {
    /// The User-Agent of REQ-CMP-004.
    pub user_agent: String,
    /// LIM-S12.
    pub connect_timeout: Duration,
    /// LIM-S5.
    pub timeout: Duration,
    /// A proxy for engine traffic, when configured (REQ-RES-008).
    pub proxy: Option<String>,
    /// Refuse plain HTTP; off only for a loopback mock in tests.
    pub https_only: bool,
    /// Headers sent on every request, including a credential.
    pub headers: HeaderMap,
}

/// Why a client could not be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClientError {
    /// The proxy URL is not usable.
    #[error("the proxy URL is not usable")]
    Proxy,
    /// The HTTP client could not be built.
    #[error("the HTTP client could not be built: {0}")]
    Build(String),
}

/// Installs the TLS crypto provider once per process.
pub fn ensure_crypto_provider() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Builds a client under the engine profile.
pub fn engine_client(settings: &EngineClientSettings) -> Result<reqwest::Client, ClientError> {
    ensure_crypto_provider();
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .https_only(settings.https_only)
        .user_agent(
            HeaderValue::from_str(&settings.user_agent)
                .map_err(|e| ClientError::Build(e.to_string()))?,
        )
        .default_headers(settings.headers.clone())
        .connect_timeout(settings.connect_timeout)
        .timeout(settings.timeout)
        .gzip(true);
    builder = match &settings.proxy {
        Some(proxy) if !proxy.is_empty() => {
            builder.proxy(reqwest::Proxy::all(proxy).map_err(|_| ClientError::Proxy)?)
        }
        _ => builder.no_proxy(),
    };
    builder
        .build()
        .map_err(|e| ClientError::Build(e.to_string()))
}

/// What the agent-directed profile needs to build its one client
/// (RFC 011).
#[derive(Debug, Clone)]
pub struct AgentClientSettings {
    /// The User-Agent of REQ-CMP-004.
    pub user_agent: String,
    /// LIM-F6.
    pub connect_timeout: Duration,
    /// LIM-F4.
    pub timeout: Duration,
    /// `[egress].allow_http`.
    pub allow_http: bool,
}

/// The `Accept` header of a fetch: the types `web_fetch` extracts
/// (REQ-FETCH-008) and nothing else.
pub const FETCH_ACCEPT: &str =
    "text/html, application/xhtml+xml, text/plain;q=0.9, text/markdown;q=0.9";

/// Builds the client of the agent-directed profile (REQ-SEC-007,
/// REQ-SEC-009): the judging resolver decides every connection; no
/// redirect is followed by the client; environment proxies are ignored and
/// no proxy is used; no cookie store; `User-Agent` and `Accept` are the
/// only default headers; idle connections are dropped after 30 s.
pub fn agent_client(
    settings: &AgentClientSettings,
    resolver: Arc<JudgingResolver>,
) -> Result<reqwest::Client, ClientError> {
    ensure_crypto_provider();
    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static(FETCH_ACCEPT),
    );
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .https_only(!settings.allow_http)
        .user_agent(
            HeaderValue::from_str(&settings.user_agent)
                .map_err(|e| ClientError::Build(e.to_string()))?,
        )
        .default_headers(headers)
        .connect_timeout(settings.connect_timeout)
        .timeout(settings.timeout)
        .gzip(true)
        .no_proxy()
        .dns_resolver(resolver)
        .pool_idle_timeout(Some(Duration::from_secs(30)))
        .build()
        .map_err(|e| ClientError::Build(e.to_string()))
}

/// Why a body could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyError {
    /// More than the bound, after decompression.
    TooLarge,
    /// The connection failed or timed out while reading.
    Transport,
}

/// Reads at most `max_bytes` of a response body, after decompression.
pub async fn read_body(
    response: &mut reqwest::Response,
    max_bytes: u64,
) -> Result<Vec<u8>, BodyError> {
    if response
        .content_length()
        .is_some_and(|declared| declared > max_bytes)
    {
        return Err(BodyError::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| BodyError::Transport)? {
        if body.len() as u64 + chunk.len() as u64 > max_bytes {
            return Err(BodyError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// The standard User-Agent (REQ-CMP-004): `rypsok/<version> (+<project
/// URL>; <operator contact>)`.
pub fn standard_user_agent(contact: &str) -> String {
    let contact = contact.trim();
    if contact.is_empty() {
        format!(
            "rypsok/{} (+https://github.com/nabbisen/rypsok)",
            env!("CARGO_PKG_VERSION")
        )
    } else {
        format!(
            "rypsok/{} (+https://github.com/nabbisen/rypsok; {contact})",
            env!("CARGO_PKG_VERSION")
        )
    }
}

#[cfg(test)]
mod tests;
