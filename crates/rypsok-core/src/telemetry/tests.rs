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
/// its reason, and counting it runs the flush check a call runs: once the
/// interval has passed, the next refusal logs the counters, with no call
/// in between.
#[test]
fn req_obs_007_http_refusals_are_counted_and_flush() {
    let counters = Counters::new();
    counters.refused_http("sdk_forbidden");
    counters.refused_http("sdk_forbidden");
    counters.refused_http("unauthorized");
    assert_eq!(counters.refusals_of("http", "sdk_forbidden"), 2);
    assert_eq!(counters.refusals_of("http", "unauthorized"), 1);
    assert_eq!(counters.refusals_of("http", "session_limit"), 0);

    // The flush: no event inside the interval, one after it, from the
    // refusal itself.
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;
    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    struct SinkWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for SinkWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for Sink {
        type Writer = SinkWriter;
        fn make_writer(&'a self) -> SinkWriter {
            SinkWriter(Arc::clone(&self.0))
        }
    }
    let sink = Sink::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_writer(sink.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let quick = Counters::with_flush_interval(std::time::Duration::from_millis(100));
        quick.refused_http("sdk_forbidden");
        quick.refused_http("sdk_forbidden");
        assert!(sink.0.lock().unwrap().is_empty(), "inside the interval");
        std::thread::sleep(std::time::Duration::from_millis(150));
        quick.refused_http("unauthorized");
        let log = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
        assert_eq!(log.lines().count(), 1, "{log}");
        assert!(log.contains("http.sdk_forbidden=2"), "{log}");
        assert!(log.contains("http.unauthorized=1"), "{log}");
    });
}
