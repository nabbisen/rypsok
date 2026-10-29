# Handoff 019c — Release binaries, the missing gates, the book's dead link

**Revision 2 (2026-10-29)**, after the independent review (`arch-002-review-019`,
M5) and the owner's decision D-C (option 2). Where this revision differs
from revision 1, this one holds.

**Task title.** Attach binaries to a release the owner published, run
every gate in CI, and fix the book's dead link (RFC 019 D7, D8, D9 as
amended).
**RFC.** [`../../accepted/019-first-independent-review.md`](../../accepted/019-first-independent-review.md),
including its amendment of 2026-10-29.
**Applicable requirements.** REQ-DEP-005, REQ-RUST-002, REQ-TEST-012,
REQ-SEC-020, REQ-DOC-001. **Source findings.** `arch-001-prepare` U4, U5,
C1, C4; `dev-001-prepare` D1, D2, D5; `arch-002-review-019` M5, D-C.
**Rules.** As 019a. **Depends on.** Nothing. **May start at once:** items
2, 3, 4; item 1 starts with this revision. Item 2 and 019b item 3 both
touch the gate tests: one developer, or 019b item 3 first.
**Purpose.** The sentence "binaries for Linux, macOS and Windows are
attached to each release" becomes true without the machine ever
publishing a release; two gates the packages called gates run in CI; the
published book has no dead link.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | `.github/workflows/release.yml`: (a) `build` jobs (matrix: `x86_64-unknown-linux-musl` on ubuntu with `musl-tools` installed, `aarch64-apple-darwin` on macos, `x86_64-pc-windows-msvc` on windows) that build `rypsok` in release with `--locked`, package `rypsok-<version>-<target>.tar.gz` (`.zip` on Windows) with `LICENSE` and `NOTICE`, and **always upload workflow artifacts**; (b) a `checksums` job that writes `SHA256SUMS` over the archives; (c) an `attach` job, the **only** job with `permissions: contents: write`, that uploads the archives and `SHA256SUMS` to the GitHub release of the tag **only if that release exists** (`gh release view` first), with `--clobber` **not** used so an existing asset fails visibly, and otherwise writes a notice in the run summary and exits 0; (d) triggers: the tag push as today, plus `workflow_dispatch` with an input `tag` validated against `^[0-9]+\.[0-9]+\.[0-9]+$` and passed through an environment variable, never interpolated into a script, so the attach can be re-run after the owner publishes; (e) `build_only: true` dispatch input that skips the attach; (f) the image job unchanged; every action pinned by hash | the pinned-actions gate test green; **a build-only dispatch on `main` run green and recorded in the review request before 0.7.1** (artifacts for the three targets present, `SHA256SUMS` present) | getting started: "Binaries for Linux x86_64, macOS arm64 and Windows x86_64 are attached to each release; `SHA256SUMS` proves a download is intact, not where it came from" (REQ-RUST-002 names x86_64 and aarch64 across the three systems: the other cells are an M8 item); the deployment page: one line on verifying `SHA256SUMS` |
| 2 | `ci.yml` rules job: `tools/check-limits` after the coverage step; `cargo deny check advisories bans licenses sources`; `req_test_012_ci_workflow_lists_every_gate` extended to require both | the gate test | RFC 002's gate table: an amendment line dated with this handoff |
| 3 | `docs/src/SUMMARY.md`: `[DuckDuckGo](./maintainers/engines/duckduckgo.md)` under the engine records; a gates test `req_doc_001_every_page_is_in_the_summary` that walks `SUMMARY.md`'s links and asserts every `.md` under `docs/src` (except `SUMMARY.md`) is linked | the new test; `mdbook build` green | — |
| 4 | `deny.toml`: remove the two allowed-but-unused licenses or keep them with a comment naming why; the review request says which | `cargo deny check` without the two warnings | — |

## Explicit non-change scope

The container image job; the semver job; the pinned actions' versions
and the float of `mdbook-version: latest` and the toolchains (M8); the
release itself (the owner publishes; the original architect tags).

## Prohibited shortcuts

No `gh release create` anywhere in the workflow; no unpinned action; no
`continue-on-error` on the build jobs; no hand-uploaded binaries; no
asset overwrite.

## Boundaries

Workflows, `deny.toml`, `docs/src/SUMMARY.md`, the gates test; no source
change in the crates beyond the two tests.

## Known risks

The first real attach happens at 0.7.1 after the owner publishes the
release; the dispatch path exists for exactly that order. The build-only
dry run proves the build matrix before.

## Required evidence and review request

The gates; the pinned-actions test; the recorded dry run; the review
request at `.git-exclude/review-requests/019c-release-ci-records/README.md`.
