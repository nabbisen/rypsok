# Handoff — RFC 018: Threat model revision and hardening

**RFC.** [`../../done/018-threat-model-revision.md`](../../done/018-threat-model-revision.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-SEC-018,
REQ-SEC-020, REQ-SEC-022, REQ-SEC-023, REQ-SEC-013, REQ-SEC-016,
REQ-PRIV-004, REQ-TEST-013, REQ-DEP-003), `docs/src/maintainers/external-design.md`
§1, §8, §10, §15, §16, `docs/src/maintainers/threat-register.md`,
`docs/src/maintainers/limits-and-defaults.md` LIM-H1 to LIM-H5,
`ROADMAP.md` M7.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** Release 0.6.0.
**Milestone.** M7. Deliverable 8 closes the milestone (release 0.7.0 on
the owner's authorization).

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `threat-model.md` (assets, attackers, flows F1–F8 with controls, how to read the register), in the book's maintainers part; `SUMMARY.md` | `mdbook build` green |
| 2 | The register revised: THR-25, THR-26, THR-22 extended, residual rows, review log rows M1–M7 | Every THR id unique; every row names a control requirement and a verifying requirement that exist |
| 3 | LIM-H4, LIM-H5 (constants, rows, `[service.http] max_streams`, `request_timeout_s`, validation with the fetch-deadline rule, page rows) | `check-limits` 49 rows; page test green; `req_sec_022_request_timeout_needs_room_for_a_fetch` |
| 4 | The gate's request timeout (408, `http_refused` reason `request_timeout`) and `max_concurrent_streams` | `req_sec_022_a_slow_body_is_ended_with_408` against the in-process listener: headers and a partial body, then nothing; 408 within LIM-H5 + 1 s; the connection permit is released |
| 5 | I5 amended; `PROVIDER_TEXT_MAX_CHARS` constant and a test that `debug_provider_text` truncates to it | `req_sec_016_provider_text_in_logs_is_bounded_and_debug_only` |
| 6 | Every workflow action pinned to a commit hash with a version comment; the gates test `req_sec_020_ci_actions_are_pinned` | CI green with the pinned actions |
| 7 | `fuzz/` with the eight targets; the scheduled job builds and runs them (nightly, fixed time per target, report only); each target run locally once | A note per target in the package: time run, executions, findings |
| 8 | Deployment page hardened; REQ-SEC-020/022/023 in the requirements; `tools/CURRENT_MILESTONE = M7`; coverage gate; `CHANGELOG.md` Unreleased for 0.7.0; the review request package with the release-readiness report | CI green with the M7 gate |

## 2. Conventions that the reviewer will check

- A finding of the walk that changes nothing is still recorded as
  verified, with the file and the line that holds the control.
- The 408 is answered by the gate, never by the SDK; the log carries the
  reason and the address family, never the body.
- A fuzz target feeds bytes to the public function a caller would reach;
  it never constructs an internal state by hand.
- No behavior change rides with this milestone except H1 and the
  profiles; each is a changelog line.

## 3. Tests

`req_sec_022_*` (two), `req_sec_016_*` (one), `req_sec_020_ci_actions_are_pinned`,
plus the existing listener tests unchanged in outcome.

## 4. Review request package

`.git-exclude/review-requests/018-threat-model-revision/README.md`: the
walk's findings with evidence, the gate results, the fuzz runs, and the
release-readiness report (risk per dimension, Go/No-Go for 0.7.0).
