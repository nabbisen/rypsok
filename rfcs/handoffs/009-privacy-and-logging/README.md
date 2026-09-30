# Handoff — RFC 009: Query privacy and logging

**RFC.** [`../../accepted/009-privacy-and-logging.md`](../../accepted/009-privacy-and-logging.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-PRIV-001 to
-005, REQ-OBS-003, -005 to -007, REQ-SEC-020, REQ-TEST-008),
`docs/src/maintainers/external-design.md` §10, §13.3,
`docs/src/maintainers/threat-register.md` THR-09, THR-10, THR-16,
`ROADMAP.md` M1.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 003 PR 1, RFC 004 PR 2.
**Milestone.** M1. Deliverable 7 closes the milestone.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `rypsok-core::privacy::scan` with the pattern table of RFC 009 as one constant, and the four modes of `privacy.secret_detection` | Positive and near-miss tests per pattern with synthetic values; a test per mode; `denied` carries `kind` and `position`, never the value |
| 2 | `Sensitive<String>` for query and URL, `Secret<String>` shared with RFC 004; neither implements `Display` or `Debug` with content | Compile-fail test (`trybuild`) that `format!("{}", sensitive)` does not compile |
| 3 | `rypsok-core::telemetry`: `tracing` JSON subscriber to stderr, the field set of RFC 009, per-call `request_id`, `log_queries` unlock with a start-up warning | A test that captures stderr for a refused call and asserts the secret is absent at every level; a test that `log_queries = true` logs the warning once |
| 4 | Counters (calls by tool and outcome, refusals by tool and reason, engine errors by engine and class, characters removed, results dropped) with the periodic `info` event | Tests that each path increments; the event appears only when a counter is non-zero |
| 5 | Disclosure text in `docs/src/reference/tools.md` with the tool profiles of external design §3 | Built by `mdbook`; the sentence of RFC 009 present verbatim |
| 6 | `CHANGELOG.md` entry for 0.1.0 listing the M1 tools and limits | Present |
| 7 | M1 closing: `tools/CURRENT_MILESTONE` set to `M1`; the coverage script passes; every exit criterion of `ROADMAP.md` M1 checked in the PR description | CI green with the M1 gate |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | `scan`, modes, `Sensitive` | RFC 003 PR 1 |
| 2 | Telemetry subscriber, `request_id`, counters | 1, RFC 004 PR 2 |
| 3 | Disclosure page, changelog, milestone closing | 2, RFC 007 PR 4 |

## 3. Conventions that the reviewer will check

- No `tracing` field ever receives a `Sensitive` or `Secret` value; the
  type system is the proof, not a review.
- Test samples are synthetic; a reviewer greps the fixtures with the same
  patterns and finds only the test files.
- Stderr is the only log sink; no file, no network.

## 4. Review request package

Hand back `.git-exclude/reviews/009-privacy-and-logging/README.md` with
links to this handoff and RFC 009, the PR list, the stderr capture test
output, the coverage script output at M1, and any deviation as a question.
