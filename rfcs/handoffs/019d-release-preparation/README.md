# Handoff 019d — Release preparation for 0.7.1

**Created 2026-10-29** by the RFC 019 amendment (the independent review's
M6 and the owner's decision D-B, option 3): the preparation of a release
is a code change and goes through the same implement → review path as
any other.

**Task title.** Prepare the tree for release 0.7.1 after 019a, 019b and
019c are approved.
**RFC.** [`../../accepted/019-first-independent-review.md`](../../accepted/019-first-independent-review.md),
including its amendment of 2026-10-29.
**Applicable requirements.** REQ-MAIN-006 (every contract change
announced), REQ-DEP-005. **Source findings.** `arch-001-prepare` B16, P4;
`arch-002-review-019` M6, D-B.
**Rules.** As 019a. **Depends on.** The approval of 019a, 019b and 019c,
and the original architect's RFC amendment commit for B4, B5, B9, B11,
B14 (landed before this handoff starts; verify it is on `main`).
**Purpose.** The changelog describes what 0.7.1 delivers, in the words
the pages use; the version is bumped; nothing else changes.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | `CHANGELOG.md`: the `[Unreleased]` section becomes `## [0.7.1] — <date left as YYYY-MM-DD placeholder for the release day> — Corrections from the first independent review`, with `### Added` (the `max_sessions` key and LIM-H6; binaries attached to releases; the fuzz target `clean_decoded_text`; `check-limits` and `bans` in CI), `### Changed` (the RFCs moved to `done/`; `clean_text` renamed `clean_markup_text`; a refused handshake answers 503 with `Retry-After`; the documented statuses 400/403/406 named with their owner), `### Fixed` (one line each: the trailing-dot bypass; the unreachable site reported as a robots refusal and the refused name now answering like an unreachable one; titles keeping `<T>`; the compare's length aliasing; the documented `curl`; the DuckDuckGo link; the binaries sentence; the key-correction wording; the `IPAddressAllow` placeholder) and, under `### Changed`, the two cleaner changes 0.6.0 and 0.7.0 omitted (B16): "`<` followed by a non-ASCII letter is kept as text (2026-10-27)" and "control characters are removed before the markup step (2026-10-28)"; the comparison links | — | the lines are the documentation |
| 2 | `Cargo.toml`: workspace version `0.7.1` and the `rypsok-core` dependency version; `cargo update --workspace --offline`; `cargo publish --workspace --dry-run` green | `cargo test --workspace`; the dry run's output in the review request | — |
| 3 | `tools/CURRENT_MILESTONE` stays `M7` (0.7.1 is a patch within M7); verify and say so | the coverage gate | — |
| 4 | A consistency pass, no edits beyond what it finds: every page that names an HTTP status names the delivered one; the tools page, the troubleshooting page and "Reading a response" agree on `unavailable`/`network` for an unreachable name; `examples/README.md` still describes the sample answers truly after 019a (the sample answers themselves are unchanged unless a regenerated answer differs, in which case regenerate by the documented method and note it) | `tests/examples.rs` green | findings fixed in place and listed in the review request |

## Explicit non-change scope

No source change; no tag; no publish; no GitHub release (the original
architect does those on the owner's authorization after the reviewer's
Go); no edit to an RFC.

## Prohibited shortcuts

No changelog line that paraphrases a handoff instead of what landed; no
version bump before the three approvals are recorded.

## Required evidence and review request

The gates; the dry run; the review request at
`.git-exclude/review-requests/019d-release-preparation/README.md` naming
the three approved review requests it builds on. The reviewer writes
the 0.7.1 release-readiness report on this request.
