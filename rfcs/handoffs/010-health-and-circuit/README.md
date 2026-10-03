# Handoff — RFC 010: Health, circuit breaking, provider limits, `engine_status`

**RFC.** [`../../accepted/010-health-and-circuit.md`](../../accepted/010-health-and-circuit.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-STATUS-001
to -003, REQ-RES-004, -005, -009, REQ-OBS-001 to -004, -008, REQ-TEST-003,
-017), `docs/src/maintainers/external-design.md` §3.3, §6, §14,
`docs/src/maintainers/limits-and-defaults.md` LIM-B1 to LIM-B5, LIM-S11,
LIM-S13, `ROADMAP.md` M2.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** M1 closed (release 0.1.0).
**Milestone.** M2. Deliverable 8 closes the milestone.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `telemetry::HealthTable` with `Circuit`, the latency ring buffer and the transition table of RFC 010 | Unit tests per transition row with a paused clock (REQ-TEST-017); the lock is never held across an await (a `clippy::await_holding_lock` denial in the crate lints) |
| 2 | Selection hook in `engine`: cooldown skip with reason `cooldown`, half-open probe assignment under the lock | Test with two concurrent calls: one probe, one skip |
| 3 | Completion hook in `search`: success, counted failure, uncounted failure, cancellation during a probe | Tests per path; `Retry-After` honored and capped |
| 4 | `[resilience]` table in `config`, `Limits` and `tools/check-limits`; validation rules | `rypsok check` refuses `first_cooldown_s > max_cooldown_s` with the key in the message |
| 5 | `Rypsok::status()` snapshot and the adapter's handling of `[status] enabled = false` | Serialized key set test; protocol test that the tool is absent from `tools/list` and refused when called; duration under LIM-S13 in a test with 16 engines |
| 6 | Circuit events in the log with the fields of RFC 010 | stderr capture test for each of the three events |
| 7 | Documentation: `engine_status` in `docs/src/reference/tools.md` with the restart sentence; "Live check" field in both admission records; the live-check statement in the maintainers' part | Built by `mdbook`; sentences present verbatim |
| 8 | M2 closing: `tools/CURRENT_MILESTONE` set to `M2`; coverage script green; `CHANGELOG.md` entry for 0.2.0; threat register reviewed for THR-13 and THR-15 with a note in the PR; the open question of RFC 010 answered in the review package with counter data | CI green with the M2 gate |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | `HealthTable`, `Circuit`, ring buffer, transition tests | — |
| 2 | Configuration table, limits, validation | — |
| 3 | Selection and completion hooks, concurrency tests, log events | 1, 2 |
| 4 | `status()` snapshot, adapter switch, protocol tests | 1 |
| 5 | Documentation and milestone closing | 3, 4 |

## 3. Conventions that the reviewer will check

- No `tokio::spawn`, interval or sleep in the health code; the clock is
  read, never waited on.
- No request is ever made by `engine_status` or by the health table.
- `StatusResponse` has exactly the fields of RFC 003; adding one is a
  change to the external contract, not to this handoff.
- Tests use the paused clock, never real sleeps.

## 4. Review request package

Hand back `.git-exclude/reviews/010-health-and-circuit/README.md` with
links to this handoff and RFC 010, the PR list, the transition test log,
the `engine_status` timing, the counter data for the open question, and any
deviation as a question.
