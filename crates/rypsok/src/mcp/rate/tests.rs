//! Tests of the per-caller invocation limiters.

use std::sync::Arc;

use rypsok_core::limits::Limits;
use rypsok_core::model::CallerId;

use super::CallerLimiters;

/// REQ-SEC-014 (handoff 020j, NET-13): a panic while the callers' lock is
/// held does not make every later call panic: the limiters of a caller seen
/// before the panic are still found, and a new caller gets its own.
#[test]
fn req_sec_014_poisoned_caller_lock_does_not_stop_the_limiters() {
    let limiters = Arc::new(CallerLimiters::new(&Limits::default()));
    let before = limiters.for_caller(&CallerId::local());

    let shared = Arc::clone(&limiters);
    let outcome = std::thread::spawn(move || {
        let _guard = shared.callers.lock().unwrap();
        panic!("poisoning the lock on purpose");
    })
    .join();
    assert!(outcome.is_err());
    assert!(limiters.callers.is_poisoned());

    let after = limiters.for_caller(&CallerId::local());
    assert!(Arc::ptr_eq(&before, &after));
    let other = limiters.for_caller(&CallerId::new("mimlys"));
    assert!(other.search.admit().is_ok());
}
