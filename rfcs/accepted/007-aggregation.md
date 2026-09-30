# RFC 007 — Aggregation: deadlines, concurrency, cancellation, partial results

**Status.** Accepted (2026-09-30) — implementer may start after RFC 006.
**Tracks.** Milestone M1; requirements REQ-AGG-001 to -009, REQ-MCP-007,
REQ-SEC-011, REQ-SEC-014, REQ-ERR-001, -005, REQ-TEST-004, REQ-TEST-011,
REQ-PERF-004, REQ-PERF-005; external design §6, §7.
**Touches.** `rypsok-core::search`, the mock server used by integration
tests.
**Handoff.** [`../handoffs/007-aggregation/README.md`](../handoffs/007-aggregation/README.md)

## Summary

The search pipeline fans out to the selected engines under one absolute
deadline, aborts what cannot contribute, completes as soon as every engine
has answered or failed, and turns the engine outcomes into exactly one call
outcome with notices. No retries, no detached tasks, no arrival-order
effects.

## Design

### Pipeline

```text
validate ─▶ privacy check ─▶ select engines ─▶ fan out ─▶ collect ─▶
normalize (RFC 008) ─▶ deduplicate ─▶ rank ─▶ boundary (RFC 005) ─▶ cut ─▶ respond
```

`search::run(req, cancel)` owns the whole call. Its deadline is
`start + LIM-S4`; the fan-out deadline is `start + LIM-S4 − LIM-S6`.

### Fan-out

- Engines: the selection of RFC 006, at most LIM-S8; each engine gets one
  `EngineRequest` with LIM-S3 results requested (or fewer if the driver
  allows fewer).
- Each engine call runs in a `tokio::task::JoinSet` task wrapped in
  `tokio::time::timeout(LIM-S5)` and holding a child of the call's
  `CancellationToken`. Dropping the `JoinSet` aborts every task, so a
  cancelled or timed-out search leaves nothing running (REQ-AGG-006).
- Before the request, the task acquires the engine's limiter and semaphore
  with a deadline equal to the fan-out deadline; failure to acquire yields
  `engine_skipped`, `rate_limited`.
- Global concurrency LIM-C1 is a process-wide semaphore acquired in the same
  step.
- A panic inside a driver becomes `EngineError::RequestFailure` through the
  `JoinError`; the process keeps running (REQ-AGG-007, REQ-SEC-014).

### Collection and early completion

The collector awaits `JoinSet::join_next` until every task has finished or
the fan-out deadline passes, whichever comes first; at the deadline it
drops the `JoinSet` (REQ-AGG-009, REQ-AGG-003). Each engine ends in one of:
`Answered(results)`, `Failed(class)`, `Skipped(reason)`.

### Outcome

`decide(n, a, f, r)` of RFC 003:

| n | a | f | r | Result |
|---|---|---|---|---|
| 0 | — | — | — | `unavailable` / `no_engine` |
| ≥1 | 0 | ≥1 | — | `unavailable` / `all_failed`, or `deadline` when every failure is a timeout |
| ≥1 | ≥1 | 0 | ≥1 | `ok` |
| ≥1 | ≥1 | ≥1 | ≥1 | `partial` with one `engine_unavailable` notice per failed engine |
| ≥1 | ≥1 | — | 0 | `empty`, with notices for failed engines |

Skipped engines produce `engine_skipped` notices in every outcome and never
count as failures. `retry_after_ms` in an `unavailable` error is the
smallest `Retry-After` among the failed engines when one exists.

### Cancellation (REQ-MCP-007)

The adapter passes its token; `run` observes it between stages and the
`JoinSet` observes it inside each task; within LIM-O2 no outbound request
remains. A cancelled call returns nothing.

### Determinism (REQ-ARCH-004, REQ-TEST-011)

Collected results are stored per engine and processed in the order of the
engine selection, never in arrival order. The rank fusion of RFC 008 uses
integer arithmetic. A test shuffles the mock server's response order and
asserts byte-identical output.

### Measurements (REQ-PERF-004, REQ-PERF-005)

An integration test records, with the mock server answering instantly, the
time between call start and response as rypsok's overhead, and a separate
manual measurement script (`tools/measure-cold-start`) records the first
call against the real engines. Both results go into the M1 review package
and, if needed, into a change of LIM-S6 and LIM-S4.

### Mock server

Integration tests use `wiremock` (MIT/Apache-2.0) on a loopback port. The
engine egress profile admits it through an allow rule injected in the test
configuration only (REQ-TEST-005 principle).

## Alternatives considered

- **Detached `tokio::spawn` with `join_all`.** Rejected: a cancelled search
  leaves requests running (startup review R-05).
- **Waiting for the full deadline.** Rejected: early completion saves a
  second on most calls.
- **Retrying a failed engine within the call.** Rejected: it spends the
  deadline and the provider's quota on a request that just failed
  (REQ-AGG-008).

## Open questions

None.
