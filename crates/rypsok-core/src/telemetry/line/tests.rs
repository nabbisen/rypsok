//! Tests of the log line (RFC 009; RFC 020, TST-2). Every test names the
//! requirement it verifies.

use super::super::capture::at_debug;

/// REQ-OBS-005: a line is one JSON object with `timestamp`, `level`, the
/// fields of the event and its `message`, as before this change; an event
/// outside a span has nothing else.
#[test]
fn req_obs_005_a_line_is_one_object_with_the_fields_of_its_event() {
    let (buffer, _guard) = at_debug();
    tracing::info!(
        event = "x",
        count = 3u64,
        ok = true,
        name = "a \"quoted\"\nname",
        "the message"
    );
    let lines = buffer.lines();
    assert_eq!(lines.len(), 1);
    let line = lines[0].as_object().unwrap();
    let mut keys: Vec<&str> = line.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "count",
            "event",
            "level",
            "message",
            "name",
            "ok",
            "timestamp"
        ]
    );
    assert_eq!(line["level"], "INFO");
    assert_eq!(line["event"], "x");
    assert_eq!(line["count"], 3);
    assert_eq!(line["ok"], true);
    assert_eq!(line["name"], "a \"quoted\"\nname");
    assert_eq!(line["message"], "the message");
    assert!(
        line["timestamp"].as_str().unwrap().ends_with('Z'),
        "{line:?}"
    );
    assert!(!line.contains_key("span") && !line.contains_key("spans"));
}

/// REQ-OBS-005 (RFC 020, TST-2): the fields of the spans an event is inside
/// of are top-level fields of its line, the outer span first; a name the
/// event already has is not overwritten by a span's.
#[test]
fn req_obs_005_span_fields_are_top_level_and_the_event_wins() {
    let (buffer, _guard) = at_debug();
    let outer = tracing::error_span!(
        "call",
        request_id = "r-1",
        caller = "alpha",
        tool = "web_search"
    );
    let _outer = outer.enter();
    let inner = tracing::error_span!("inner", step = 2u64, tool = "inner-tool");
    let _inner = inner.enter();
    tracing::info!(event = "e", tool = "event-tool", "inside");
    let lines = buffer.lines();
    let line = lines[0].as_object().unwrap();
    assert_eq!(line["request_id"], "r-1");
    assert_eq!(line["caller"], "alpha");
    assert_eq!(line["step"], 2);
    assert_eq!(line["tool"], "event-tool", "the event's own field wins");
    assert!(!line.contains_key("span"));
}

/// REQ-OBS-005 (RFC 020, TST-2): the span of a call exists at every log
/// level: with the level at `error` a refusal logged at `warn` is cut by
/// the filter, but an `error` event inside the call still has the id.
#[test]
fn req_obs_005_the_call_span_exists_at_the_strictest_level() {
    use tracing_subscriber::prelude::*;
    let buffer = super::super::capture::Buffer::default();
    let subscriber = tracing_subscriber::registry().with(
        super::layer(buffer.clone())
            .with_filter(super::super::filter(crate::config::LogLevel::Error)),
    );
    let _guard = tracing::subscriber::set_default(subscriber);
    let caller = crate::model::CallerId::new("alpha");
    let span = super::super::call_span("web_fetch", &caller);
    span.in_scope(|| {
        tracing::warn!(event = "refused", "cut by the level");
        tracing::error!(event = "boom", "kept");
    });
    let lines = buffer.lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["event"], "boom");
    assert_eq!(lines[0]["caller"], "alpha");
    assert_eq!(lines[0]["tool"], "web_fetch");
    let id = lines[0]["request_id"].as_str().unwrap();
    assert_eq!(id.len(), 18, "{id}");
}
