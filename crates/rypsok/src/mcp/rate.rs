//! Per-tool invocation rate limits (REQ-MCP-010, REQ-SEC-019): a token
//! bucket per tool per caller, consulted before the service is called.
//! Over stdio the one caller is `local`; over HTTP the caller is the
//! authenticated identity (RFC 013).

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use rypsok_core::limits::Limits;

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

/// The identity a call is attributed to for its limits and its log line.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallerId(String);

impl CallerId {
    /// The caller of stdio and of an HTTP listener without callers.
    pub fn local() -> Self {
        CallerId("local".to_owned())
    }

    /// A configured caller.
    pub fn new(id: &str) -> Self {
        CallerId(id.to_owned())
    }

    /// The identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The three limiters of one caller.
#[derive(Debug)]
pub struct ToolLimiters {
    /// `web_search` (LIM-C6).
    pub search: ToolLimiter,
    /// `web_fetch` (LIM-C7).
    pub fetch: ToolLimiter,
    /// `engine_status` (LIM-C8).
    pub status: ToolLimiter,
}

/// The limiters of every caller seen, created on first use. The set of
/// callers is the configured ones plus `local`, so the map stays small.
#[derive(Debug)]
pub struct CallerLimiters {
    per_minute: (u32, u32, u32),
    callers: Mutex<HashMap<CallerId, Arc<ToolLimiters>>>,
}

impl CallerLimiters {
    /// Limiters at the configured rates.
    pub fn new(limits: &Limits) -> Self {
        CallerLimiters {
            per_minute: (
                limits.web_search_per_minute,
                limits.web_fetch_per_minute,
                limits.engine_status_per_minute,
            ),
            callers: Mutex::new(HashMap::new()),
        }
    }

    /// The limiters of a caller.
    pub fn for_caller(&self, caller: &CallerId) -> Arc<ToolLimiters> {
        let mut callers = self.callers.lock().expect("limiters lock");
        Arc::clone(callers.entry(caller.clone()).or_insert_with(|| {
            Arc::new(ToolLimiters {
                search: ToolLimiter::per_minute(self.per_minute.0),
                fetch: ToolLimiter::per_minute(self.per_minute.1),
                status: ToolLimiter::per_minute(self.per_minute.2),
            })
        }))
    }
}
