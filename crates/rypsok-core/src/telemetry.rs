//! Structured logging on standard error, counters and the health state of
//! every engine (external design §13.3; RFC 009 and RFC 010). Nothing here
//! ever receives a query, a URL path or page content (REQ-PRIV-004).

pub mod counters;
pub mod health;

use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;

use crate::config::LogLevel;

pub use counters::Counters;
pub use health::HealthTable;

/// The crates whose events follow the configured level. Every other
/// target is capped at [`FOREIGN_CAP`] whatever the level, because a
/// dependency may log request parameters or URLs at `debug` (the MCP SDK
/// logs each request it receives, query included) and REQ-PRIV-004
/// forbids that.
pub const OWN_TARGETS: [&str; 2] = ["rypsok", "rypsok_core"];

/// The cap on every target outside [`OWN_TARGETS`].
pub const FOREIGN_CAP: LevelFilter = LevelFilter::WARN;

/// Installs the process-wide subscriber: one JSON event per line on
/// standard error at the configured level (REQ-OBS-005). Calling it twice
/// is harmless; the second call changes nothing.
pub fn init(level: LogLevel) {
    let layer = tracing_subscriber::fmt::layer()
        .json()
        .flatten_event(true)
        .with_timer(UtcTime::rfc_3339())
        .with_target(false)
        .with_writer(std::io::stderr);
    let _ = tracing_subscriber::registry()
        .with(layer.with_filter(filter(level)))
        .try_init();
}

/// The filter of [`init`]: the configured level for the crates of
/// [`OWN_TARGETS`], at most [`FOREIGN_CAP`] for everything else.
pub fn filter(level: LogLevel) -> Targets {
    let own = match level {
        LogLevel::Error => LevelFilter::ERROR,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Debug => LevelFilter::DEBUG,
    };
    Targets::new()
        .with_default(FOREIGN_CAP.min(own))
        .with_targets(OWN_TARGETS.map(|target| (target, own)))
}

/// A per-call identifier, unique within the process: the start time in
/// milliseconds and a sequence number, in hexadecimal.
pub fn request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format!(
        "{millis:011x}-{:06x}",
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests;
