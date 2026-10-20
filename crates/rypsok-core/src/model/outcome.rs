//! The outcome decision table of external design §7 as one pure function
//! (REQ-ERR-001, REQ-ERR-005).

use super::Outcome;

/// What a search ends with, given the engine counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// A successful outcome.
    Outcome(Outcome),
    /// No engine was queried: `unavailable`, reason `no_engine`.
    NoEngine,
    /// Every queried engine failed: `unavailable`, reason `all_failed`
    /// (or `deadline` when every failure was a timeout; the aggregator
    /// refines this).
    AllFailed,
}

/// Decides the outcome from the number of engines queried (`n`), answered
/// (`a`), failed or timed out (`f`), and the results after ranking (`r`).
///
/// Skipped engines are not counted in `n`; `a + f == n` for queried engines.
pub fn decide(n: u32, a: u32, f: u32, r: u32) -> Decision {
    if n == 0 {
        Decision::NoEngine
    } else if a == 0 {
        Decision::AllFailed
    } else if r == 0 {
        Decision::Outcome(Outcome::Empty)
    } else if f == 0 {
        Decision::Outcome(Outcome::Ok)
    } else {
        Decision::Outcome(Outcome::Partial)
    }
}
