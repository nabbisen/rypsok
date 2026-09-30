# Handoff — RFC 007: Aggregation

**RFC.** [`../../accepted/007-aggregation.md`](../../accepted/007-aggregation.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-AGG-001 to
-009, REQ-MCP-007, REQ-SEC-011, -014, REQ-ERR-001, -005, REQ-TEST-004, -011,
REQ-PERF-004, -005), `docs/src/maintainers/external-design.md` §6, §7,
`ROADMAP.md` M1.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 006 PR 2 (registry, limiter, client); RFC 005 PR 2 and
RFC 008, RFC 009 for the wiring PR.
**Milestone.** M1.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `rypsok-core::search::run`: the pipeline of RFC 007 with the absolute deadline and the fan-out deadline | Integration tests with `wiremock`: one fast and one slow engine yield `partial` before LIM-S4; all engines fast yield `ok` well before the deadline (REQ-AGG-009) |
| 2 | Fan-out on a `JoinSet` with per-engine timeout, limiter and semaphore acquisition, global LIM-C1 semaphore | Tests with a paused clock for each skip and failure path |
| 3 | Collection and outcome through `decide` | Tests for every row of the outcome table with the mock server |
| 4 | Cancellation: token observed between stages and in tasks; nothing in flight LIM-O2 after cancellation | A test that counts the mock server's open connections after cancel |
| 5 | Panic isolation: a driver that panics becomes `request_failure` and the process continues | Test named after REQ-SEC-014 |
| 6 | Wiring of the privacy check (RFC 009), the boundary (RFC 005), normalization, deduplication and ranking (RFC 008) in the order of RFC 007 | End-to-end test with fixtures through the stdio adapter producing the §3 example shape |
| 7 | Determinism test: shuffled response order, byte-identical output (REQ-TEST-011) | Passes 100 iterations in CI |
| 8 | `tools/measure-cold-start` and the overhead measurement (REQ-PERF-004, -005) | Numbers in the review package; a proposal for LIM-S4 and LIM-S6 if the numbers contradict the defaults |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | Pipeline skeleton, fan-out, collection, outcome, `wiremock` tests | RFC 006 PR 2 |
| 2 | Cancellation, panic isolation, determinism test | 1 |
| 3 | Wiring of the other M1 modules and the end-to-end test | 2, RFC 005 PR 2, RFC 008 PR 2, RFC 009 PR 2 |
| 4 | Measurement script and results | 3 |

## 3. Conventions that the reviewer will check

- No `tokio::spawn` outside the `JoinSet`; no task outlives the call.
- No retry anywhere in the search path.
- Results are processed in engine selection order; no code path depends on
  arrival order.
- The mock server is admitted by a test-only allow rule, never by a change
  to the engine profile.

## 4. Review request package

Hand back `.git-exclude/reviews/007-aggregation/README.md` with links to
this handoff and RFC 007, the PR list, the integration test log, the
determinism test count, the measurements, and any deviation as a question.
