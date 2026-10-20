//! One module per driver, each behind a Cargo feature: `wikipedia` under
//! `engine-wikipedia`, `brave` under `engine-brave` (RFC 006).
//!
//! Every driver declares the configuration keys it accepts; the
//! configuration refuses any other key in its table (RFC 004). Drivers
//! deliver raw text and classified errors; the provider's words never
//! leave a driver above the `debug` level.

#[cfg(feature = "engine-brave")]
pub mod brave;
#[cfg(feature = "engine-wikipedia")]
pub mod wikipedia;

use std::time::Duration;

use reqwest::header::HeaderMap;

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
        _ => None,
    }
}

/// Whether a driver type needs a credential.
pub fn needs_credential(kind: &str) -> bool {
    kind == "brave"
}

/// What every driver gets when it is built.
#[derive(Debug, Clone)]
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
}

impl DriverContext {
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

/// Logs provider error text at `debug`, truncated, and nothing else
/// (REQ-PRIV-004, REQ-SEC-016).
pub(crate) fn debug_provider_text(kind: &'static str, status: u16, body: &[u8]) {
    let text: String = String::from_utf8_lossy(body).chars().take(200).collect();
    tracing::debug!(event = "provider_error", driver = kind, status, provider_text = %text, "provider answered an error");
}

#[cfg(test)]
mod tests;
