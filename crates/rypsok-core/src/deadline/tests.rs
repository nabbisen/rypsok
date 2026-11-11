//! Tests of the bounding function. Every test names the requirement it
//! verifies.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::time::Instant;

use super::*;

/// Sets its flag when dropped: proof that the bounded future was dropped.
struct Dropped(Arc<AtomicBool>);

impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// REQ-AGG-003 and REQ-PAGE-008 (RFC 021, D1): a future that never ends is
/// left at exactly the deadline, the error is the one `on_expiry` makes, and
/// the future is dropped (so what it started is cancelled).
#[tokio::test(start_paused = true)]
async fn req_agg_003_a_future_that_never_ends_is_left_at_the_deadline() {
    let flag = Arc::new(AtomicBool::new(false));
    let guard = Dropped(Arc::clone(&flag));
    let start = Instant::now();
    let result: Result<(), &str> = within(
        start + Duration::from_millis(2800),
        || "expired",
        async move {
            let _guard = guard;
            std::future::pending::<()>().await;
            Ok(())
        },
    )
    .await;
    assert_eq!(result, Err("expired"));
    assert_eq!(start.elapsed(), Duration::from_millis(2800), "exactly");
    assert!(flag.load(Ordering::SeqCst), "the future was dropped");
}

/// REQ-AGG-003 (RFC 021, D1): a call that ends in time returns its own
/// result, a value or its own error, and `on_expiry` is not run.
#[tokio::test(start_paused = true)]
async fn req_agg_003_a_call_that_ends_in_time_returns_its_own_result() {
    let start = Instant::now();
    let deadline = start + Duration::from_millis(1000);
    let ok: Result<u32, &str> = within(deadline, || panic!("not expired"), async {
        tokio::time::sleep(Duration::from_millis(400)).await;
        Ok(7)
    })
    .await;
    assert_eq!(ok, Ok(7));
    assert_eq!(start.elapsed(), Duration::from_millis(400));
    let failed: Result<u32, &str> =
        within(deadline, || panic!("not expired"), async { Err("its own") }).await;
    assert_eq!(failed, Err("its own"));
}

/// REQ-AGG-003 (RFC 021, D1): a deadline that has already passed ends a call
/// that waits, at once, and `on_expiry` runs once.
#[tokio::test(start_paused = true)]
async fn req_agg_003_a_deadline_in_the_past_ends_a_waiting_call_at_once() {
    let start = Instant::now();
    let mut runs = 0;
    let result: Result<(), u32> = within(
        start,
        || {
            runs += 1;
            runs
        },
        async {
            std::future::pending::<()>().await;
            Ok(())
        },
    )
    .await;
    assert_eq!(result, Err(1));
    assert_eq!(start.elapsed(), Duration::ZERO);
    assert_eq!(runs, 1);
}
