# Handoff — RFC 005: Content boundary

**RFC.** [`../../accepted/005-content-boundary.md`](../../accepted/005-content-boundary.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-SEC-001 to
-004, -015, -016, REQ-CTX-002 to -007, -010, REQ-TEST-007, -013),
`docs/src/maintainers/external-design.md` §9, `docs/src/maintainers/threat-register.md`
THR-01 to THR-04, `ROADMAP.md` M1.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 003 PR 1 (the types). Wiring into the pipeline belongs
to the RFC 007 handoff.
**Milestone.** M1 (`clean_text`); `clean_content` is M3 and not built here.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `rypsok-core::boundary::clean_text` with the five steps of RFC 005 in order, returning the cleaned text and the counts for notices | Unit tests per step and for the order (an entity that decodes to a control character is removed, so decoding precedes removal) |
| 2 | The removed-character table as one constant with the Unicode categories and ranges of RFC 005 | A test per range with one code point inside and one outside |
| 3 | Bounding at LIM-S9 and LIM-S10 with the word-boundary rule | Tests at the bound, one below, one above, and with multi-byte text (Japanese, emoji, combining marks) |
| 4 | Instruction-like pattern list and matching; `search.drop_flagged_results` honored | Tests for each phrase in three casings and for the drop path with count |
| 5 | Builders so that `SearchResult` external fields are only settable through the boundary | A compile-fail test (`trybuild`) or a visibility test that direct construction outside the crate is impossible |
| 6 | Hostile corpus entries of RFC 005 under `crates/rypsok-core/fixtures/hostile/` with a `README.md` line each | Corpus test runs every entry through `clean_text` and the JSON rendering and asserts the envelope invariants (REQ-TEST-007) |
| 7 | Property tests: idempotence, bound, absence of removed code points, JSON round-trip | `proptest` suite passes with the seed recorded in CI |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | `clean_text`, character table, bounding, unit tests | RFC 003 PR 1 |
| 2 | Patterns, drop path, builders | 1 |
| 3 | Corpus entries and property tests | 2 |

## 3. Conventions that the reviewer will check

- Visible text is never rewritten beyond the five steps; no "sanitization"
  of words, no markdown escaping, no quoting.
- The dependencies used for HTML entity decoding and Unicode normalization
  are on the `deny.toml` allow list and have no network or unsafe code
  (a `cargo geiger` or manual note in the PR).
- Every notice count is exact, not capped.

## 4. Review request package

Hand back `.git-exclude/reviews/005-content-boundary/README.md` with links
to this handoff and RFC 005, the PR list, the corpus test output, the
property test seed, and any deviation as a question.
