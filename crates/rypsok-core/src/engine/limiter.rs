//! The per-engine limiter: a token bucket at the admission record's rate
//! and a semaphore of in-flight requests (REQ-CMP-002, REQ-ENG-010), with
//! `Retry-After` honored (LIM-B4).

use std::num::NonZeroU32;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use governor::clock::DefaultClock;
use governor::{Quota, RateLimiter};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Why the limiter did not admit a request before the deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotAdmitted;

/// The limiter of one engine.
pub struct EngineLimiter {
    bucket: RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, DefaultClock>,
    in_flight: std::sync::Arc<Semaphore>,
    blocked_until: Mutex<Option<Instant>>,
}

impl std::fmt::Debug for EngineLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineLimiter")
            .field("available_permits", &self.in_flight.available_permits())
            .finish()
    }
}

impl EngineLimiter {
    /// A limiter admitting `per_minute` requests per minute with a burst
    /// of one tenth of that (at least one), and `max_in_flight` requests
    /// at once.
    pub fn new(per_minute: u32, max_in_flight: u8) -> Self {
        let rate = NonZeroU32::new(per_minute.max(1)).expect("at least one");
        let burst = NonZeroU32::new((per_minute / 10).max(1)).expect("at least one");
        EngineLimiter {
            bucket: RateLimiter::direct(Quota::per_minute(rate).allow_burst(burst)),
            in_flight: std::sync::Arc::new(Semaphore::new(usize::from(max_in_flight.max(1)))),
            blocked_until: Mutex::new(None),
        }
    }

    /// Blocks the engine until `now + retry_after` (a provider's
    /// `Retry-After`), capped by `max`.
    pub fn block_for(&self, retry_after: Duration, max: Duration) {
        let until = Instant::now() + retry_after.min(max);
        let mut blocked = self.blocked_until.lock().expect("limiter lock");
        if blocked.is_none_or(|current| current < until) {
            *blocked = Some(until);
        }
    }

    /// Whether a provider block is in force.
    pub fn is_blocked(&self) -> bool {
        self.blocked_until
            .lock()
            .expect("limiter lock")
            .is_some_and(|until| until > Instant::now())
    }

    /// Admits one request before `deadline`: the bucket first, then the
    /// in-flight semaphore. The permit is released when dropped.
    pub async fn acquire(&self, deadline: Instant) -> Result<OwnedSemaphorePermit, NotAdmitted> {
        if self.is_blocked() {
            return Err(NotAdmitted);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        tokio::time::timeout(remaining, self.bucket.until_ready())
            .await
            .map_err(|_| NotAdmitted)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        tokio::time::timeout(remaining, self.in_flight.clone().acquire_owned())
            .await
            .map_err(|_| NotAdmitted)?
            .map_err(|_| NotAdmitted)
    }
}

#[cfg(test)]
mod tests;
