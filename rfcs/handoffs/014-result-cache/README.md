# Handoff — RFC 014: Result cache

**RFC.** [`../../done/014-result-cache.md`](../../done/014-result-cache.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-CACHE-001
to -007, REQ-CMP-007, REQ-TEST-010, REQ-PRIV-003, REQ-ENG-010, REQ-OBS-001),
`docs/src/maintainers/external-design.md` §3.1, §5, §12, §16,
`docs/src/maintainers/limits-and-defaults.md` LIM-K1 to LIM-K4,
`docs/src/maintainers/threat-register.md` THR-17, `ROADMAP.md` M5.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** Release 0.4.0 and the M4 review decisions (`f869eaf`).
**Milestone.** M5 (first part; RFC 015 is the second). The milestone
closes with RFC 015's last deliverable.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `CallerId` in `rypsok_core::model::ids` (re-exported by the binary); `Rypsok::search_as(caller, req, cancel)`; the adapter passes the gate's caller | Existing protocol and listener tests unchanged in outcome; `local` over stdio |
| 2 | `[cache]` table with validation and the page row that names query retention; LIM-K4 constant | Page test of RFC 002 green; `check-limits` 47 rows; the `warn` `cache_without_storable_engines` |
| 3 | `cache::ResultCache`: exact keys, expiry on the tokio clock, LRU eviction, entry bound, poison handling | Unit tests: hit/miss/expiry/eviction/too large/poisoned, all without I/O |
| 4 | Pipeline integration: the lookup and store table of the RFC, `Ended::Cached`, the `from_cache` notice, `engine_answered` with `cached = true`, the counters | `req_cache_00x` and `req_cmp_007` tests with `wiremock` (the RFC's list); REQ-TEST-010 named by a test |
| 5 | Documentation: tools page (`from_cache`, age up to `ttl_s`), configuration page (`[cache]`, retention), integration guide (restarting clears the cache; the notice) | `mdbook build` green; `CHANGELOG.md` Unreleased for 0.5.0 |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | `CallerId` move, `search_as`, adapter wiring | — |
| 2 | `[cache]` configuration, LIM-K4, `ResultCache` with unit tests | — |
| 3 | Pipeline integration, notice, counters, `wiremock` tests, documentation | 1, 2 |

## 3. Conventions that the reviewer will check

- The lock is never held across an `await`; the workspace lint denies it,
  and the reviewer reads the code anyway.
- No `unwrap` on the lock: a poisoned lock is the `cache_disabled` path.
- The fetch side has no path to the cache: the type is private to the
  search side; `tools/check-core-dependencies` is unchanged.
- The query stored in a key is the text after the privacy check; a
  `Warned` verdict bypasses the cache entirely.
- Time is the tokio clock (`tokio::time::Instant`); tests pause it.
- The notice vocabulary is extended in `model.rs` and in external design
  §3.1 in the same commit.

## 4. Tests

Named by requirement identifier so the coverage gate finds them:
`req_cache_001` to `req_cache_007`, `req_cmp_007`, `req_test_010`. The
RFC's list is the test plan; each bullet is at least one test. Tests live
in `tests.rs` files beside the module, never inline.

## 5. Review package

`.git-exclude/reviews/014-result-cache/README.md` with the gate results,
the memory bound stated as LIM-K3 × LIM-K4 at the configured values, the
threat review of THR-17 and the security audit of the project rules.
Deviations from the RFC are questions in the package, not redesigns.
