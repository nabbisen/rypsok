//! Tests of the `telemetry` module. Every test names the requirement it verifies.

use super::*;

/// REQ-OBS-003 and REQ-OBS-007: every counter path increments and is
/// readable by tool, outcome, reason, engine and class.
#[test]
fn req_obs_007_counters_increment_per_path() {
    let counters = Counters::new();
    counters.call("web_search", "ok");
    counters.call("web_search", "ok");
    counters.call("web_search", "invalid_request");
    counters.refused("web_search", "secret_in_query");
    counters.engine_error("brave", "rate_limited");
    counters.characters_removed(3);
    counters.results_dropped(1);
    assert_eq!(counters.calls_of("web_search", "ok"), 2);
    assert_eq!(counters.calls_of("web_search", "invalid_request"), 1);
    assert_eq!(counters.refusals_of("web_search", "secret_in_query"), 1);
    assert_eq!(counters.engine_errors_of("brave", "rate_limited"), 1);
    assert_eq!(counters.engine_errors_of("brave", "timeout"), 0);
    counters.log();
}

/// REQ-OBS-005: request identifiers are unique within the process.
#[test]
fn req_obs_005_request_ids_are_unique() {
    let ids: std::collections::BTreeSet<String> = (0..1000).map(|_| request_id()).collect();
    assert_eq!(ids.len(), 1000);
    assert!(
        ids.iter()
            .all(|id| id.len() == 18 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
    );
}

/// REQ-PRIV-004: at `debug` level the events of a dependency stay capped
/// at `warn`; only the crates of rypsok follow the configured level.
#[test]
fn req_priv_004_foreign_targets_are_capped_whatever_the_level() {
    use tracing_subscriber::filter::LevelFilter;
    let debug = filter(LogLevel::Debug);
    assert!(debug.would_enable("rypsok_core::search", &tracing::Level::DEBUG));
    assert!(debug.would_enable("rypsok::mcp", &tracing::Level::DEBUG));
    assert!(!debug.would_enable("rmcp::service", &tracing::Level::DEBUG));
    assert!(!debug.would_enable("rmcp::service", &tracing::Level::INFO));
    assert!(debug.would_enable("rmcp::service", &tracing::Level::WARN));
    assert!(!debug.would_enable("hyper_util::client", &tracing::Level::DEBUG));
    let error = filter(LogLevel::Error);
    assert!(!error.would_enable("rypsok_core::search", &tracing::Level::WARN));
    assert!(!error.would_enable("rmcp::service", &tracing::Level::WARN));
    assert_eq!(FOREIGN_CAP, LevelFilter::WARN);
}

/// REQ-OBS-007 (RFC 019 D6): an HTTP refusal is counted under `http` and
/// its reason. (That counting it runs the flush check a call runs, so that
/// the next refusal after the interval logs the counters with no call in
/// between, is read from the log in `tests/log_events.rs`.)
#[test]
fn req_obs_007_http_refusals_are_counted() {
    let counters = Counters::new();
    counters.refused_http("sdk_forbidden");
    counters.refused_http("sdk_forbidden");
    counters.refused_http("unauthorized");
    assert_eq!(counters.refusals_of("http", "sdk_forbidden"), 2);
    assert_eq!(counters.refusals_of("http", "unauthorized"), 1);
    assert_eq!(counters.refusals_of("http", "session_limit"), 0);
}

/// How a library test reads the log without a subscriber of its own: the
/// process has **one** global subscriber, installed once, whose writer hands
/// each line to the sink of the thread that wrote it. A thread-scoped
/// subscriber per test can miss an event (the callsite's interest is cached
/// for the process); one dispatcher cannot. A test that uses it runs on one
/// thread (a plain `#[test]`, or a current-thread runtime).
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
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry().with(
                    crate::telemetry::layer(Routed)
                        .with_filter(crate::telemetry::filter(crate::config::LogLevel::Warn)),
                ),
            )
            .expect("no other test of this process installs a global subscriber");
        });
        let sink = Arc::new(Mutex::new(Vec::new()));
        SINK.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&sink)));
        Capture(sink)
    }
}

/// REQ-OBS-005 and REQ-PRIV-004 (RFC 020, F-1 of the 020q review): the line a
/// panic leaves is one JSON object with the keys of every log line, the
/// location and the thread; it is a function of those and of the time only,
/// so the panic's message cannot be in it; awkward names stay one line.
#[test]
fn req_obs_005_the_panic_line_is_one_object_of_its_inputs_only() {
    let line = panic_line(
        "2026-11-07T10:00:00.000000Z",
        "worker-3",
        "src/fetch.rs",
        420,
    );
    assert!(!line.contains('\n'));
    let value: serde_json::Value = serde_json::from_str(&line).unwrap();
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "event",
            "level",
            "location",
            "message",
            "thread",
            "timestamp"
        ]
    );
    assert_eq!(value["level"], "ERROR");
    assert_eq!(value["event"], "panic");
    assert_eq!(value["location"], "src/fetch.rs:420");
    assert_eq!(value["thread"], "worker-3");
    assert_eq!(value["timestamp"], "2026-11-07T10:00:00.000000Z");
    // A thread name or path with quotes and line breaks is still one line.
    let awkward = panic_line("t", "a\"b\nc", "dir\\x\n.rs", 1);
    assert!(!awkward.contains('\n'), "{awkward}");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&awkward).unwrap()["thread"],
        "a\"b\nc"
    );
}
