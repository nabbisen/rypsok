# RFC 010 — Health, circuit breaking, provider limits, `engine_status`

**Status.** Accepted (2026-10-03) — implementer may start after M1 closes.
**Tracks.** Milestone M2; requirements REQ-STATUS-001 to -003, REQ-RES-004,
-005, -009, REQ-OBS-001 to -004, -008, REQ-TEST-003, REQ-TEST-017; external
design §3.3, §6, §14; threats THR-04, THR-13, THR-15; decisions D-26, D-33.
**Touches.** `rypsok-core::telemetry` (health state), `rypsok-core::engine`
(selection), `rypsok-core::search` (completion hook), `rypsok::mcp`
(`engine_status`), the `[resilience]` table of RFC 004.
**Handoff.** [`../handoffs/010-health-and-circuit/README.md`](../handoffs/010-health-and-circuit/README.md)

## Summary

Each engine carries a circuit with four states, driven only by real
requests and the clock, held in memory, read without waiting. The search
pipeline consults it at selection and informs it at completion. The tool
`engine_status` returns a snapshot of exactly three fields per engine and
nothing else. No background task, no synthetic probe, no persistence.

## Design

### State per engine (REQ-OBS-001)

```rust
pub enum Circuit {
    Disabled,                                              // from configuration; never changes at run time
    Closed { consecutive_failures: u8 },
    Open { until: Instant, cooldown: Duration },
    HalfOpen { cooldown: Duration, probe_in_flight: bool },
}

pub struct Health {
    circuit: Circuit,
    latencies: RingBuffer<u32, LIM_S11>,                   // milliseconds of completed requests
}
```

`Health` lives in `telemetry::HealthTable`, one entry per configured engine
behind a `std::sync::RwLock` (held for nanoseconds, never across an await).
It is built with the registry and dropped with the process (REQ-RES-009).

### Transitions (REQ-RES-004, REQ-RES-005)

Evaluated at two moments only: when the pipeline selects engines and when
an engine request completes. There is no timer task.

| From | Event | To |
|---|---|---|
| `Closed{n}` | Failure of class `timeout`, `rate_limited`, `blocked` or `parse_failure`, with n + 1 < LIM-B1 | `Closed{n + 1}` |
| `Closed{n}` | Such a failure with n + 1 = LIM-B1 | `Open{until = now + c, cooldown = c}` where c = LIM-B2, or the provider's `Retry-After` when larger, capped by the LIM-B3 maximum (LIM-B4); event `circuit_opened` |
| `Closed{n}` | Success, or a failure of class `auth_failure` or `request_failure` | `Closed{0}` on success; unchanged on those classes |
| `Open` | Selection with `now < until` | Unchanged; the engine is skipped with notice `engine_skipped`, reason `cooldown` |
| `Open{cooldown}` | Selection with `now ≥ until` | `HalfOpen{cooldown, probe_in_flight = true}`; this call carries the probe (LIM-B5); event `circuit_half_open` |
| `HalfOpen{probe_in_flight = true}` | Selection by another call | Unchanged; skipped with reason `cooldown` |
| `HalfOpen` | Probe succeeds | `Closed{0}`; event `circuit_closed` |
| `HalfOpen{cooldown}` | Probe fails with a counted class | `Open{until = now + c', cooldown = c'}` with c' = min(2 × cooldown, LIM-B3 maximum), or `Retry-After` when larger within the cap; event `circuit_opened` |
| `HalfOpen` | Probe fails with `auth_failure` or `request_failure`, or the call is cancelled before completion | `Open` again with the same cooldown; event `circuit_opened` |

`auth_failure` never opens the circuit: it is an operator problem, logged at
`warn` once per engine per process, and the engine keeps being selected so
that a corrected key takes effect at once. `request_failure` (network and
5xx) is also outside the counted classes by REQ-RES-004; it is counted in
the error counters and left to the M2 review as an open question below.

The probe is the real request of the call that found the cooldown elapsed;
it is an ordinary fan-out member with the ordinary timeout. Only one call
holds the probe: the transition to `HalfOpen` happens under the lock, so a
second concurrent call sees `probe_in_flight = true`.

