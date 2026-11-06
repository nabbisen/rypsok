# Handoff 020s — Clean-up: inert code, time-dependent tests, drift

**Created 2026-11-07.** A fix handoff of the audit (RFC 020, D3), wave 3.
Findings TST-8 to TST-13, TST-15 (Low); owner decision D-9 for the limits
table rows. Clean code is part of the owner's philosophy: nothing in the tree
that does nothing, no test that passes for the wrong reason.

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md).
**Rules.** As 020q. Evidence to `.git-exclude/evidence/020s/`.
**Depends on.** 020q and 020r approved (they touch the same test files).

## Change scope

| # | Work |
|---|---|
| 1 | **TST-8, tests that depend on elapsed time.** The pipeline tests (`req_agg_002`, `req_agg_003`, `req_err_005`, `req_agg_009`, `req_agg_001`, `req_agg_006`, the `req_life_002`/`req_priv_003` sleeps) run with paused tokio time (`#[tokio::test(start_paused = true)]`) and exact assertions. The limiter tests (`req_cmp_002_retry_after_blocks_until_elapsed` and its kin) get a clock: `EngineLimiter` takes a clock type parameter defaulting to the real clock (governor's `FakeRelativeClock` in tests); no behaviour change. Re-run the starved-core series of 020d (10 runs, one core with busy loops) and report 0 failures |
| 2 | **TST-9, features.** `cargo test -p rypsok-core --no-default-features` compiles (gate the DuckDuckGo test import), no warnings with any single `engine-*` feature; CI gets a `cargo check` step for no features and for each single feature (`--locked`) |
| 3 | **TST-10, inert code.** Remove: `UserAgentFormat::Custom` and the field `EngineProperties.user_agent` (set to `Standard` by every driver, read by none); the six unused `EngineProperties::with_*` builders; `Rypsok::returned_urls`; the seven `Limits` fields that duplicate `config.service.http`/`fetch.ports` (the listener reads the config; make one source of each number); the `#[cfg(test)]` helpers outside test modules move into the test modules. **LIM-O2** (`CANCEL_MAX_MS`) is used: a test cancels a call and asserts that the engine task is dropped within it (the mock records the drop). List every removed public item in the changelog under Changed |
| 4 | **D-9, the limits table.** The eight rows with an "operator" range and no key become "fixed" (LIM-F9, F10, F12, F13, S11, C9, O2 and the others the 020c table listed, LIM-F11 is already fixed); **LIM-S12 and LIM-O1 get keys** (`search.connect_timeout_ms`, `service.shutdown_ms`, with the table's ranges, validated, in `configuration.md`, in `check`'s output, and used by the code). `tools/check-limits` passes |
| 5 | **TST-11.** A test over every `ToolError` constructor: message ≤ LIM-I4, no `{`/`}` left, no URL/host/address (if 020q item 6 already did this, say so and skip) |
| 6 | **TST-12.** `req_main_005_*` stays only if REQ-MAIN-005 stays `T` (020q item 7 proposes `I`); `req_fetch_003_*`'s `is_char_boundary(len)` assertion is replaced by one that can fail |
| 7 | **TST-13.** The hand-written page behind `examples/calls/web-fetch-*.json` goes into `examples/pages/` and `examples/README.md` says how the four answers are regenerated; regenerate them and say whether they changed |
| 8 | **TST-15, drift.** RFC amendments (appended) where the RFC is wrong and the code is right: RFC 003 (the `redirected` message wording), RFC 006 (the User-Agent property, removed by item 3), RFC 009 (`basic_auth_url` covers `user:password@`; `user@` is `credential_in_url`), RFC 011 (`tcp_keepalive` is not set: either set it on the agent client to 60 s, which is what the RFC meant, or amend; set it) |
| 9 | Changelog `[Unreleased]`: Changed (removed public items, the two new keys), Fixed (nothing user-visible otherwise) |

## Explicit non-change scope

No behaviour change except item 4's two keys and item 8's keepalive.

## Required evidence and review request

The gates; the starved-core series; the review request at
`.git-exclude/review-requests/020s-clean-up/README.md`.
