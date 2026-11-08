# Handoff 020j — Standard-library locks that survive a poisoned state

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3). Finding
NET-13 (audit 020b; Low): 29 sites call `.expect("… lock")` on a
`std::sync` lock. A panic while one is held would make every later use
panic. No input reaches it today; the failure mode would be total.

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md).
**Rules.** As 020g (commit your own work; push only after approval).
**Depends on.** Nothing. **Evidence rule.** As 020g.

## Design (decided here, no judgment left)

The critical sections in these files only insert into maps, push to queues,
read or set an `Instant`, or use `saturating_*` arithmetic; none can panic,
and every state they leave is valid at each statement. Recovering the guard
from a poisoned lock (`PoisonError::into_inner`) is therefore safe and keeps
the service answering. The result cache is the one deliberate exception: it
disables itself on poison (RFC 014, tested by
`req_cache_006_a_poisoned_lock_disables_the_cache`) and **stays as it is**.
`mcp/http/sessions.rs` already recovers.

## Change scope

| # | Required implementation | Required tests |
|---|---|---|
| 1 | In `rypsok-core`, a crate-private module `sync` with `lock(&Mutex<T>) -> MutexGuard<T>`, `read(&RwLock<T>)` and `write(&RwLock<T>)`, each recovering from poison, with a doc comment that states the rule above. In `rypsok`, `mcp/rate.rs` gets a local one-line equivalent (no new public API in either crate) | Per helper: poison the lock in a thread that panics while holding it, then use the helper and assert the value is readable and writable |
| 2 | Replace the `expect("… lock")` calls in non-test code: `telemetry/counters.rs` (14), `engine/limiter.rs` (2), `fetch/robots.rs` (3), `egress/strict.rs` (3), `telemetry/health.rs` (5), `fetch.rs` (1), `mcp/rate.rs` (1); find them with `grep -rn 'expect(".*lock' crates/*/src` and report the final count (the audit counted 29) | One behavioral test: poison the `Counters` lock, then record a call and read the counters |
| 3 | A gate test in `crates/rypsok/tests/m0_gates.rs`: no non-test source file contains `.lock().expect(`, `.read().expect(`, `.write().expect(` (exempt `cache.rs`, which handles poison itself, and test code) | The gate test, seen failing first on the unmodified tree |

## Explicit non-change scope

`cache.rs`; any lock that is not `std::sync`; any behavior other than the
poisoned case; the public API of both crates (`cargo semver-checks` must stay
green in CI).

## Documentation and changelog

`threat-register.md`: if a row mentions lock poisoning, update it; the
changelog `[Unreleased]` Fixed: "a panic inside a critical section no longer
makes every later call panic". The review request states the count of
replaced sites and lists each file.

## Required evidence and review request

The gates as in 020g; the review request at
`.git-exclude/review-requests/020j-lock-helper/README.md`.