### Interaction with the per-engine limiter (RFC 006)

The limiter and the circuit are separate controls with separate reasons:
a limiter that cannot be acquired yields `rate_limited`, a cooldown yields
`cooldown`. A 429 does both: the limiter is emptied until `Retry-After`,
and the circuit counts one `rate_limited` failure. The circuit is consulted
first at selection, so an engine in cooldown never touches its limiter.

### Latency (REQ-OBS-002)

Every engine request that received a response, successful or classified,
records its duration in the engine's ring buffer; timeouts and skips do
not. `latency_ms` is the integer median of the buffer, or null when the
buffer is empty. Every call records its own duration in the `info` event
of RFC 009.

### Counters (REQ-OBS-003)

The counters of RFC 009 carry engine errors by class and call errors by
kind from M1. M2 adds the three circuit events to the log at `info`, with
fields `engine`, `state`, `cooldown_ms`, `consecutive_failures`
(REQ-OBS-004). No other exposure in M2 (REQ-OBS-008).

### `engine_status` (REQ-STATUS-001 to -003)

`Rypsok::status()` takes a read lock, copies `(id, state, latency_ms)` for
every configured engine in configuration order, releases the lock, and
returns `StatusResponse { outcome: Ok, notices: [], engines }`. It performs
no I/O and awaits nothing, which keeps it inside LIM-S13. `state` is the
serialized `EngineState` of RFC 003: `disabled`, `closed`, `open`,
`half_open`. The structure has no other field, so no endpoint, credential
or provider text can appear (REQ-STATUS-002); a test asserts the serialized
key set. When `[status] enabled = false`, the adapter of RFC 003 omits the
tool from `tools/list` and answers a call to it as an unknown tool.

### Configuration

RFC 004 gains the table below; the values mirror LIM-B1 to LIM-B3 and are
validated as `10 ≤ first_cooldown_s ≤ max_cooldown_s ≤ 3600` and
`1 ≤ failures_to_open ≤ 10`.

```toml
[resilience]
failures_to_open = 3                  # LIM-B1
first_cooldown_s = 60                 # LIM-B2
max_cooldown_s = 900                  # LIM-B3
```

### Live checks (REQ-TEST-003)

The scheduled job of RFC 006 is the live check. M2 documents it: each
admission record in `docs/src/maintainers/engines/` gains a "Live check"
field naming the query, the assertion, the schedule and what a failure
means (format drift), and `docs/src/maintainers/` states that the job never
gates and never runs on a pull request.

### Documentation (REQ-RES-009)

`docs/src/reference/tools.md` describes `engine_status` with the sentence:
"State lives in memory and is reset when the process restarts." The
integration guide repeats it for hosts that restart rypsok per session.

### Tests (REQ-TEST-017)

With a paused Tokio clock and a mock driver that fails on command: opening
after LIM-B1 failures; a success in between resetting the count; `Open`
skipped with reason `cooldown`; `HalfOpen` after the cooldown with exactly
one probe under two concurrent calls; closing on probe success; doubling on
probe failure up to the cap; `Retry-After` larger than the cooldown
honored and capped; `auth_failure` not counted; cancellation during a probe
reopening; `engine_status` snapshot key set and median; `latency_ms` null
with no completed request; `[status] enabled = false` removing the tool.

## Amendment to RFC 004

The `[resilience]` table above is added to the configuration schema and to
`Limits`; applied in the same commit.

## Alternatives considered

- **A background task that ticks the circuits.** Rejected: nothing changes
  without a request, and a task is one more thing that must stop on
  shutdown (REQ-ARCH-004, D-33).
- **Synthetic probe requests to the provider.** Rejected by D-33 and
  REQ-RES-005: they spend quota and announce rypsok's presence without a
  caller's need.
- **Counting `request_failure` toward the circuit.** Not in REQ-RES-004;
  recorded as an open question rather than decided here.
- **Persisting circuit state.** Rejected by REQ-RES-009; a restart is the
  operator's reset.

## Open questions

- Whether `request_failure` (network errors and 5xx) should count toward
  opening the circuit. The M2 review decides with the error counters in
  hand; adopting it is a change to REQ-RES-004.
