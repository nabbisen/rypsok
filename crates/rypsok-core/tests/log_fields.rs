//! The request id and the caller in the log (REQ-OBS-005; RFC 020, TST-2,
//! TST-15), read from the lines the process's own layer writes.
//!
//! This binary installs **one global subscriber** and runs all its scenarios
//! in **one** `#[test]`, one after the other, so no other test shares the
//! process and nothing races the subscriber. (A thread-scoped subscriber in a
//! process whose other tests run in parallel can have a span's callsite
//! cached as disabled while its events are enabled, which dropped the span
//! fields of a line on CI: correction R1 of the 020r review.) Do not add a
//! second `#[test]` to this file.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::Value;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::prelude::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rypsok_core::config::{Config, LogLevel};
use rypsok_core::engine::registry::{Engine, Registry};
use rypsok_core::engine::{
    Driver, EngineError, EngineErrorClass, EngineProperties, EngineRequest, EngineResponse,
    RawResult,
};
use rypsok_core::model::{CallError, CallerId, Category, EngineId, FetchRequest, SearchRequest};
use rypsok_core::{CancellationToken, Rypsok, telemetry};

/// The text the layer wrote, shared with the subscriber.
#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

struct BufferWriter(Arc<Mutex<Vec<u8>>>);

impl Write for BufferWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(bytes);
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
    /// The lines written since the last call, each parsed (a line that is
    /// not a JSON object fails the test).
    fn take(&self) -> Vec<Value> {
        let bytes = std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner));
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .map(|line| {
                serde_json::from_str::<Value>(line)
                    .unwrap_or_else(|e| panic!("not a JSON line ({e}): {line}"))
            })
            .collect()
    }
}

/// What a scripted engine does when asked.
#[derive(Debug, Clone, Copy)]
enum Script {
    Answer,
    Slow(Duration),
}

#[derive(Debug)]
struct Scripted {
    script: Script,
    properties: EngineProperties,
}

#[async_trait::async_trait]
impl Driver for Scripted {
    fn kind(&self) -> &'static str {
        "scripted"
    }

    fn properties(&self) -> &EngineProperties {
        &self.properties
    }

    async fn search(
        &self,
        _req: &EngineRequest,
        cancel: CancellationToken,
    ) -> Result<EngineResponse, EngineError> {
        // An event from inside the engine's task, as a driver writes one: it
        // is in the call's span only if the task carries the span with it.
        // (The target is the crate's own: the filter caps every other one.)
        tracing::debug!(
            target: "rypsok_core::scripted",
            event = "driver_ran",
            "the driver was asked"
        );
        if let Script::Slow(delay) = self.script {
            tokio::select! {
                () = cancel.cancelled() => {
                    return Err(EngineError::new(EngineErrorClass::RequestFailure));
                }
                () = tokio::time::sleep(delay) => {}
            }
        }
        Ok(EngineResponse::new(vec![
            RawResult::new("A page", "https://example.org/a", "About the page."),
            RawResult::new("Another", "https://example.org/b", "About another."),
        ]))
    }
}

fn properties() -> EngineProperties {
    EngineProperties::new(400, 6000, 2)
}

/// A service with the scripted engines, and, for fetches, the one `allow`
/// rule and port that reach the test server on loopback.
fn service(engines: &[(&str, Script)], fetch_port: u16) -> Rypsok {
    let built = engines
        .iter()
        .enumerate()
        .map(|(i, (id, script))| {
            Engine::new(
                EngineId::new(*id).unwrap(),
                vec![Category::General],
                10 + i as u32,
                Box::new(Scripted {
                    script: *script,
                    properties: properties(),
                }),
            )
        })
        .collect();
    let config = Config::parse(&format!(
        "config_version = 1\n[egress]\nallow = [\"127.0.0.1/32\"]\nallow_http = true\n[fetch]\nports = [80, 443, {fetch_port}]\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n"
    ))
    .unwrap();
    Rypsok::from_parts(config, Registry::from_engines(built)).unwrap()
}

/// The lines grouped by their `request_id`, in order; a line without one
/// fails.
fn by_request(lines: &[Value]) -> BTreeMap<String, Vec<&Value>> {
    let mut groups: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for line in lines {
        let id = line["request_id"]
            .as_str()
            .unwrap_or_else(|| panic!("a line without a request_id: {line}"));
        groups.entry(id.to_owned()).or_default().push(line);
    }
    groups
}

