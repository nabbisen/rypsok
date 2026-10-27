//! One module per driver, each behind a Cargo feature: `wikipedia` under
//! `engine-wikipedia`, `brave` under `engine-brave` (RFC 006).
//!
//! Every driver declares the configuration keys it accepts; the
//! configuration refuses any other key in its table (RFC 004). Drivers
//! deliver raw text and classified errors; the provider's words never
//! leave a driver above the `debug` level.

#[cfg(feature = "engine-brave")]
pub mod brave;
#[cfg(feature = "engine-duckduckgo")]
pub mod duckduckgo;
#[cfg(feature = "engine-wikipedia")]
pub mod wikipedia;

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::HeaderMap;
use tokio::sync::Semaphore;

use super::Driver;
use crate::config::{EngineTable, Secret};
use crate::egress::{self, ClientError, EngineClientSettings};

/// The configuration keys a driver type declares, or `None` for an unknown
/// type. Drivers compiled out by feature are unknown.
pub fn declared_keys(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        #[cfg(feature = "engine-wikipedia")]
        "wikipedia" => Some(&["base_url"]),
        #[cfg(feature = "engine-brave")]
        "brave" => Some(&["api_key_env", "api_key_file", "endpoint"]),
        #[cfg(feature = "engine-duckduckgo")]
        "duckduckgo" => Some(&[]),
        _ => None,
    }
}

/// Whether a driver type needs a credential.
pub fn needs_credential(kind: &str) -> bool {
    kind == "brave"
}

/// Whether a driver type reads a human-facing interface (RFC 015): such
/// a table is refused unless `[scraping] enabled = true` (REQ-ENG-013).
pub fn is_scraping(kind: &str) -> bool {
    kind == "duckduckgo"
}

/// What every driver gets when it is built.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DriverContext {
    /// The standard User-Agent (REQ-CMP-004).
    pub user_agent: String,
    /// LIM-S12.
    pub connect_timeout: Duration,
    /// LIM-S5.
    pub timeout: Duration,
    /// Proxy for engine traffic (REQ-RES-008).
    pub proxy: Option<String>,
    /// Refuse plain HTTP; tests with a loopback mock turn it off.
    pub https_only: bool,
    /// A base URL replacing the provider's, for tests.
    pub base_override: Option<url::Url>,
    /// The process-wide parsing permits (LIM-C5), shared with the fetch
    /// pipeline; scraping drivers parse under them (REQ-ENG-015).
    pub parsing: Arc<Semaphore>,
    /// LIM-F10, for scraping drivers.
    pub html_max_nodes: u32,
}

impl DriverContext {
    /// A context with this User-Agent and the defaults of the limits
    /// table: for a driver author's tests. Set the public fields
    /// afterwards (`base_override`, `https_only`).
    pub fn new(user_agent: impl Into<String>) -> Self {
        use crate::limits::defaults;
        DriverContext {
            user_agent: user_agent.into(),
            connect_timeout: Duration::from_millis(u64::from(defaults::ENGINE_CONNECT_TIMEOUT_MS)),
            timeout: Duration::from_millis(u64::from(defaults::ENGINE_TIMEOUT_MS)),
            proxy: None,
            https_only: true,
            base_override: None,
            parsing: Arc::new(Semaphore::new(defaults::PARSING_IN_FLIGHT as usize)),
            html_max_nodes: defaults::HTML_MAX_NODES,
        }
    }

    /// The client settings for a driver with these headers.
    pub fn client_settings(&self, headers: HeaderMap) -> EngineClientSettings {
        EngineClientSettings {
            user_agent: self.user_agent.clone(),
            connect_timeout: self.connect_timeout,
            timeout: self.timeout,
            proxy: self.proxy.clone(),
            https_only: self.https_only,
            headers,
        }
    }
}

/// Why a driver could not be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DriverBuildError {
    /// No driver of this type is compiled in.
    #[error("no such driver")]
    UnknownKind,
    /// A declared key has a value the driver does not accept.
    #[error("key {0}: {1}")]
    Key(String, String),
    /// The HTTP client could not be built.
    #[error(transparent)]
    Client(#[from] ClientError),
}

/// Builds the driver of a table. `secret` is the resolved credential when
/// the driver needs one.
pub fn build(
    table: &EngineTable,
    secret: Option<&Secret<String>>,
    context: &DriverContext,
) -> Result<Box<dyn Driver>, DriverBuildError> {
    let _ = egress::ensure_crypto_provider;
    match table.kind.as_str() {
        #[cfg(feature = "engine-wikipedia")]
        "wikipedia" => Ok(Box::new(wikipedia::Wikipedia::new(table, context)?)),
        #[cfg(feature = "engine-brave")]
        "brave" => Ok(Box::new(brave::Brave::new(table, secret, context)?)),
        #[cfg(feature = "engine-duckduckgo")]
        "duckduckgo" => Ok(Box::new(duckduckgo::DuckDuckGo::new(context)?)),
        _ => Err(DriverBuildError::UnknownKind),
    }
}

/// Whether a URL points at the local host, where plain HTTP is admitted
/// for a private mirror or a test server.
pub fn is_loopback(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        Some(url::Host::Domain(name)) => name == "localhost",
        None => false,
    }
}

/// Reads a `Retry-After` header in seconds, when present and sane.
pub(crate) fn retry_after(headers: &HeaderMap, name: &str) -> Option<Duration> {
    headers
        .get(name)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|s| *s > 0 && *s <= 86_400)
        .map(Duration::from_secs)
}

/// Most characters of a provider's error body a `debug` line may carry
/// (external design §16 I5; RFC 018).
pub const PROVIDER_TEXT_MAX_CHARS: usize = 200;

/// The provider's error body cut to [`PROVIDER_TEXT_MAX_CHARS`].
pub(crate) fn bounded_provider_text(body: &[u8]) -> String {
    String::from_utf8_lossy(body)
        .chars()
        .take(PROVIDER_TEXT_MAX_CHARS)
        .collect()
}

/// Logs provider error text at `debug`, bounded, and nothing else
/// (REQ-PRIV-004, REQ-SEC-016; external design §16 I5).
pub(crate) fn debug_provider_text(kind: &'static str, status: u16, body: &[u8]) {
    let text = bounded_provider_text(body);
    tracing::debug!(event = "provider_error", driver = kind, status, provider_text = %text, "provider answered an error");
}

#[cfg(test)]
mod tests;
