//! In-process counters (REQ-OBS-003, REQ-OBS-007): calls per tool and
//! outcome, refusals per tool and reason, warnings per tool and reason,
//! engine errors per engine and class, characters removed, results dropped.
//! Logged as one `info` event every minute when something changed, from the
//! call or the refusal that notices the minute has passed; no background
//! task.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How often the counters are logged.
pub const FLUSH_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Totals {
    calls: BTreeMap<(&'static str, &'static str), u64>,
    refusals: BTreeMap<(&'static str, &'static str), u64>,
    warnings: BTreeMap<(&'static str, &'static str), u64>,
    engine_errors: BTreeMap<(String, &'static str), u64>,
    characters_removed: u64,
    results_dropped: u64,
    /// The result cache: `hit`, `miss`, `stored`, `too_large`, `bypassed`
    /// (RFC 014).
    cache: BTreeMap<&'static str, u64>,
}

/// The counters of a process.
#[derive(Debug)]
pub struct Counters {
    totals: Mutex<Totals>,
    last_flush: Mutex<Instant>,
    flush_interval: Duration,
}

impl Default for Counters {
    fn default() -> Self {
        Self::with_flush_interval(FLUSH_INTERVAL)
    }
}

impl Counters {
    /// A fresh set.
    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh set that logs itself after `interval` instead of after a
    /// minute; for tests.
    pub fn with_flush_interval(interval: Duration) -> Self {
        Counters {
            totals: Mutex::new(Totals::default()),
            last_flush: Mutex::new(Instant::now()),
            flush_interval: interval,
        }
    }

    /// One call of a tool ended with this outcome or error kind.
    pub fn call(&self, tool: &'static str, outcome: &'static str) {
        *self
            .totals
            .lock()
            .expect("counters lock")
            .calls
            .entry((tool, outcome))
            .or_default() += 1;
        self.flush_if_due();
    }

    /// One event of the result cache (RFC 014).
    pub fn cache(&self, outcome: &'static str) {
        *self
            .totals
            .lock()
            .expect("counters lock")
            .cache
            .entry(outcome)
            .or_default() += 1;
    }

    /// The count of one cache outcome.
    pub fn cache_of(&self, outcome: &str) -> u64 {
        self.totals
            .lock()
            .expect("counters lock")
            .cache
            .get(outcome)
            .copied()
            .unwrap_or(0)
    }

    /// One call was refused before work started (REQ-OBS-007).
    pub fn refused(&self, tool: &'static str, reason: &'static str) {
        *self
            .totals
            .lock()
            .expect("counters lock")
            .refusals
            .entry((tool, reason))
            .or_default() += 1;
    }

    /// One HTTP request was refused by the listener, by its gate or by the
    /// SDK behind it (REQ-OBS-007, RFC 019 D6), counted under the tool
    /// `http` and the reason. Like a call, it runs the flush check:
    /// without it a flood of refused requests and no tool call would
    /// never be logged.
    pub fn refused_http(&self, reason: &'static str) {
        self.refused("http", reason);
        self.flush_if_due();
    }

    /// One call proceeded with a warning (privacy mode `warn`); not a
    /// refusal.
    pub fn warned(&self, tool: &'static str, reason: &'static str) {
        *self
            .totals
            .lock()
            .expect("counters lock")
            .warnings
            .entry((tool, reason))
            .or_default() += 1;
    }

    /// One engine request failed with this class (REQ-OBS-003).
    pub fn engine_error(&self, engine: &str, class: &'static str) {
        *self
            .totals
            .lock()
            .expect("counters lock")
            .engine_errors
            .entry((engine.to_owned(), class))
            .or_default() += 1;
    }

    /// Characters the boundary removed.
    pub fn characters_removed(&self, count: u64) {
        self.totals
            .lock()
            .expect("counters lock")
            .characters_removed += count;
    }

    /// Results the pipeline dropped.
    pub fn results_dropped(&self, count: u64) {
        self.totals.lock().expect("counters lock").results_dropped += count;
    }

    /// The total calls of a tool with an outcome, for tests and status.
    pub fn calls_of(&self, tool: &str, outcome: &str) -> u64 {
        self.totals
            .lock()
            .expect("counters lock")
            .calls
            .iter()
            .filter(|((t, o), _)| *t == tool && *o == outcome)
            .map(|(_, n)| *n)
            .sum()
    }

    /// The total refusals of a tool with a reason.
    pub fn refusals_of(&self, tool: &str, reason: &str) -> u64 {
        self.totals
            .lock()
            .expect("counters lock")
            .refusals
            .iter()
            .filter(|((t, r), _)| *t == tool && *r == reason)
            .map(|(_, n)| *n)
            .sum()
    }

    /// The total warnings of a tool with a reason.
    pub fn warnings_of(&self, tool: &str, reason: &str) -> u64 {
        self.totals
            .lock()
            .expect("counters lock")
            .warnings
            .iter()
            .filter(|((t, r), _)| *t == tool && *r == reason)
            .map(|(_, n)| *n)
            .sum()
    }

    /// The total errors of an engine with a class.
    pub fn engine_errors_of(&self, engine: &str, class: &str) -> u64 {
        self.totals
            .lock()
            .expect("counters lock")
            .engine_errors
            .iter()
            .filter(|((e, c), _)| e == engine && *c == class)
            .map(|(_, n)| *n)
            .sum()
    }

    fn flush_if_due(&self) {
        let due = {
            let mut last = self.last_flush.lock().expect("counters lock");
            if last.elapsed() >= self.flush_interval {
                *last = Instant::now();
                true
            } else {
                false
            }
        };
        if due {
            self.log();
        }
    }

    /// Logs the counters now, when any is non-zero.
    pub fn log(&self) {
        let totals = self.totals.lock().expect("counters lock").clone();
        if totals == Totals::default() {
            return;
        }
        let render = |map: &BTreeMap<(&'static str, &'static str), u64>| {
            map.iter()
                .map(|((a, b), n)| format!("{a}.{b}={n}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let engine_errors = totals
            .engine_errors
            .iter()
            .map(|((e, c), n)| format!("{e}.{c}={n}"))
            .collect::<Vec<_>>()
            .join(" ");
        let cache = totals
            .cache
            .iter()
            .map(|(k, n)| format!("{k}={n}"))
            .collect::<Vec<_>>()
            .join(" ");
        tracing::info!(
            event = "counters",
            calls = %render(&totals.calls),
            refusals = %render(&totals.refusals),
            warnings = %render(&totals.warnings),
            engine_errors = %engine_errors,
            characters_removed = totals.characters_removed,
            results_dropped = totals.results_dropped,
            cache = %cache,
            "counters"
        );
    }
}
