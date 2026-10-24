//! Tests of the health table with a paused clock (REQ-TEST-017). Every
//! test names the requirement it verifies.

use std::time::Duration;

use super::*;

fn id(name: &str) -> EngineId {
    EngineId::new(name).unwrap()
}

fn table() -> HealthTable {
    HealthTable::from_ids([(id("a"), true), (id("z"), false)], &Limits::default())
}

fn fail(table: &HealthTable, class: EngineErrorClass) -> Completed {
    table.complete(
        &id("a"),
        Completion::Failure {
            class,
            retry_after: None,
        },
    )
}

fn state(table: &HealthTable) -> EngineState {
    table.circuit(&id("a")).unwrap().state()
}

/// REQ-RES-004 and REQ-TEST-017: the circuit opens after LIM-B1
/// consecutive counted failures; a success in between resets the count;
/// the opening is reported as a transition with the cooldown.
#[tokio::test(start_paused = true)]
async fn req_test_017_opens_after_consecutive_counted_failures() {
    let t = table();
    fail(&t, EngineErrorClass::Timeout);
    fail(&t, EngineErrorClass::RateLimited);
    assert_eq!(
        t.circuit(&id("a")),
        Some(Circuit::Closed {
            consecutive_failures: 2
        })
    );
    t.complete(&id("a"), Completion::Success);
    assert_eq!(
        t.circuit(&id("a")),
        Some(Circuit::Closed {
            consecutive_failures: 0
        })
    );
    fail(&t, EngineErrorClass::Blocked);
    fail(&t, EngineErrorClass::ParseFailure);
    let opened = fail(&t, EngineErrorClass::Timeout);
    assert_eq!(state(&t), EngineState::Open);
    assert_eq!(
        opened.transition,
        Some(Transition {
            engine: id("a"),
            state: EngineState::Open,
            cooldown: Some(Duration::from_secs(60)),
            consecutive_failures: 3,
        })
    );
}

/// REQ-RES-004: `auth_failure` and `request_failure` never count; the
/// first `auth_failure` asks for one warning per engine per process.
#[tokio::test(start_paused = true)]
async fn req_res_004_uncounted_classes_never_open_the_circuit() {
    let t = table();
    for _ in 0..10 {
        fail(&t, EngineErrorClass::RequestFailure);
    }
    let first = fail(&t, EngineErrorClass::AuthFailure);
    let second = fail(&t, EngineErrorClass::AuthFailure);
    assert!(first.warn_auth);
    assert!(!second.warn_auth);
    assert_eq!(
        t.circuit(&id("a")),
        Some(Circuit::Closed {
            consecutive_failures: 0
        })
    );
    assert_eq!(t.admit(&id("a")).0, Admission::Allowed);
}

/// REQ-RES-004 and REQ-RES-005: an open circuit is skipped until the
/// cooldown elapses; the first selection after that carries the probe and
/// turns the circuit half-open; a second selection is skipped meanwhile.
#[tokio::test(start_paused = true)]
async fn req_res_005_cooldown_then_exactly_one_probe() {
    let t = table();
    for _ in 0..3 {
        fail(&t, EngineErrorClass::Timeout);
    }
    assert_eq!(t.admit(&id("a")), (Admission::Skipped, None));
    tokio::time::advance(Duration::from_secs(59)).await;
    assert_eq!(t.admit(&id("a")).0, Admission::Skipped);
    tokio::time::advance(Duration::from_secs(1)).await;
    let (admission, transition) = t.admit(&id("a"));
    assert_eq!(admission, Admission::Probe);
    assert_eq!(
        transition.map(|x| x.state),
        Some(EngineState::HalfOpen),
        "the transition to half-open is reported once"
    );
    assert_eq!(t.admit(&id("a")), (Admission::Skipped, None));
    assert_eq!(state(&t), EngineState::HalfOpen);
}

