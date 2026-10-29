# Handoff 020d — Audit of test adequacy, dead code and drift from the RFCs

**Created 2026-10-29.** An audit handoff: findings only, no source change.

**Task title.** Show which requirements are verified by their tests and
which are only named by them; find dead and risky code; find where the code
differs from what the done RFCs say; report in the format of RFC 020 D1.
**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md),
D1 and D4.
**Applicable requirements.** REQ-TEST-*, REQ-MAIN-*, and every requirement
with verification method `T`. **Seeds.** `arch-001-prepare` S9, B14;
`M8-DEFERRED.md` row "Tests that depend on a real-time window" (the 008
survey: `engine/limiter/tests.rs`, `search/pipeline/tests.rs`).
**Rules.** As 020b. **Depends on.** 020a approved (the texts are the baseline).
**Purpose.** The coverage gate passes when a function name **contains** a
requirement id. That proves a test was named for the requirement, not that
it verifies it. This audit measures the difference.

## Change scope (what to audit)

| # | Area | Questions the report answers |
|---|---|---|
| 1 | **Test adequacy per requirement.** All `T` requirements up to M7 (159) | A table, one row per requirement: id; the test(s) that name it; **verdict** `verifies` (the test fails if the requirement is broken in the way the text describes), `partial` (verifies part; say which part is not covered), `names-only` (the assertion does not depend on the behavior), `none`; one line of why. Read every test body; do not judge by name. Where the verdict is `partial` or worse, write the missing assertion in one sentence. Work in batches by group (REQ-SEC, REQ-FETCH, …) and report per batch as you go in the scratchpad notes |
| 2 | **Mutation testing.** Where judgment is not enough | Run `cargo mutants` (installed) on these modules, one at a time, with `--in-place` forbidden (use its default copy; do not run it on your working tree): `egress/policy.rs`, `privacy.rs`, `boundary.rs`, `fetch/robots.rs`, `search/aggregate*` (the rank fusion and dedup), `mcp/http.rs`'s gate and `sessions.rs`, `mcp/rate.rs`, `cache.rs`. Report per module: mutants caught / missed / unviable / timeout, and for each **missed** mutant in a security-relevant function the function, the mutation and whether a test could catch it. Use a time budget per module (state it) and run with `--jobs` fitting this machine; if a module cannot finish, say what was run |
| 3 | **Tests that depend on elapsed time.** Flaky in principle | The 008 survey named `engine/limiter/tests.rs` (80 ms block; 70 ms sleep) and `search/pipeline/tests.rs` (`req_agg_002` 380–500 ms window; `req_perf_004` 50 ms per call). For each test of that kind (re-derive the list by reading, not only by the grep of 008): the margin, and whether tokio's paused time or a controllable clock would make it exact; the cost in production code. Run the suite 10 times on one starved core (`taskset -c 0` with busy loops, as 008 did) and report failures per test |
| 4 | **Dead and unreachable code.** | `cargo +nightly udeps` is not installed; do this: (a) every `pub` item of `rypsok-core` never used outside its module and not part of the stability page's frozen list; (b) every `#[allow(...)]` and `#[cfg(...)]` in non-test code, with its reason or "none"; (c) every function only used by tests (`#[cfg(test)]` helpers outside test modules, `pub` for tests); (d) unused config keys (a key parsed and never read), unused error variants (never constructed), unused LIM constants; (e) TODO/FIXME/XXX/HACK. Report each with `path:line` |
| 5 | **Risky constructs in non-test code.** | An inventory (counts by file, then each site in a table for the request path): `unwrap()`, `expect(`, `panic!`, `unreachable!`, `unsafe`, indexing that can panic (`[..]` on input-derived data), `as` casts between integer widths (S3 was one), `std::sync::Mutex` held across an `.await` (clippy lint status), `Arc<Mutex<…>>` with unbounded growth. For each site on the request path: what input reaches it, the worst result. The `expect("… lock")` sites (S9) are covered with 020b area 7; refer, do not repeat |
| 6 | **Drift from the done RFCs.** | For RFCs 003, 004, 005, 006, 007, 008, 009, 010, 011, 012, 013, 014 (015 to 019 were read by the first review and 0.8.0): extract every normative sentence of "Design" and "Tests" and "Acceptance" that makes a checkable claim (a number, a status, a name, an order, "never", "always") and check it against the code, the pages and the tests; 25 claims per RFC is enough if you pick the ones a user or an attacker depends on, and say how you picked. Table: RFC, claim, code or test that keeps it, verdict `holds`/`differs`/`unverified`. B14 (RFC 012 says a 4xx/5xx body is read to 64 KiB and discarded; the code returns before reading; RFC 015 cites "LIM-F6" for a redirect count) is the shape of what to find |
| 7 | **Fixtures and examples.** | Are the sample answers in `examples/` produced by the code today (regenerate to the scratchpad by the documented method and diff)? Do the fuzz targets' corpora run in CI (`scheduled.yml`) — how long per target, how many targets cover a parser that reads untrusted bytes (list every such parser; mark which has a target)? |

## Explicit non-change scope

No source file, test or document is changed. `cargo mutants` works on a copy;
check `git status` is clean when you finish.

## Prohibited shortcuts

A verdict from a test's name. `verifies` for a test that asserts only
`is_ok()`. A mutation run reported as a count without the missed mutants.

## Required evidence and review request

`.git-exclude/review-requests/020d-tests-and-code-audit/README.md` in the
format of RFC 020 D1: findings (`TST-n`), then the tables of areas 1 (one row
per requirement; may be an appendix file `requirements-table.md` next to
the README), 2, 3, 6, then the four closing lists. The README names the
count of each verdict.
