# Handoff 020q — Close the verification debt on the security paths

**Created 2026-11-07.** A fix handoff of the audit (RFC 020, D3), wave 3.
Owner decisions **D-18** (close in 0.9.0 the debt on security paths and
behind the Medium findings; the rest is a published ledger) and **D-19** (a
committed exclusion list for mutation testing; the job itself runs in the
release checks of 020w, D-23). Findings TST-1, TST-5,
TST-6, TST-7 (Medium); the `partial` and `names-only` rows of the audit's
per-requirement table (`.git-exclude/review-requests/020d-tests-and-code-audit/requirements-table.md`)
named below.

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md), RFC 002 (test strategy).
**Rules.** As the earlier fix handoffs: tests first and shown failing where a
behaviour is missing (a test that passes at once must say so and say why it is
still worth having); commit your own work; push only after approval; evidence
to `.git-exclude/evidence/020q/`; relative paths with `cd`. **Production code
changes only where a test seam is unavoidable**, and then stop and say so in
the review request before going on. A token-shaped test string is built at
run time (a bare prefix is fine; a prefix with its body is not).
**Depends on.** Waves 1 and 2 (they changed several of these paths).

## Change scope

| # | Work | Requirement rows closed |
|---|---|---|
| 1 | **Re-run mutation testing first**, on the current tree: `egress/policy.rs`, `privacy.rs`, `boundary.rs`, `fetch/robots.rs`, `search/rank.rs`, `search/normalize.rs`, `cache.rs`, `mcp/http.rs`, `mcp/http/sessions.rs`, `mcp/http/detect.rs`, `mcp/rate.rs` (`cargo mutants --jobs 2`, `df /tmp` checked before; unit **and** integration tests this time: `-- --workspace` scoped to the crate). Report per module caught / missed / unviable / timeout and every missed mutant. That list replaces the audit's (waves 1 and 2 killed some and added code) | — |
| 2 | **A test per missed mutant** that a test can catch (audit marks **T**): for each, the smallest test that fails with the mutant. Equivalent mutants (**E**) are written to `mutants.toml`'s exclusion list (item 8) with one line of reason each, never silently | TST-7 |
| 3 | **TST-5, REQ-FETCH-010: a successful `http`→`https` upgrade.** A TLS loopback server with a certificate generated **at test time** with `rcgen` (a dev-dependency of `rypsok-core`; run `cargo deny` and show the lockfile diff), trusted by the fetch client through a test-only root store. The upgraded fetch answers with the `https` URL in `url`. If the client has no way to add a root without a production seam, stop and ask | REQ-FETCH-010 |
| 4 | **TST-6: what a fetch sends.** A loopback server that records the request line and every header: `web_fetch` sends `GET`, only the headers rypsok sets (name them from the code: `User-Agent`, `Accept`, `Accept-Encoding`, `Host`), no `Cookie`, no `Authorization`, and a `Set-Cookie` in one answer is not sent back in the next fetch to the same host. Run the same with `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` set to a closed loopback port: the fetch still reaches the server directly | REQ-FETCH-007, REQ-SEC-007, REQ-SEC-009, REQ-RES-008 |
| 5 | **REQ-SEC-011: the engine client really times out.** A loopback engine that accepts and never answers: the engine's call ends at LIM-S5 (with the test's shortened value) as a `timeout`, not by the overall deadline | REQ-SEC-011 |
| 6 | **Security statements asserted for one path only.** (a) REQ-SEC-001/REQ-ERR-004: a hostile provider title and snippet (instruction text, markup, a URL, an address) never appear in an error message or a log line at the default level (capture the log); every error constructor's message is at most LIM-I4 and contains no URL, host or address (a table over all constructors). (b) REQ-SEC-014: a panic inside extraction is contained and logged `extraction_panic` (inject one through a test-only page that the extractor cannot handle, or a test-only hook; if a hook is needed in production code, stop and ask). (c) REQ-SEC-016: the provider-text debug event is asserted by capturing the event, not by searching the source file. (d) REQ-PRIV-003/004: after a search and a fetch, the counters, the health table, the cache keys' `Debug` and the log at the default level contain no query text, no URL path and no query string. (e) REQ-SEC-005: a gate test that every `.send()` in non-test code of `rypsok-core` is on a client built by `egress.rs` (a source scan, as the lock gate does) | REQ-SEC-001, REQ-ERR-004, REQ-SEC-014, REQ-SEC-016, REQ-PRIV-003, REQ-PRIV-004, REQ-SEC-005 |
| 7 | **The five `names-only` rows.** REQ-ENG-002 (concurrent use: one driver instance called from eight tasks at once against a mock, all answers correct); REQ-PAGE-005 (rename or add the media-type test under its id); REQ-CACHE-003 (read RFC 014: the engine set, configuration version and policy live in the key or the process; if the requirement text is the drift, propose the corrected sentence in the review request instead of a test; the architect edits requirements); REQ-OBS-005 (handled by 020r; leave); REQ-MAIN-005 (a structural requirement: propose changing its verification method from T to I in the review request) | REQ-ENG-002, REQ-PAGE-005, REQ-CACHE-003, REQ-MAIN-005 |
| 8 | **D-19 as amended by D-23 (2026-11-07): the exclusion list only, no job.** Write `.cargo/mutants.toml` (the place cargo-mutants reads by default; amended in the review, 2026-11-07) with the equivalent mutants of item 2, each with one line of reason, never silently. Name the `cargo-mutants` version you used in the review request: handoff 020w pins it in the release-check matrix job, which replaces the weekly job. **Do not add a job to any workflow here.** The baseline is that a run over the six security modules (`egress/policy` with `policy/`, `privacy`, `boundary`, `fetch/robots`, `mcp/http` with `http/detect`, `mcp/http/sessions`) with `mutants.toml` misses nothing. Show it in the evidence (`--jobs 2`, `df` checked first) | — |
| 9 | **The ledger.** A file `docs/src/maintainers/verification-ledger.md` (in `SUMMARY.md`) that lists every `T` requirement still `partial` after this handoff with the one-line missing assertion from the audit table, so the debt is public and is closed before 1.0.0 (D-18) | — |

## Explicit non-change scope

No behaviour change. No new public item except what a test seam needs (stop
and ask first). REQ-OBS-005 is 020r's. Timing hygiene (paused time) is 020s's.

## Required evidence and review request

The gates; the mutation reports before and after; failing-first output for
items 3 to 7; the baseline run over the six modules with `mutants.toml` for
item 8; the review request at
`.git-exclude/review-requests/020q-verification-debt/README.md`.
