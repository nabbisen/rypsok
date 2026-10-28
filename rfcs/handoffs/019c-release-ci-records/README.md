# Handoff 019c — Release binaries, the missing gates, the book's dead link

**Task title.** Make the release page carry binaries, run every gate in
CI, and fix the book's dead link (RFC 019 D7, D8, D9).
**RFC.** [`../../accepted/019-first-independent-review.md`](../../accepted/019-first-independent-review.md).
**Applicable requirements.** REQ-DEP-005, REQ-TEST-012, REQ-SEC-020,
REQ-DOC-001. **Source findings.** `arch-001-prepare` U4, U5, C1, C4;
`dev-001-prepare` D1, D2, D5.
**Rules.** As 019a. **Depends on.** Nothing; may run beside 019a and 019b.
**Purpose.** The sentence "binaries for Linux, macOS and Windows are
attached to each release" becomes true; two gates the task text and the
packages called gates run in CI; the published book has no dead link.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | `.github/workflows/release.yml`: a `binaries` job (matrix: `x86_64-unknown-linux-musl` on ubuntu, `aarch64-apple-darwin` on macos, `x86_64-pc-windows-msvc` on windows) that builds `rypsok` in release with `--locked`, packages `rypsok-<tag>-<target>.tar.gz` (`.zip` on Windows) with `LICENSE` and `NOTICE`, writes `SHA256SUMS`, and uploads them to the GitHub release of the tag with `gh release upload` (`permissions: contents: write` on this job only; every action pinned by hash as the gate test requires); the image job unchanged | the pinned-actions gate test still green; a dry run of the job on a branch tag is not possible without a release, so the review request records the first real run (0.7.1) | getting started: the sentence stays, now true; the deployment page: one line on verifying `SHA256SUMS` |
| 2 | `ci.yml` rules job: `tools/check-limits` step after the coverage step; `cargo deny check advisories bans licenses sources`; `req_test_012_ci_workflow_lists_every_gate` extended to require both | the gate test | RFC 002's gate table (amendment line dated with this handoff) |
| 3 | `docs/src/SUMMARY.md`: `[DuckDuckGo](./maintainers/engines/duckduckgo.md)` under the engine records | `mdbook build` green; a test? No: a check in the examples or gates test that every `.md` under `docs/src` is reachable from `SUMMARY.md` (a short Rust test that walks the summary's links and the directory; name it `req_doc_001_every_page_is_in_the_summary`) | — |
| 4 | `deny.toml`: remove the two allowed-but-unused licenses, or keep them with a comment naming why; the review request says which | `cargo deny check` without the two warnings | — |

## Explicit non-change scope

The container image job; the semver job; the versions of the pinned
actions (M8 decides the float of `mdbook-version: latest` and the
toolchains).

## Prohibited shortcuts

No unpinned action; no `continue-on-error` on the binaries job; no
hand-uploaded binaries.

## Boundaries

Workflows, `deny.toml`, `docs/src/SUMMARY.md`, the gates test; no source
change in the crates beyond the two tests.

## Known risks

The first real run of the binaries job happens at 0.7.1; if it fails, the
release is complete on crates.io and the image, and the binaries are
attached by a re-run of the job on the same tag (`workflow_dispatch`
with the tag as input: add it).

## Required evidence and review request

The gates; the pinned-actions test; the review request at
`.git-exclude/review-requests/019c-release-ci-records/README.md`.
