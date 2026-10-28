# Handoff — RFC 015: Scraping drivers as an opt-in engine type

**RFC.** [`../../done/015-scraping-drivers.md`](../../done/015-scraping-drivers.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-ENG-008,
REQ-ENG-009, REQ-ENG-013 to -015, REQ-CMP-001, REQ-CMP-002, REQ-CMP-004,
REQ-RES-002, REQ-TEST-003, REQ-TEST-018, REQ-MAIN-004, REQ-OPS-003),
`docs/src/maintainers/external-design.md` §11, `docs/src/maintainers/limits-and-defaults.md`
LIM-G1, LIM-C2, LIM-C5, LIM-C9, LIM-F6, LIM-F10, LIM-F11,
`docs/src/maintainers/threat-register.md` THR-01, THR-20,
`docs/src/maintainers/engines/README.md`, `ROADMAP.md` M5.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 014 PR 2 (so that `html` is moved once).
**Milestone.** M5. Deliverable 6 closes the milestone (release 0.5.0).

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `rypsok_core::html`: `fetch::dom` and `fetch::charset` moved, `fetch` importing them; no behavior change | The fetch corpus and all fetch tests unchanged; `tools/check-core-dependencies` green |
| 2 | `scraping` property; `[scraping] enabled` (LIM-G1); validation with the exact message; the default set never contains a scraping engine; `warn` `scraping_enabled` | `req_eng_013_*` tests; page test green; `check-limits` 48 rows |
| 3 | `engine::scrape`: the result shape, the matcher, the iterative walk on the bounded DOM under LIM-C5, the three page states | Unit tests on the fixtures; no recursion; a page above LIM-F10 never panics |
| 4 | The `duckduckgo` driver with its admission record written against the live site; the standard User-Agent, no browser headers; redirects on-host only; `blocked`/`rate_limited` classification; the live check in `scheduled.yml` | `req_eng_014_*`, `req_eng_015_*`, `req_test_018_*` with `wiremock`; the record's table complete (endpoint, permitted use, identification, rate, storage, attribution, robots.txt status, markers, live check) |
| 5 | Documentation: configuration page (`[scraping]`, the scraping engines table, the operator's responsibility sentence, robots.txt not consulted), engines README row, integration guide note | `mdbook build` green |
| 6 | M5 closing: `tools/CURRENT_MILESTONE = M5`; coverage gate green for every M5 T requirement (RFC 014 and 015); `CHANGELOG.md` for 0.5.0; threat review of THR-01, THR-17, THR-20; the security audit of the project rules recorded in the package | CI green with the M5 gate |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | `html` module move | RFC 014 PR 2 |
| 2 | Property, switch, validation, documentation of the switch | — |
| 3 | `engine::scrape` with fixtures and unit tests | 1 |
| 4 | The `duckduckgo` driver, record, live check, `wiremock` tests | 2, 3 |
| 5 | Documentation and the milestone closing | 4, RFC 014 PR 3 |

## 3. Conventions that the reviewer will check

- The driver sends exactly the headers of the engine client of RFC 006
  plus what the record names; the test asserts the absence of browser
  headers, not only the presence of the standard User-Agent.
- Markers and parameters live in the record and in one `const` block of
  the driver, side by side; a change to one is a change to the other.
- Fixtures are hand-written in the page's shape; no copied page, no
  third-party text.
- The walk is iterative; parsing runs under the LIM-C5 permit on the
  blocking pool, as in `fetch`.
- The message for a refused table is the RFC's sentence, verbatim.
- What the implementer cannot verify against the live site (terms, robots
  status, parameters) is a question in the package with the evidence
  found, never a guess written as a fact into the record.

## 4. Tests

Named by requirement identifier: `req_eng_013`, `req_eng_014`,
`req_eng_015`, `req_test_018`, plus the fixtures' unit tests. The RFC's
list is the test plan. Tests live in `tests.rs` files beside the module.

## 5. Review package

`.git-exclude/reviews/015-scraping-drivers/README.md` with the gate
results, the admission record's open points, the threat review and the
security audit; it is also the M5 closing record before release 0.5.0.
