//! The events the core writes to the log (REQ-OBS-004, REQ-OBS-007,
//! REQ-SEC-007), read from the lines the process's own layer writes.
//!
//! Like `log_fields.rs`, this binary installs **one global subscriber** and
//! runs all its scenarios in **one** `#[test]`, one after the other, so no
//! other test shares the process. A thread-scoped subscriber in a process
//! whose other tests run in parallel can miss an event: `tracing` caches
//! whether a callsite is enabled for the whole process, and a callsite that
//! another thread reached without a subscriber can be cached as disabled
//! (correction R3 of the 020r review). Do not add a second `#[test]`.

use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde_json::Value;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::prelude::*;

use rypsok_core::config::{Config, LogLevel};
use rypsok_core::egress::policy::EgressPolicy;
use rypsok_core::egress::resolver::Lookup;
use rypsok_core::egress::{AgentClientSettings, ReturnedUrls};
use rypsok_core::engine::registry::{Engine, Registry};
use rypsok_core::engine::{
    Driver, EngineError, EngineErrorClass, EngineProperties, EngineRequest, EngineResponse,
    RawResult,
};
use rypsok_core::fetch::{self, FetchService};
use rypsok_core::limits::Limits;
use rypsok_core::model::{CallError, CallerId, Category, EngineId, ErrorKind, FetchRequest};
use rypsok_core::model::{DetailReason, SearchRequest};
use rypsok_core::telemetry::Counters;
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
    /// The text written since the last call.
    fn take_text(&self) -> String {
        let bytes = std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner));
        String::from_utf8(bytes).unwrap()
    }
}

/// An engine that is blocked for its first `blocked` calls and answers after.
#[derive(Debug)]
struct Blocking {
    blocked: usize,
    calls: std::sync::atomic::AtomicUsize,
    properties: EngineProperties,
}

#[async_trait::async_trait]
impl Driver for Blocking {
    fn kind(&self) -> &'static str {
        "scripted"
    }

    fn properties(&self) -> &EngineProperties {
        &self.properties
    }

    async fn search(
        &self,
        _req: &EngineRequest,
        _cancel: CancellationToken,
    ) -> Result<EngineResponse, EngineError> {
        let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        if call <= self.blocked {
            return Err(EngineError::new(EngineErrorClass::Blocked));
        }
        Ok(EngineResponse::new(vec![RawResult::new(
            "A page",
            "https://example.org/a",
            "About the page.",
        )]))
    }
}

/// A scripted name lookup: `private.invalid` resolves to a private address,
/// `missing.invalid` to nothing, `gone.invalid` fails.
struct ByName;

impl Lookup for ByName {
    fn lookup(
        &self,
        name: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<Vec<std::net::IpAddr>>> + Send>,
    > {
        Box::pin(async move {
            match name.as_str() {
                "private.invalid" => Ok(vec!["10.0.0.1".parse().unwrap()]),
                "missing.invalid" => Ok(Vec::new()),
                _ => Err(std::io::Error::other("no such name")),
            }
        })
    }
}

#[tokio::test(flavor = "current_thread")]
async fn req_obs_004_req_obs_007_req_sec_007_core_events_reach_the_log() {
    let log = Buffer::default();
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(telemetry::layer(log.clone()).with_filter(telemetry::filter(LogLevel::Debug))),
    )
    .expect("the only test of this binary installs the only subscriber");

