# Handoff — RFC 008: URL comparison keys, deduplication, rank fusion

**RFC.** [`../../accepted/008-urls-dedup-ranking.md`](../../accepted/008-urls-dedup-ranking.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-RANK-001 to
-007, REQ-ARCH-004, REQ-CTX-001), `docs/src/maintainers/external-design.md`
§3.1, §6, `ROADMAP.md` M1.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** RFC 003 PR 1 and RFC 006 PR 1 (engine identifiers and
preference). Independent of RFC 007 until the wiring PR.
**Milestone.** M1.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `search::normalize`: returned URL rules (parse, refuse, tracking removal, IDNA host) and the tracking list constant | Table-driven tests with at least 40 rows covering each rule, IDNA, percent-encoding, ports, fragments; refused URLs produce the `results_dropped` reasons of the external design |
| 2 | `search::normalize::comparison_key` | Table-driven tests; property test of idempotence; the key is a private type that cannot be placed in a `SearchResult` |
| 3 | `search::dedup`: grouping and merge with the best-rank rule and `engines` list | Tests for merge selection, tie by preference, and `search.engine_attribution` on and off plus `attribution_required` |
| 4 | `search::rank`: reciprocal rank fusion in integers, total order, cut to `max_results` | Hand-computed score tests; a test that two engines at rank 3 beat one engine at rank 1 is stated with its numbers in the test name's doc comment |
| 5 | Determinism: fusion over a shuffled engine order gives the same output when the selection order is fixed | Test shared with RFC 007's determinism suite |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | Normalization and comparison key with tables | RFC 003 PR 1 |
| 2 | Deduplication, fusion, order | 1, RFC 006 PR 1 |

## 3. Conventions that the reviewer will check

- No floating point in `rank`.
- The returned URL differs from the driver's URL only by tracking parameters
  and the ASCII host; a test asserts this for the table.
- The tracking list and `k` live in one constant each with a comment.

## 4. Review request package

Hand back `.git-exclude/reviews/008-urls-dedup-ranking/README.md` with
links to this handoff and RFC 008, the PR list, the table test count, and
any deviation as a question.
