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

/// How a library test reads the log without a subscriber of its own: the
/// process has **one** global subscriber, installed once, whose writer hands
/// each line to the sink of the thread that wrote it. A thread-scoped
/// subscriber per test can miss an event (the callsite's interest is cached
/// for the process); one dispatcher cannot. A test that uses it runs on one
/// thread (a plain `#[test]`, or a current-thread runtime).
#[cfg(test)]
pub(crate) mod routed {
    use std::cell::RefCell;
    use std::io::Write;
    use std::sync::{Arc, Mutex, Once, PoisonError};

    use serde_json::Value;
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::prelude::*;

    type Sink = Arc<Mutex<Vec<u8>>>;

    thread_local! {
        static SINK: RefCell<Option<Sink>> = const { RefCell::new(None) };
    }

    struct Routed;

    struct RoutedWriter(Option<Sink>);

    impl Write for RoutedWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if let Some(sink) = &self.0 {
                sink.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .extend_from_slice(bytes);
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Routed {
        type Writer = RoutedWriter;

        fn make_writer(&'a self) -> RoutedWriter {
            RoutedWriter(SINK.with(|sink| sink.borrow().clone()))
        }
    }

    /// The lines this thread has written since the capture began.
    pub struct Capture(Sink);

    impl Capture {
        pub fn lines(&self) -> Vec<Value> {
            let text = String::from_utf8(
                self.0
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone(),
            )
            .unwrap();
            text.lines()
                .map(|line| {
                    serde_json::from_str::<Value>(line)
                        .unwrap_or_else(|e| panic!("not a JSON line ({e}): {line}"))
                })
                .collect()
        }
    }

    impl Drop for Capture {
        fn drop(&mut self) {
            SINK.with(|sink| *sink.borrow_mut() = None);
        }
    }

    /// Begins capturing this thread's log at `warn` and above.
    pub fn capture() -> Capture {
        static INSTALL: Once = Once::new();
        INSTALL.call_once(|| {
            tracing::subscriber::set_global_default(tracing_subscriber::registry().with(
                super::layer(Routed).with_filter(super::filter(crate::config::LogLevel::Warn)),
            ))
            .expect("no other test of this process installs a global subscriber");
        });
        let sink = Arc::new(Mutex::new(Vec::new()));
        SINK.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&sink)));
        Capture(sink)
    }
}

#[cfg(test)]
mod tests;
