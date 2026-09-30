# Handoff — RFC 006: Engine abstraction, registry, admission; Wikipedia and Brave

**RFC.** [`../../accepted/006-engines-and-admission.md`](../../accepted/006-engines-and-admission.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-ENG-001 to
-012, REQ-CMP-001 to -004, REQ-SEC-017, REQ-RES-002, -008, REQ-TEST-002,
-003, -015, -016), `docs/src/maintainers/external-design.md` §8, §11,
`docs/src/maintainers/threat-register.md` THR-08, THR-13, THR-15, THR-20,
`ROADMAP.md` M1.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 004 PR 2 (configuration and secrets), RFC 003 PR 1.
**Milestone.** M1.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `rypsok-core::engine`: `Driver` trait, `EngineProperties`, `EngineRequest`, `EngineResponse`, `EngineError` with the classes of REQ-ENG-006 | Types compile behind the trait; a mock driver in tests implements it |
| 2 | Registry built from `[engines.*]`: declared keys, missing secret → `disabled`, selection by category and preference | Tests named after REQ-ENG-007, REQ-OPS-003; an undeclared key fails `rypsok check` |
| 3 | Per-engine limiter (`governor`) and in-flight semaphore; `Retry-After` handling | Tests with a paused Tokio clock: acquisition within the deadline, skip when exhausted, bucket emptied until `Retry-After` |
| 4 | `rypsok-core::egress::engine_client`: one `reqwest::Client` per engine with the engine profile settings of RFC 006 | A test against a loopback server that a redirect is not followed, that the credential header is present only on the engine's origin, that the body read stops at `max_response_bytes` |
| 5 | Wikipedia driver, feature `engine-wikipedia`, fixtures `crates/rypsok-core/fixtures/wikipedia/`, admission record `docs/src/maintainers/engines/wikipedia.md` and its `SUMMARY.md` entry | Fixture tests for every file; the record has every field of REQ-CMP-001 |
| 6 | Brave driver with `endpoint = "web" | "news"`, feature `engine-brave`, fixtures `fixtures/brave/`, admission record `engines/brave.md` | Same; the 75-word rule is tested; no credential in any fixture |
| 7 | Scheduled live check job for both drivers (Brave skipped without the secret) | Present in the weekly workflow; a run log in the PR |
| 8 | Measurement of Wikipedia's real query length limit, written into the admission record | Number and method recorded |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | Trait, properties, registry, tests with a mock driver | RFC 004 PR 2 |
| 2 | Limiter, semaphore, engine client | 1 |
| 3 | Wikipedia driver, fixtures, admission record | 2 |
| 4 | Brave driver, fixtures, admission record | 2 |
| 5 | Live check job, query length measurement | 3, 4 |

## 3. Conventions that the reviewer will check

- Provider text never reaches a response or a log line above `debug`.
- The User-Agent is one string per process from REQ-CMP-004, with the
  operator contact appended; no rotation.
- Fixtures are hand-edited copies with credentials and personal data
  removed; the PR says how each was recorded.
- No driver reads the environment directly; keys come from the registry.

## 4. Review request package

Hand back `.git-exclude/reviews/006-engines-and-admission/README.md` with
links to this handoff and RFC 006, the PR list, the fixture test output, the
live check log, the measured Wikipedia limit, and any deviation as a
question.