    // --- REQ-SEC-007 and REQ-FETCH-006 (RFC 019 D2) --------------------------
    // The operator's log says, once and without the host, that a name was
    // refused at connect time; a name that cannot be reached leaves no
    // `refused` event.
    let limits = Limits::default();
    let policy = Arc::new(EgressPolicy::new(
        Vec::new(),
        Vec::new(),
        true,
        vec![80, 443],
    ));
    let settings = AgentClientSettings {
        user_agent: "rypsok/test (+https://example.org)".into(),
        connect_timeout: Duration::from_millis(limits.fetch_connect_timeout_ms.into()),
        timeout: Duration::from_millis(limits.fetch_deadline_ms.into()),
        allow_http: true,
    };
    let service =
        FetchService::with_lookup(&settings, Arc::clone(&policy), &limits, ByName).unwrap();
    let returned = ReturnedUrls::new(100);
    let counters = Counters::new();
    let caller = CallerId::local();
    for robots_txt in [true, false] {
        let pipeline = fetch::Pipeline {
            limits: &limits,
            policy: &policy,
            service: &service,
            returned: &returned,
            strict_mode: false,
            robots_txt,
            follow_cross_host_redirects: false,
            secret_detection: rypsok_core::config::SecretDetection::Refuse,
            counters: &counters,
            caller: &caller,
        };
        for url in [
            "https://private.invalid/a",
            "https://missing.invalid/a",
            "https://gone.invalid/a",
        ] {
            for round in 1..=2 {
                log.take_text();
                let error =
                    match fetch::run(&pipeline, FetchRequest::new(url), CancellationToken::new())
                        .await
                    {
                        Err(CallError::Tool(error)) => error,
                        other => panic!("{url}: {other:?}"),
                    };
                assert_eq!(error.kind, ErrorKind::Unavailable, "{url} {round}");
                assert_eq!(error.detail.reason, Some(DetailReason::Network));
                let text = log.take_text();
                let refusals = text
                    .lines()
                    .filter(|l| l.contains("\"event\":\"refused\""))
                    .count();
                let expected = usize::from(url.contains("private"));
                assert_eq!(
                    refusals, expected,
                    "{url}, robots {robots_txt}, call {round}: refused events\n{text}"
                );
                if expected == 1 {
                    assert!(text.contains("\"denial\":\"address\""), "{text}");
                }
                assert!(!text.contains(".invalid"), "no host in the log:\n{text}");
            }
        }
    }

    // --- REQ-OBS-007 (RFC 019 D6): the counters' flush -----------------------
    // An hour has not passed: refusals are counted and nothing is logged.
    log.take_text();
    let slow = Counters::with_flush_interval(Duration::from_secs(3600));
    slow.refused_http("sdk_forbidden");
    slow.refused_http("sdk_forbidden");
    assert_eq!(log.take_text(), "", "inside the interval");
    assert_eq!(slow.refusals_of("http", "sdk_forbidden"), 2);
    // The interval has always passed: each refusal runs the flush check and
    // logs the totals so far, one line per refusal, with no call in between.
    // The third line carries all three counts.
    let due = Counters::with_flush_interval(Duration::ZERO);
    due.refused_http("sdk_forbidden");
    due.refused_http("sdk_forbidden");
    due.refused_http("unauthorized");
    let text = log.take_text();
    assert_eq!(text.lines().count(), 3, "{text}");
    let last = text.lines().last().unwrap();
    assert!(last.contains("http.sdk_forbidden=2"), "{last}");
    assert!(last.contains("http.unauthorized=1"), "{last}");

    // --- REQ-OBS-004: the three circuit events -------------------------------
    // (Last, because time stops here: the cooldown of a minute is one call.)
    let engine = Engine::new(
        EngineId::new("a").unwrap(),
        vec![Category::General],
        10,
        Box::new(Blocking {
            blocked: 3,
            calls: std::sync::atomic::AtomicUsize::new(0),
            properties: EngineProperties::new(400, 6000, 2),
        }),
    );
    let config = Config::parse(
        "config_version = 1\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n",
    )
    .unwrap();
    let service = Rypsok::from_parts(config, Registry::from_engines(vec![engine])).unwrap();
    tokio::time::pause();
    log.take_text();
    for _ in 0..3 {
        let _ = service
            .search(SearchRequest::new("rust"), CancellationToken::new())
            .await;
    }
    tokio::time::advance(Duration::from_secs(60)).await;
    service
        .search(SearchRequest::new("rust"), CancellationToken::new())
        .await
        .unwrap();
    let text = log.take_text();
    let events: Vec<Value> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .filter(|v: &Value| {
            v["event"]
                .as_str()
                .is_some_and(|e| e.starts_with("circuit_"))
        })
        .collect();
    let names: Vec<&str> = events
        .iter()
        .map(|e| e["event"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["circuit_opened", "circuit_half_open", "circuit_closed"],
        "{text}"
    );
    assert_eq!(events[0]["engine"], "a");
    assert_eq!(events[0]["state"], "open");
    assert_eq!(events[0]["cooldown_ms"], 60_000);
    assert_eq!(events[0]["consecutive_failures"], 3);
    assert_eq!(events[1]["state"], "half_open");
    assert_eq!(events[2]["state"], "closed");
    assert!(!text.contains("rust"), "the query is not in the log");
}