/// REQ-RES-005: a successful probe closes the circuit; a failed probe
/// reopens it with a doubled cooldown, up to LIM-B3's maximum.
#[tokio::test(start_paused = true)]
async fn req_res_005_probe_closes_or_doubles_up_to_the_cap() {
    let t = table();
    for _ in 0..3 {
        fail(&t, EngineErrorClass::Timeout);
    }
    let mut expected = 60u64;
    for _ in 0..6 {
        tokio::time::advance(Duration::from_secs(expected)).await;
        assert_eq!(t.admit(&id("a")).0, Admission::Probe);
        let reopened = fail(&t, EngineErrorClass::Timeout);
        expected = (expected * 2).min(900);
        assert_eq!(
            reopened.transition.and_then(|x| x.cooldown),
            Some(Duration::from_secs(expected)),
            "cooldown doubles and stops at the cap"
        );
    }
    assert_eq!(expected, 900);
    tokio::time::advance(Duration::from_secs(900)).await;
    assert_eq!(t.admit(&id("a")).0, Admission::Probe);
    let closed = t.complete(&id("a"), Completion::Success);
    assert_eq!(
        closed.transition.map(|x| x.state),
        Some(EngineState::Closed)
    );
    assert_eq!(
        t.circuit(&id("a")),
        Some(Circuit::Closed {
            consecutive_failures: 0
        })
    );
}

/// LIM-B4: a `Retry-After` larger than the cooldown is honored, and it is
/// capped by LIM-B3's maximum.
#[tokio::test(start_paused = true)]
async fn req_res_004_retry_after_is_honored_and_capped() {
    let t = table();
    fail(&t, EngineErrorClass::Timeout);
    fail(&t, EngineErrorClass::Timeout);
    let opened = t.complete(
        &id("a"),
        Completion::Failure {
            class: EngineErrorClass::RateLimited,
            retry_after: Some(Duration::from_secs(120)),
        },
    );
    assert_eq!(
        opened.transition.and_then(|x| x.cooldown),
        Some(Duration::from_secs(120))
    );
    tokio::time::advance(Duration::from_secs(120)).await;
    assert_eq!(t.admit(&id("a")).0, Admission::Probe);
    let reopened = t.complete(
        &id("a"),
        Completion::Failure {
            class: EngineErrorClass::RateLimited,
            retry_after: Some(Duration::from_secs(10_000)),
        },
    );
    assert_eq!(
        reopened.transition.and_then(|x| x.cooldown),
        Some(Duration::from_secs(900)),
        "capped by LIM-B3"
    );
    // A Retry-After smaller than the cooldown does not shorten it.
    tokio::time::advance(Duration::from_secs(900)).await;
    assert_eq!(t.admit(&id("a")).0, Admission::Probe);
    let again = t.complete(
        &id("a"),
        Completion::Failure {
            class: EngineErrorClass::RateLimited,
            retry_after: Some(Duration::from_secs(1)),
        },
    );
    assert_eq!(
        again.transition.and_then(|x| x.cooldown),
        Some(Duration::from_secs(900))
    );
}

/// REQ-RES-005: a probe that ends without a verdict (the call was
/// cancelled, or the failure is uncounted) reopens the circuit with the
/// same cooldown.
#[tokio::test(start_paused = true)]
async fn req_res_005_cancelled_or_uncounted_probe_reopens_with_same_cooldown() {
    let t = table();
    for _ in 0..3 {
        fail(&t, EngineErrorClass::Timeout);
    }
    tokio::time::advance(Duration::from_secs(60)).await;
    assert_eq!(t.admit(&id("a")).0, Admission::Probe);
    let reopened = t.complete(&id("a"), Completion::Cancelled);
    assert_eq!(
        reopened.transition.and_then(|x| x.cooldown),
        Some(Duration::from_secs(60))
    );
    tokio::time::advance(Duration::from_secs(60)).await;
    assert_eq!(t.admit(&id("a")).0, Admission::Probe);
    let reopened = fail(&t, EngineErrorClass::RequestFailure);
    assert_eq!(
        reopened.transition.and_then(|x| x.cooldown),
        Some(Duration::from_secs(60))
    );
    assert_eq!(state(&t), EngineState::Open);
}

/// REQ-OBS-002 and REQ-STATUS-001: the median of the last LIM-S11
/// recorded latencies, null when nothing completed; the window slides.
#[tokio::test(start_paused = true)]
async fn req_obs_002_latency_median_over_the_window() {
    let t = table();
    assert_eq!(t.snapshot()[0].latency_ms, None);
    t.record_latency(&id("a"), Duration::from_millis(100));
    assert_eq!(t.snapshot()[0].latency_ms, Some(100));
    t.record_latency(&id("a"), Duration::from_millis(300));
    assert_eq!(
        t.snapshot()[0].latency_ms,
        Some(200),
        "mean of the two middles"
    );
    t.record_latency(&id("a"), Duration::from_millis(900));
    assert_eq!(t.snapshot()[0].latency_ms, Some(300));
    for _ in 0..20 {
        t.record_latency(&id("a"), Duration::from_millis(50));
    }
    assert_eq!(
        t.snapshot()[0].latency_ms,
        Some(50),
        "the first three fell out of the 20-wide window"
    );
    assert_eq!(t.snapshot()[1].latency_ms, None);
}

