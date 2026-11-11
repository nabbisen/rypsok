# Handoff 021a — Enforced call maxima, shown by `check`; definition changes announced and checked

**Created 2026-11-11.** **RFC.** [`../../accepted/021-call-maxima-and-definition-changes.md`](../../accepted/021-call-maxima-and-definition-changes.md) (D1 to D3).
**Rules.** As in the audit: tests first and shown failing; commit your own work; push only after approval; evidence goes to `.git-exclude/evidence/021a/`; relative `cd`. Two rules apply:
- **The per-change rule** (`.git-exclude/rules/rypsok-checks-per-change.md`). Run mutation on every module you change, even outside its list.
- **The new rule on tool-definition changes** (`.git-exclude/rules/rypsok-tool-definition-changes.md`).

**Depends on.** Nothing. The work lands on `main` and is released with the next minor.

## Change scope

| # | Change | Test (fails before) |
|---|---|---|
| 1 | **The bounding function (D1).** In `rypsok-core`, one function that runs a future until a deadline and answers a given error after it, for example `async fn within<T>(deadline: Instant, error: impl FnOnce() -> ToolError, call: impl Future<Output = Result<T, ToolError>>) -> Result<T, ToolError>`. Its shape is yours: one function, no clock of its own, and tokio's `Instant` as in the limiter. `contained` (020q) stays as it is; say in the review request whether it can be built on the new function without a behavior change | A unit test with a future that never ends: paused time, the error at exactly the deadline, and the future dropped (a drop flag) |
| 2 | **LIM-S7 enforced (D1).** `Rypsok::search_as` measures the start on entry and bounds the whole call by `search_call_max_ms()`. On expiry the answer is the search's existing deadline error: `unavailable`, reason `deadline`, and the message the search uses for a deadline. Find it and name it in the review request. No new kind, reason or notice | A paused-time test that a search whose post-processing never ends answers the deadline error at LIM-S7. Use a test engine, not a production hook; if that is impossible, stop and ask. A test named `req_agg_003_…` that a `web_search` call never lasts longer than LIM-S7, over the existing slow and hanging engine cases. A test named `req_agg_003_…` that LIM-S5 + LIM-S6 > LIM-S4 is refused at startup, so the ledger row closes |
| 3 | **LIM-F5 enforced end to end (D1).** `Rypsok::fetch_as` bounds the whole call by `fetch_call_max_ms()` from entry. The parse step keeps its own bound. On expiry: the existing `fetch_deadline` error | A paused-time test that a fetch call never lasts longer than LIM-F5, including a call whose permits are held by others |
| 4 | **`check` prints the maxima (D2).** A line after the rate-limits line: `rypsok: tool call maxima in ms: web_search=<LIM-S7> web_fetch=<LIM-F5>; engine_status reads memory only`, with the effective values. Update `configuration.md`: the sample output, and one sentence. The sentence says that `check`'s text is for people and may change in a minor version (as the stability page says), and that for a program the stable source is its own configuration plus the fixed margins LIM-S7 and LIM-F5 in the limits table | `check` output tests: the defaults, and a changed `search.deadline_ms` and `fetch.deadline_ms` |
| 5 | **The changelog rule, in the documents (D3).** `releasing.md`, in step 2 (the release commit): "When a tool definition changed since the last release, the section has a `### Tool definitions` heading, first under the version, with one line per change: the tool, the field (name, description, input schema, output schema, annotations, or the server instructions) and what changed." The CHANGELOG preamble gets one sentence that says so | `req_doc_001` |
| 6 | **The server instructions pinned.** If the instructions text is not pinned by a fixture yet (`req_mcp_006_…` reads it, so check what it asserts), pin it in `crates/rypsok/fixtures/schemas/instructions.txt` with a test, so that D3's check covers it | the fixture test, failing on a changed instruction |
| 7 | **The release-check job (D3).** A job `tool-definitions` in `release-checks.yml`. It checks out with the tags (`fetch-depth: 0`) and finds the latest release tag reachable from `HEAD` (`git describe --tags --abbrev=0 --match '[0-9]*.[0-9]*.[0-9]*'`). It compares `tools_list.json` and the instructions fixture between that tag and `HEAD`. Compare the files as committed: they hold the definitions without the server version; confirm that. When they differ, it requires the newest `## [` section of `CHANGELOG.md` to contain `### Tool definitions`, and fails with a message naming the changed files when it does not. A tag older than the fixture (0.8.0 had none) counts as "differ". `permissions: contents: read`, a timeout, no `continue-on-error`. **The cap stays:** make it `needs: reproducible-compare` (or another job that ends early), so the peak of at most 12 jobs holds, and the cap test stays true | Gate tests: the job exists with these properties; its script compares those two paths against the latest tag and checks the heading; the cap test |
| 8 | **The ledger.** Strike REQ-AGG-003's row, naming the tests. If item 3's test also closes REQ-PAGE-008's "shall not extend a `web_fetch` call beyond LIM-F5", say so | — |
| 9 | **Changelog `[Unreleased]`.** Under Changed: "A `web_search` call ends at LIM-S7 and a `web_fetch` call at LIM-F5, measured from its start, with the existing deadline error (the bounds were documented; they are now enforced over the whole call)". Under Added: "`rypsok check` prints each tool's maximum call time". **No `### Tool definitions` heading:** this handoff changes no definition. If you find that it does, it needs the heading | — |

## Explicit non-change scope

- No new error kind, reason or notice.
- No change to the limits' values.
- No machine-readable form of `check`.
- No change to the tool definitions.

## Required evidence and review request

1. The gates.
2. Failing first.
3. Mutation of every changed module.
4. The review request at `.git-exclude/review-requests/021a-call-maxima-and-definition-changes/README.md`.

After the push, the architect dispatches the release checks once, to see the new job run against `0.9.0`. The fixture is unchanged since 0.9.0, so the job must pass without the heading. This is the acceptance of item 7.
