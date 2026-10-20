//! Tests of the engine limiter. Every test names the requirement it verifies.

use std::time::{Duration, Instant};

use super::*;

/// REQ-CMP-002: the in-flight cap holds; a request beyond it is not
/// admitted before the deadline and is admitted once a permit returns.
#[tokio::test]
async fn req_cmp_002_in_flight_cap_is_enforced() {
    let limiter = EngineLimiter::new(6000, 1);
    let first = limiter
        .acquire(Instant::now() + Duration::from_millis(200))
        .await
        .unwrap();
    let second = limiter
        .acquire(Instant::now() + Duration::from_millis(50))
        .await;
    assert_eq!(second.unwrap_err(), NotAdmitted);
    drop(first);
    assert!(
        limiter
            .acquire(Instant::now() + Duration::from_millis(200))
            .await
            .is_ok()
    );
}

/// REQ-CMP-002 and LIM-B4: a provider's `Retry-After` blocks the engine
/// for that time, capped, and the block is visible.
#[tokio::test]
async fn req_cmp_002_retry_after_blocks_until_elapsed() {
    let limiter = EngineLimiter::new(6000, 2);
    limiter.block_for(Duration::from_millis(80), Duration::from_secs(900));
    assert!(limiter.is_blocked());
    assert_eq!(
        limiter
            .acquire(Instant::now() + Duration::from_millis(10))
            .await
            .unwrap_err(),
        NotAdmitted
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!limiter.is_blocked());
    assert!(
        limiter
            .acquire(Instant::now() + Duration::from_millis(100))
            .await
            .is_ok()
    );
    let capped = EngineLimiter::new(6000, 2);
    capped.block_for(Duration::from_secs(3600), Duration::from_millis(50));
    tokio::time::sleep(Duration::from_millis(70)).await;
    assert!(
        !capped.is_blocked(),
        "the block is capped at the maximum cooldown"
    );
}

/// REQ-ENG-010: the token bucket admits at most the burst at once; a
/// request beyond it waits and is refused when the deadline is closer.
#[tokio::test]
async fn req_eng_010_rate_bucket_refuses_beyond_burst_before_deadline() {
    let limiter = EngineLimiter::new(10, 8);
    let permit = limiter
        .acquire(Instant::now() + Duration::from_millis(100))
        .await
        .unwrap();
    drop(permit);
    let second = limiter
        .acquire(Instant::now() + Duration::from_millis(30))
        .await;
    assert_eq!(
        second.unwrap_err(),
        NotAdmitted,
        "one per six seconds at 10 per minute with a burst of one"
    );
}

/// REQ-TEST-016: the provider limit of REQ-CMP-002 is tested: a limiter at
/// the record's rate admits the burst and then waits.
#[tokio::test]
async fn req_test_016_provider_rate_limit_is_enforced() {
    let limiter = EngineLimiter::new(60, 4);
    let mut admitted = 0;
    for _ in 0..10 {
        if limiter
            .acquire(Instant::now() + Duration::from_millis(5))
            .await
            .is_ok()
        {
            admitted += 1;
        }
    }
    assert_eq!(admitted, 6, "60 per minute allows a burst of 6");
}
