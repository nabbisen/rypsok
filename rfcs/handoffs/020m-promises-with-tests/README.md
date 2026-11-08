# Handoff 020m — Promises the stability page makes, each with a test

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3), wave 2.
Owner decisions **D-11** (reword to what the code holds) and **D-18**.
Findings DAT-5, DAT-6, DAT-7 (Medium); DAT-10 (Low).

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md), RFC 016 (the stable contract).
**Applicable requirements.** REQ-STATUS-*, REQ-MAIN-*, REQ-MCP-*, REQ-SEC-003.
**Rules.** As 020h; evidence to `.git-exclude/evidence/020m/`.
**Depends on.** 020k approved (it changed the tool description and the output schemas).

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | **DAT-5.** The stability page claims all public types of `rypsok-core` are `#[non_exhaustive]`; 33 of 101 structs and about 10 of 45 enums are. Reword the row to what holds: the **facade** (`Rypsok` and the calls named in `lib.rs`) and the **`model` types** are the frozen surface; the other public items of the crate are public for the adapter crate and are **outside** the promise until the owner's 1.0.0 decision (D-11). Put in the page a fenced list "Types marked `#[non_exhaustive]`" (generated from the source by you once, then kept true by the test below) and say that `Outcome`, `ErrorKind`, `CallError`, `EngineState` and `TimeRange` are deliberately exhaustive (RFC 016 amendment) | A gate test in `m0_gates.rs`: scan the `model` module and the facade's public types; the set of `pub struct/enum` that carry `#[non_exhaustive]` equals the list in `stability.md`; and every public type of `model` that is not on the list is on a second fenced list "deliberately exhaustive" in the page. Seen failing first (an item removed from the list) | `stability.md` |
| 2 | **DAT-6.** Add the three **input schemas**, the **descriptions** and the **annotations** returned by `tools/list` to the fixtures: one committed file `crates/rypsok/fixtures/schemas/tools_list.json` (the whole `tools/list` result for a fixed reference configuration with two languages and two categories), compared by a test, regenerated with the same `RYPSOK_UPDATE_FIXTURES=1` switch as the output schemas | `req_mcp_011_*` extended or a sibling test; failing first: change a parameter's `maximum` in a scratch edit and show the test fails | `stability.md` sentence ("every tool definition and every response schema") is now true; say so in the review request |
| 3 | **DAT-7.** A test that `serve --transport http` writes nothing to stdout: spawn the binary with stdout piped, wait until it logs `listening` on stderr, send one request, send SIGTERM (as `req_ops_002_sigterm_*` does), assert stdout is empty and the exit code is 0 | The test (it passes today; show it fails when you add a `println!` in a scratch edit, then remove it) | — |
| 4 | **DAT-10.** Remove `Deserialize` from `SearchResult` (and from any type whose doc says the boundary is the only way to build it; list them in the review request). Where something in the tree needs to read a result back (a test, the cache), say what and use the boundary or a private mirror | A `compile_fail` doc test: `serde_json::from_str::<SearchResult>` does not compile. If a use is found that cannot go, **stop and ask** instead of keeping the derive | `boundary.rs` doc; changelog Changed ("`SearchResult` no longer implements `Deserialize`: public item of `rypsok-core`") |
| 5 | **Changelog.** Changed (the stability page's surface; `SearchResult`); nothing else | — | `CHANGELOG.md` |

## Explicit non-change scope

No change of any public type's fields; no `#[non_exhaustive]` added to make
the old sentence true (that is a 1.0.0 decision); the scheduled/CI files.

## Prohibited shortcuts

A list in the page that the test does not read. Regenerating a fixture to make
a failing test pass without reading the diff (the diff goes in the review request).

## Required evidence and review request

The gates; failing-first output; the review request at
`.git-exclude/review-requests/020m-promises-with-tests/README.md`.
