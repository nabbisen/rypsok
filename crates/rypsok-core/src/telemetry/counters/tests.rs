//! Tests of the counters that need their private fields.

use std::sync::Arc;
use std::time::Duration;

use super::Counters;

/// Panics in a thread that holds both locks of `counters`, which poisons
/// them.
fn poison(counters: &Arc<Counters>) {
    let shared = Arc::clone(counters);
    let outcome = std::thread::spawn(move || {
        let _totals = shared.totals.lock().unwrap();
        let _last = shared.last_flush.lock().unwrap();
        panic!("poisoning the locks on purpose");
    })
    .join();
    assert!(outcome.is_err());
    assert!(counters.totals.is_poisoned());
    assert!(counters.last_flush.is_poisoned());
}

/// REQ-SEC-014 (handoff 020j, NET-13): a panic while the counters' locks
/// are held does not make every later call panic: a call is recorded, read
/// back and logged, and the counts from before the panic are kept.
#[test]
fn req_sec_014_poisoned_counter_locks_do_not_stop_the_service() {
    let counters = Arc::new(Counters::new());
    counters.call("web_search", "ok");
    poison(&counters);

    counters.call("web_search", "ok");
    counters.refused("web_fetch", "denied");
    counters.engine_error("brave", "timeout");
    assert_eq!(counters.calls_of("web_search", "ok"), 2);
    assert_eq!(counters.refusals_of("web_fetch", "denied"), 1);
    assert_eq!(counters.engine_errors_of("brave", "timeout"), 1);
    counters.log();
}

/// REQ-SEC-014: the flush check, which takes the second lock, also survives
/// a poisoned state (an interval of zero means every call flushes).
#[test]
fn req_sec_014_the_flush_check_survives_poison() {
    let counters = Arc::new(Counters::with_flush_interval(Duration::ZERO));
    poison(&counters);
    counters.call("web_search", "ok");
    counters.refused_http("unauthorized");
    assert_eq!(counters.calls_of("web_search", "ok"), 1);
}
