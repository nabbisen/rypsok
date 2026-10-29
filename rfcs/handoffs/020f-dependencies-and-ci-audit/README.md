# Handoff 020f — Audit of the dependencies, the build, CI and the release pipeline

**Created 2026-10-29.** An audit handoff: findings only, no source change.

**Task title.** Audit what the build depends on and trusts: crates, features,
tools, base images, workflow permissions, schedules and the release path;
report in the format of RFC 020 D1.
**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md),
D1 and D4.
**Applicable requirements.** REQ-RUST-*, REQ-SEC-020 (supply chain),
REQ-SEC-022, REQ-SEC-023, REQ-TEST-012, REQ-DEP-*. **Seeds.**
`arch-001-prepare` C2, C3, C5, C6, C7; `M8-DEFERRED.md` rows "Build
attestation for release binaries", "aarch64 binaries", "The book is
published from `main`…", the four duplicate-version warnings of `cargo deny`.
**Rules.** As 020b. **Depends on.** Nothing.
**Purpose.** Show the whole trust surface of building and shipping rypsok,
so the owner can decide what to pin, remove or attest.

## Change scope (what to audit)

| # | Area | Questions the report answers |
|---|---|---|
| 1 | **Direct dependencies.** | A table, one row per direct dependency of each crate (and of `fuzz/`): version; purpose in one line; the features enabled and whether `default-features = false` would do; latest version on crates.io and date of last release (`cargo search`/`cargo info`, public metadata only); maintainers' status (archived? yes/no from the crates.io page or repository as cached by `cargo info`); `unsafe` use in our code that touches it; could the standard library or an existing dependency replace it? |
| 2 | **The full tree.** | `cargo tree --workspace -d` (duplicates: the four `cargo deny` warns about — `getrandom`, `hashbrown`, `syn`, `windows-sys` — why each, which dependency pulls each, does a newer version of the puller remove it); `cargo tree -e features` for the heavy ones (`tokio`, `hyper`, `reqwest`, `rustls`, `rmcp`); the count of crates, of crates with `build.rs`, of proc-macro crates; every crate that runs code at build time and what it does (read the `build.rs` of those you can) |
| 3 | **Advisories and licenses.** | `cargo deny check` output in full; `deny.toml` line by line: which setting is lax (`multiple-versions = "warn"`, `unused-allowed-license = "allow"`), and what each stricter setting would break today (run it in the scratchpad with the stricter value) |
| 4 | **Toolchain and MSRV.** | The declared MSRV (1.88) against the truth: build and test with 1.88 locally (`rustup` has it) and report; the lowest version at which it builds (try 1.85 to 1.87, which are installed); `rust-version` in the manifests; what in the tree needs 1.88 |
| 5 | **Workflows: permissions.** | For each workflow and job: the permissions it has (explicit or inherited from the repository default; read the repository setting with `gh api repos/nabbisen/rypsok/actions/permissions/workflow` and report), the secrets it can read, whether it runs on `pull_request` from forks, what an untrusted contributor's pull request can make each job do (the first review: C2) |
| 6 | **Workflows: floating inputs (C3).** | Every input not pinned by hash or digest: `mdbook-version: latest`, `toolchain: stable|nightly`, `tool: cargo-deny` and `cargo-fuzz` without versions, `Dockerfile` base tags, `apt` installs (`musl-tools`), `pip`/`npm` installs, `curl | sh`. For each: what changes if it floats, what it would take to pin (the way: a version, a digest, a lockfile), and whether the pin would go stale (who updates it) |
| 7 | **Workflows: failure visibility (C5).** | `continue-on-error`, `|| true`, `if: always()`, steps that cannot fail the job. For the weekly jobs (live checks, fuzz): the last 10 runs (`gh run list`) and their results, and what a failure shows to the owner today (nothing? an email? a red mark?). Fuzzing: seconds per target and number of targets (`fuzz/fuzz_targets`: nine) against the parsers of untrusted bytes (020d area 7 lists them; coordinate by reading its report if it is there) |
| 8 | **Matrix and platforms (C6).** | REQ-RUST-002 versus the CI matrix versus the release matrix: a table platform × (built in CI / tested in CI / built in release / smoke-tested in release / attached). What would aarch64 Linux and Windows cost in minutes and in complexity (GitHub-hosted runners available to this repository: check `gh api` for runner labels you can read; do not create runners) |
| 9 | **The release path.** | Read `release.yml`, `releasing.md` and the 0.8.0 run (40883698930): trust chain from commit to the file a user downloads (who can change what at each step: tag, workflow at tag, runner image, actions, toolchain, base images). Are the binaries reproducible (build the Linux archive twice in the scratchpad with the same toolchain; compare hashes; say what differs)? What would GitHub artifact attestations (build provenance) need and give — report only. `SHA256SUMS` proves integrity only |
| 10 | **Container image.** | `Dockerfile`: base images by tag (digests?), user, file permissions, what the image contains (list), size, whether the binary in the image is the one the release attaches, health check, what a user must pass to run it read-only. The image is built per tag and kept as a workflow artifact; nothing is published: state what a user can do today and what the documents claim |
| 11 | **The book and the public repository.** | The book publishes on every push to `main`, so it runs ahead of the release. List the pages that describe unreleased behavior at any time between a merge and a release; how a reader can tell which version a page describes today (a version stamp? the changelog?); options in the sense of cost and risk (publish from the tag, version selector, a banner), no recommendation |
| 12 | **Repository hygiene.** | Files in the tree that should not be (large, generated, secrets: scan with `grep` for `BEGIN .* PRIVATE KEY`, `api[_-]?key`, tokens, `.env`; `git ls-files` sizes over 100 KiB); `.gitignore` and `.git/info/exclude` assumptions; the licenses of non-code assets in `docs/` |

## Explicit non-change scope

No source, workflow, manifest or document is changed; `deny.toml` and other
configs are copied to the scratchpad for the stricter runs. No `gh` call that
writes (no workflow dispatch, no release or tag change); `gh api` reads only.
No commit, no push.

## Prohibited shortcuts

A table row for a dependency you did not look at. "Pinned" for an input that
is only pinned in one of two places. Reporting the repository default
permission from the YAML instead of from the API.

## Required evidence and review request

`.git-exclude/review-requests/020f-dependencies-and-ci-audit/README.md` in
the format of RFC 020 D1: findings (`DEP-n`), then the tables of areas 1, 6,
8 and 9, then the four closing lists.
