//! The egress component and its two profiles: the engine profile for
//! provider requests (milestone M1; REQ-SEC-017) and the agent-directed
//! profile for fetches (milestone M3; RFC 011) (external design §8).
//!
//! The engine profile builds one HTTP client per engine: no redirects, no
//! environment proxies, HTTPS only unless the operator admitted a plain
//! endpoint, a fixed identifying User-Agent, bounded timeouts, and the
//! credential header set on the client so that it can reach one origin
//! only. Bodies are read with a byte bound after decompression.

use std::sync::Once;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};

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
