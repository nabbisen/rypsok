//! Structured logging on standard error, counters and the health state of
//! every engine (external design §13.3; RFC 009 and RFC 010). Nothing here
//! ever receives a query, a URL path or page content (REQ-PRIV-004).

use tracing_subscriber::fmt::time::UtcTime;

use crate::config::LogLevel;

/// Installs the process-wide subscriber: one JSON event per line on
/// standard error at the configured level (REQ-OBS-005). Calling it twice
/// is harmless; the second call changes nothing.
pub fn init(level: LogLevel) {
    let max = match level {
        LogLevel::Error => tracing::Level::ERROR,
        LogLevel::Warn => tracing::Level::WARN,
        LogLevel::Info => tracing::Level::INFO,
        LogLevel::Debug => tracing::Level::DEBUG,
    };
    let _ = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_timer(UtcTime::rfc_3339())
        .with_max_level(max)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg(test)]
mod tests;
