//! Per-tool invocation rate limits (REQ-MCP-010): a token bucket per tool
//! per process, consulted before the service is called.

use std::num::NonZeroU32;

use governor::clock::{Clock, DefaultClock};
use governor::{Quota, RateLimiter};

/// A limiter for one tool.
pub struct ToolLimiter {
    limiter: RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, DefaultClock>,
    clock: DefaultClock,
}

impl std::fmt::Debug for ToolLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ToolLimiter")
    }
}

impl ToolLimiter {
    /// A limiter that admits `per_minute` calls per minute, with a burst of
    /// the same size.
    pub fn per_minute(per_minute: u32) -> Self {
        let n = NonZeroU32::new(per_minute.max(1)).expect("at least one");
        let clock = DefaultClock::default();
        ToolLimiter {
            limiter: RateLimiter::direct_with_clock(Quota::per_minute(n), clock.clone()),
            clock,
        }
    }

    /// Admits one call, or says how many milliseconds until the next one.
    pub fn admit(&self) -> Result<(), u64> {
        self.limiter.check().map_err(|not_until| {
            let wait = not_until.wait_time_from(self.clock.now());
            (wait.as_millis() as u64).max(1)
        })
    }
}