fn count(events: &[&Value], name: &str) -> usize {
    events.iter().filter(|e| e["event"] == name).count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn req_obs_005_every_event_of_a_call_has_its_request_id_and_caller() {
    let log = Buffer::default();
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(telemetry::layer(log.clone()).with_filter(telemetry::filter(LogLevel::Debug))),
    )
    .expect("the only test of this binary installs the only subscriber");

    let alpha = CallerId::new("alpha");
    let beta = CallerId::new("beta");

    // --- the filter keeps the call span at the strictest level -------------
    // The span is at `error`, which the filter admits at every configured
    // level; a `warn` line of a refusal therefore has the id.
    for level in [
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
        LogLevel::Debug,
    ] {
        assert!(
            telemetry::filter(level).would_enable("rypsok_core::telemetry", &tracing::Level::ERROR),
            "{level:?}"
        );
    }
    let span = telemetry::call_span("web_fetch", &alpha);
    assert_eq!(
        span.metadata().expect("an enabled span").level(),
        &tracing::Level::ERROR
    );
    drop(span);
    assert!(log.take().is_empty(), "a span writes no line");

    // --- a line is one object, the span fields on top ----------------------
    {
        let outer = tracing::error_span!(
            target: "rypsok_core::scripted",
            "call",
            request_id = "r-1",
            caller = "alpha",
            tool = "web_search"
        );
        let _outer = outer.enter();
        let inner = tracing::error_span!(
            target: "rypsok_core::scripted",
            "inner",
            step = 2u64,
            tool = "inner-tool"
        );
        let _inner = inner.enter();
        tracing::info!(
            target: "rypsok_core::scripted",
            event = "e",
            tool = "event-tool",
            name = "a \"quoted\"\nname",
            "inside"
        );
    }
    tracing::info!(
        target: "rypsok_core::scripted",
        event = "x",
        count = 3u64,
        ok = true,
        "outside"
    );
    let lines = log.take();
    assert_eq!(lines.len(), 2, "{lines:#?}");
    let inside = lines[0].as_object().unwrap();
    let mut keys: Vec<&str> = inside.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "caller",
            "event",
            "level",
            "message",
            "name",
            "request_id",
            "step",
            "timestamp",
            "tool"
        ]
    );
    assert_eq!(inside["request_id"], "r-1");
    assert_eq!(inside["caller"], "alpha");
    assert_eq!(inside["step"], 2);
    assert_eq!(inside["tool"], "event-tool", "the event's own field wins");
    assert_eq!(inside["name"], "a \"quoted\"\nname");
    assert!(inside["timestamp"].as_str().unwrap().ends_with('Z'));
    let outside = lines[1].as_object().unwrap();
    let mut keys: Vec<&str> = outside.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["count", "event", "level", "message", "ok", "timestamp"],
        "an event outside a call has no span fields and no `span` key"
    );

    // --- a search: every event, the engines' tasks included ----------------
    let search = service(&[("a", Script::Answer), ("b", Script::Answer)], 80);
    let first = search
        .search_as(&alpha, SearchRequest::new("rust"), CancellationToken::new())
        .await
        .unwrap();
    let second = search
        .search_as(&beta, SearchRequest::new("rust"), CancellationToken::new())
        .await
        .unwrap();
    let lines = log.take();
    let groups = by_request(&lines);
    assert_eq!(groups.len(), 2, "two calls, two ids: {groups:#?}");
    let mut callers = Vec::new();
    for (id, events) in &groups {
        assert_eq!(id.len(), 18, "{id}");
        assert_eq!(count(events, "call"), 1, "{id}: {events:#?}");
        assert_eq!(count(events, "engine_answered"), 2, "{id}: {events:#?}");
        assert_eq!(
            count(events, "driver_ran"),
            2,
            "{id}: the events of the engines' tasks are in the call: {events:#?}"
        );
        assert!(events.iter().all(|e| e["tool"] == "web_search"), "{id}");
        let caller = events[0]["caller"].clone();
        assert!(events.iter().all(|e| e["caller"] == caller), "{id}");
        callers.push(caller.as_str().unwrap().to_owned());
    }
    callers.sort();
    assert_eq!(callers, ["alpha", "beta"]);
    // The id is in no response.
    for response in [&first, &second] {
        assert!(
            !serde_json::to_string(response)
                .unwrap()
                .contains("request_id")
        );
    }

    // --- two searches in flight at once do not mix their ids ---------------
    let overlap = service(
        &[
            ("fast", Script::Answer),
            ("slow", Script::Slow(Duration::from_millis(200))),
        ],
        80,
    );
    let (one, two) = tokio::join!(
        overlap.search_as(&alpha, SearchRequest::new("rust"), CancellationToken::new()),
        async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            overlap
                .search_as(&beta, SearchRequest::new("rust"), CancellationToken::new())
                .await
        }
    );
    one.unwrap();
    two.unwrap();
    let lines = log.take();
    let groups = by_request(&lines);
    assert_eq!(groups.len(), 2, "{groups:#?}");
    let mut callers = Vec::new();
    for (id, events) in &groups {
        assert_eq!(count(events, "call"), 1, "{id}: {events:#?}");
        assert_eq!(count(events, "engine_answered"), 2, "{id}: {events:#?}");
        assert_eq!(count(events, "driver_ran"), 2, "{id}: {events:#?}");
        let caller = events[0]["caller"].clone();
        assert!(events.iter().all(|e| e["caller"] == caller), "{id}");
        callers.push(caller.as_str().unwrap().to_owned());
    }
    callers.sort();
    assert_eq!(
        callers,
        ["alpha", "beta"],
        "the two ids belong to the two callers"
    );

    // --- a refused search has the id too -----------------------------------
    // (The token-shaped query is built at run time.)
    let secret = ["ghp_", &"A1b2C3d4E5f6G7h8I9j0".repeat(2)].concat();
    let refused = search
        .search_as(
            &CallerId::local(),
            SearchRequest::new(format!("find {secret}")),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(matches!(refused, CallError::Tool(_)));
    let lines = log.take();
    assert!(!lines.is_empty());
    assert_eq!(by_request(&lines).len(), 1, "{lines:#?}");
    assert!(lines.iter().all(|l| l["caller"] == "local"), "{lines:#?}");
    assert!(lines.iter().any(|l| l["event"] == "refused"), "{lines:#?}");

    // --- fetches: a page, a refusal and an error status --------------------
    let server = MockServer::start().await;
    let port: u16 = server.uri().rsplit(':').next().unwrap().parse().unwrap();
    Mock::given(method("GET"))
        .and(path("/article"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            b"<html><head><title>Article</title></head><body><main><p>Fetched content that is long enough to extract as the main part.</p></main></body></html>".to_vec(),
            "text/html",
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let fetcher = service(&[("a", Script::Answer)], port);
    fn request(url: impl Into<String>) -> FetchRequest {
        FetchRequest::new(url)
    }
    fetcher
        .fetch_as(
            &alpha,
            request(format!("{}/article", server.uri())),
            CancellationToken::new(),
        )
        .await
        .unwrap();
    fetcher
        .fetch_as(
            &alpha,
            request("ftp://127.0.0.1/x"),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    fetcher
        .fetch_as(
            &alpha,
            request(format!("{}/gone", server.uri())),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    let lines = log.take();
    let groups = by_request(&lines);
    assert_eq!(groups.len(), 3, "{lines:#?}");
    for (id, events) in &groups {
        assert_eq!(id.len(), 18, "{id}");
        assert_eq!(count(events, "call"), 1, "{id}: {events:#?}");
        assert!(events.iter().all(|e| e["caller"] == "alpha"), "{id}");
        assert!(events.iter().all(|e| e["tool"] == "web_fetch"), "{id}");
    }

    // --- engine_status logs a `call` event like the other tools ------------
    fetcher.status_as(&beta);
    let lines = log.take();
    assert_eq!(lines.len(), 1, "{lines:#?}");
    assert_eq!(lines[0]["event"], "call");
    assert_eq!(lines[0]["tool"], "engine_status");
    assert_eq!(lines[0]["caller"], "beta");
    assert_eq!(lines[0]["outcome"], "ok");
    assert!(lines[0]["latency_ms"].is_u64(), "{:?}", lines[0]);
    assert_eq!(lines[0]["request_id"].as_str().unwrap().len(), 18);
}