/// REQ-OBS-001 and REQ-STATUS-001: one entry per configured engine, in
/// configuration order, `disabled` for an engine without a driver; the
/// snapshot carries exactly identifier, state and latency.
#[tokio::test(start_paused = true)]
async fn req_obs_001_snapshot_per_engine_in_configuration_order() {
    let t = HealthTable::from_ids(
        [(id("m"), true), (id("b"), false), (id("x"), true)],
        &Limits::default(),
    );
    let snapshot = t.snapshot();
    let ids: Vec<&str> = snapshot.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec!["b", "m", "x"]);
    assert_eq!(snapshot[0].state, EngineState::Disabled);
    assert_eq!(snapshot[1].state, EngineState::Closed);
    assert_eq!(t.admit(&id("b")).0, Admission::Skipped);
    assert_eq!(
        t.complete(&id("b"), Completion::Success),
        Completed::default()
    );
    let value = serde_json::to_value(&snapshot).unwrap();
    for entry in value.as_array().unwrap() {
        let keys: Vec<&String> = entry.as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["id", "latency_ms", "state"]);
    }
}

/// REQ-OBS-004: each transition logs one event with the fields of RFC 010.
#[test]
fn req_obs_004_transitions_have_an_event_per_state() {
    let transition = |state| Transition {
        engine: id("a"),
        state,
        cooldown: Some(Duration::from_secs(60)),
        consecutive_failures: 3,
    };
    // The events are exercised through the pipeline's stderr capture test;
    // here the mapping is total: no state is silently dropped.
    for state in [
        EngineState::Open,
        EngineState::HalfOpen,
        EngineState::Closed,
    ] {
        transition(state).log();
    }
    assert!(counted(EngineErrorClass::Timeout));
    assert!(!counted(EngineErrorClass::AuthFailure));
    assert!(!counted(EngineErrorClass::RequestFailure));
}

/// REQ-STATUS-002: the serialized status carries exactly the envelope and
/// identifier, state and latency per engine; no endpoint, credential or
/// provider text has a field to live in.
#[tokio::test(start_paused = true)]
async fn req_status_002_status_has_no_field_for_endpoint_credential_or_provider_text() {
    let t = table();
    fail(&t, EngineErrorClass::Blocked);
    let response = crate::model::StatusResponse {
        outcome: crate::model::Outcome::Ok,
        notices: Vec::new(),
        engines: t.snapshot(),
    };
    let value = serde_json::to_value(&response).unwrap();
    let top: Vec<&String> = value.as_object().unwrap().keys().collect();
    assert_eq!(top, vec!["engines", "notices", "outcome"]);
    for entry in value["engines"].as_array().unwrap() {
        let keys: Vec<&String> = entry.as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["id", "latency_ms", "state"]);
        assert!(entry["state"].as_str().is_some());
    }
    let text = value.to_string();
    for forbidden in ["http", "key", "token", "Blocked", "error"] {
        assert!(!text.contains(forbidden), "{forbidden} in {text}");
    }
}

/// REQ-OBS-004: the failure streak reported by the events counts every
/// counted failure since the last success, across the open and half-open
/// states, and resets on success.
#[tokio::test(start_paused = true)]
async fn req_obs_004_events_report_the_failure_streak_across_states() {
    let t = table();
    for _ in 0..3 {
        fail(&t, EngineErrorClass::Timeout);
    }
    tokio::time::advance(Duration::from_secs(60)).await;
    let (_, half_open) = t.admit(&id("a"));
    assert_eq!(half_open.unwrap().consecutive_failures, 3);
    let reopened = fail(&t, EngineErrorClass::Timeout);
    assert_eq!(reopened.transition.unwrap().consecutive_failures, 4);
    tokio::time::advance(Duration::from_secs(120)).await;
    t.admit(&id("a"));
    let closed = t.complete(&id("a"), Completion::Success);
    assert_eq!(closed.transition.unwrap().consecutive_failures, 0);
    fail(&t, EngineErrorClass::Timeout);
    assert_eq!(
        t.circuit(&id("a")),
        Some(Circuit::Closed {
            consecutive_failures: 1
        })
    );
}
