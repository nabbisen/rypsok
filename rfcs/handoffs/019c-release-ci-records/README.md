# Handoff 019c — Release binaries, the missing gates, the book's dead link

**Revision 3 (2026-10-29)**, after the re-review (`arch-002-review-019/RE-REVIEW.md`,
N4, N9) and the second amendment of RFC 019. Revision 2 followed the
independent review (M5) and the owner's decision D-C (option 2). Where a
revision differs from the one before, the later one holds; revision 3
changes item 1 (the re-run is made from the tag, not by an input) and adds
the release procedure page.

**Task title.** Attach binaries to a release the owner published, run
every gate in CI, and fix the book's dead link (RFC 019 D7, D8, D9 as
amended).
**RFC.** [`../../done/019-first-independent-review.md`](../../done/019-first-independent-review.md),
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
| 1 | `.github/workflows/release.yml`: (a) `build` jobs (matrix: `x86_64-unknown-linux-musl` on ubuntu with `musl-tools` installed, `aarch64-apple-darwin` on macos, `x86_64-pc-windows-msvc` on windows) that build `rypsok` in release with `--locked`, package `rypsok-<version>-<target>.tar.gz` (`.zip` on Windows) with `LICENSE` and `NOTICE`, and **always upload workflow artifacts**; (b) a `checksums` job that writes `SHA256SUMS` over the archives; (c) an `attach` job, the **only** job with `permissions: contents: write`, with `if: github.ref_type == 'tag'`: it first fails unless the workspace `version` in `Cargo.toml` equals the tag, then uploads the archives and `SHA256SUMS` to the GitHub release of the tag **only if that release exists** (`gh release view` first), with `--clobber` **not** used so an existing asset fails visibly; if the release does not exist it writes a notice (annotation and run summary) that says exactly what to do: "the release for tag X does not exist; publish it, then run this workflow again from tag X (Actions, release, Run workflow, Use workflow from: Tag X)" and exits 0; (d) triggers: the tag push as today, plus `workflow_dispatch` **without inputs**: GitHub's "Use workflow from" accepts a tag, so the attach is re-run from the tag, on the tag's own commit, after the owner publishes; a dispatch from a branch builds and uploads workflow artifacts only (the attach job's `if` is false), and that is the dry run; the tag name is read from `GITHUB_REF_NAME` through an environment variable and matched against `^[0-9]+\.[0-9]+\.[0-9]+$` before use, never interpolated into a script; (e) the image job runs only when `github.ref_type == 'tag'` (a branch dispatch would tag an image `rypsok:main`) and is otherwise unchanged; (f) every action pinned by hash | the pinned-actions gate test green; **a dispatch from `main` (the dry run) run green and recorded in the review request with its run URL before 0.8.0** (artifacts for the three targets present, `SHA256SUMS` present, the attach job shown as skipped); the attach path itself cannot run before a real tag, and the review request says so | getting started: "Binaries for Linux x86_64, macOS arm64 and Windows x86_64 are attached to each release; `SHA256SUMS` proves a download is intact, not where it came from" (REQ-RUST-002 names x86_64 and aarch64 across the three systems: the other cells are an M8 item); the deployment page: one line on verifying `SHA256SUMS`; a new maintainers page `docs/src/maintainers/releasing.md` (in `SUMMARY.md`) with the procedure: (1) preconditions: gates green, the reviewer's Go, the owner's authorization; (2) tag `X.Y.Z` (no `v`) on the release commit and push it; (3) the owner publishes the release from the tag, or publishes first (creating the tag starts the workflow, whose attach job finds the release); (4) read the run: if the attach job wrote the notice, run the workflow again from the tag (the Actions page's "Use workflow from: Tag X", or `gh workflow run release.yml --ref X`); (5) check the release page lists the three archives and `SHA256SUMS`; (6) nothing is overwritten: to replace an asset, delete it by hand and say so in the release notes |
| 2 | `ci.yml` rules job: `tools/check-limits` after the coverage step; `cargo deny check advisories bans licenses sources`; `req_test_012_ci_workflow_lists_every_gate` extended to require both | the gate test | RFC 002's gate table: an amendment line dated with this handoff |
| 3 | `docs/src/SUMMARY.md`: `[DuckDuckGo](./maintainers/engines/duckduckgo.md)` under the engine records; a gates test `req_doc_001_every_page_is_in_the_summary` that walks `SUMMARY.md`'s links and asserts every `.md` under `docs/src` (except `SUMMARY.md`) is linked | the new test; `mdbook build` green | — |
| 4 | `deny.toml`: remove the two allowed-but-unused licenses or keep them with a comment naming why; the review request says which | `cargo deny check` without the two warnings | — |

## Explicit non-change scope

The container image job; the semver job; the pinned actions' versions
and the float of `mdbook-version: latest` and the toolchains (M8); the
release itself (the owner publishes; the original architect tags).

## Prohibited shortcuts

No `gh release create` anywhere in the workflow; no dispatch input; no unpinned action; no
`continue-on-error` on the build jobs; no hand-uploaded binaries; no
asset overwrite.

## Boundaries

Workflows, `deny.toml`, `docs/src/SUMMARY.md`, the gates test; no source
change in the crates beyond the two tests.

## Known risks

The first real attach happens at 0.8.0; the re-run from the tag exists for
the order tag-then-publish. The branch dispatch proves the build matrix
before; it cannot prove the attach, and the reviewer's readiness report says
so rather than claiming it.

## Required evidence and review request

The gates; the pinned-actions test; the recorded dry run; the review
request at `.git-exclude/review-requests/019c-release-ci-records/README.md`.
