//! Structured logging on standard error, counters and the health state of
//! every engine (external design §13.3; RFC 009 and RFC 010). Nothing here
//! ever receives a query, a URL path or page content (REQ-PRIV-004).

pub mod counters;
pub mod health;
mod line;

use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::prelude::*;

use crate::config::LogLevel;

pub use counters::Counters;
pub use health::HealthTable;
pub use line::{LineFormat, layer};

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
    let _ = tracing_subscriber::registry()
        .with(layer(std::io::stderr).with_filter(filter(level)))
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

/// The span of one tool call (RFC 020, TST-2): a request identifier and the
/// caller, which every event inside the call then carries as top-level fields
/// of its line. The id is the process's own, unique within it; it is not
/// returned to the caller. The span is at `error` level so that it exists at
/// every configured log level: a `warn` line of a refusal has the id too.
pub fn call_span(tool: &'static str, caller: &crate::model::CallerId) -> tracing::Span {
    tracing::error_span!("call", request_id = %request_id(), caller = %caller, tool = tool)
}

/// What the tests of this crate use to read the log: a subscriber built with
/// the layer the process installs, writing to a buffer.
#[cfg(test)]
pub(crate) mod capture {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use serde_json::Value;
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::prelude::*;

    /// The captured text of one test.
    #[derive(Clone, Default)]
    pub struct Buffer(Arc<Mutex<Vec<u8>>>);

    pub struct BufferWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for BufferWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            crate::sync::lock(&self.0).extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Buffer {
        type Writer = BufferWriter;

        fn make_writer(&'a self) -> BufferWriter {
            BufferWriter(Arc::clone(&self.0))
        }
    }

    impl Buffer {
        /// The lines written so far, each parsed (a line that is not a JSON
        /// object fails the test).
        pub fn lines(&self) -> Vec<Value> {
            let text = String::from_utf8(crate::sync::lock(&self.0).clone()).unwrap();
            text.lines()
                .map(|line| {
                    serde_json::from_str::<Value>(line)
                        .unwrap_or_else(|e| panic!("not a JSON line ({e}): {line}"))
                })
                .collect()
        }
    }

    /// A subscriber for the current thread at `debug` level, with the
    /// process's line layer; the guard ends it.
    pub fn at_debug() -> (Buffer, tracing::subscriber::DefaultGuard) {
        let buffer = Buffer::default();
        let subscriber = tracing_subscriber::registry().with(
            super::layer(buffer.clone()).with_filter(super::filter(crate::config::LogLevel::Debug)),
        );
        (buffer, tracing::subscriber::set_default(subscriber))
    }
}

#[cfg(test)]
mod tests;
