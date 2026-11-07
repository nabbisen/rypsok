# Handoff 020p — The release path: image from the release binary, Linux aarch64, provenance, stripped binaries

**Created 2026-11-07.** A fix handoff of the audit (RFC 020, D3), wave 4.
Owner decisions **D-12** (the image is built from the release binary and is
listed in `SHA256SUMS`), **D-13** (Linux aarch64), **D-16** (build provenance,
made in the checksums job only) and **D-22** (`strip = "symbols"`, no LTO), all
authorized on 2026-11-03. Findings DEP-1, DEP-2, DEP-15 and DEP-17 (the 020f review) and PRF-8 (the
020e review).

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md), RFC 013 (distribution), RFC 019 D8 (the release workflow).
**Rules.** As 020w, including the per-change rule. Evidence goes to `.git-exclude/evidence/020p/`.
**Depends on.** 020w: the release build is the composite action
`.github/actions/build-release`, and the reproducibility check uses it too.

## Why

What a user downloads should be exactly what the release checks examined,
and the user should be able to verify that.
- **Today the image is built from source, by a different compiler.** The
  `Dockerfile` compiles with `rust:1.88-alpine`, while the release binaries
  use the pinned `1.99.1`. The image is not in `SHA256SUMS`, and nothing ties
  it to the checked binary.
- **Integrity and origin.** `SHA256SUMS` proves that a file is intact, not
  where it came from. A provenance attestation proves the origin.
- **Linux aarch64** is a common server platform and is not built today.
- **Symbols** make up most of the size of an unstripped binary and serve no
  user.

## Change scope

| # | Change | Test (fails before) |
|---|---|---|
| 1 | **D-22.** `[profile.release]` in the workspace `Cargo.toml`: `strip = "symbols"`; no `lto`, no `codegen-units`, no `panic` (unwinding stays, REQ-SEC-014). Report the size of each release binary before and after | A gate test reads the profile: `strip = "symbols"`, and none of `lto`, `panic` |
| 2 | **D-13, CI.** A job `ubuntu-24.04-arm / stable` in `ci.yml` beside the other platform jobs, with the same steps as the Linux one. GitHub's native arm runner is free for public repositories. If the first push shows that no runner picks the job up, stop and say so; do not emulate | The platform-matrix gate test names the arm runner |
| 3 | **D-13, release.** The target `aarch64-unknown-linux-musl` on `ubuntu-24.04-arm` in the `build` matrix of `release.yml` **and** in the `reproducible` matrix of `release-checks.yml`, which gives 4 targets × 2 builds; `max-parallel: 3` stays. The composite action installs `musl-tools` for **both** musl targets (`endsWith(inputs.target, '-linux-musl')`) | The two matrices name the same four targets (one test reads both); the action's musl step covers both |
| 4 | **D-12, the image from the release binary.** The `Dockerfile` builds nothing: `FROM` the pinned distroless static image, `ARG TARGETARCH`, then `COPY` the unpacked binary of that architecture (`dist/linux-${TARGETARCH}/rypsok`) to `/rypsok`. The rest stays as today: user, port, entry point. The `image` job `needs: build`, downloads the two Linux archives, checks each against the hashes the build jobs computed (see item 5), unpacks them, and builds **two** images with `docker buildx build --platform linux/amd64` and `linux/arm64`, each `--load`ed and `docker save`d to `rypsok-<version>-image-amd64.tar.gz` and `-arm64.tar.gz`. No emulation is needed: the Dockerfile has no `RUN`. The job runs on **every** run, with the version from `Cargo.toml`, so that the dry run proves the image builds. Only the attach job is tied to a tag. No registry (D-12: a later choice of the owner) | Gate tests: the `Dockerfile` has no `RUN` and no `cargo`, and every `FROM` is pinned by digest; `image` needs `build`; the image file names. Locally: build one image from a locally built musl binary and run `docker run --rm <image> --version` |
| 5 | **The order of the jobs and `SHA256SUMS`.** `build` → `image` → `checksums` (`needs: [build, image]`) → `attach`. `SHA256SUMS` lists the four archives **and the two images**. The attach job uploads all of them. Each `build` job writes the SHA-256 of its archive to its step summary, and the image job compares the archives it downloaded with those, so a file changed between jobs is caught before the image is made | A gate test: `checksums` needs `build` and `image`, and the attach job downloads the image artifacts |
| 6 | **D-16, provenance.** In the `checksums` job, after `SHA256SUMS`, run `actions/attest-build-provenance` (pinned by SHA, the version in a comment) with `subject-path` naming the four archives and the two images, **on a tag ref only** (`if: github.ref_type == 'tag'`). Job-level `permissions`: `contents: read`, `id-token: write`, `attestations: write`; no other job gets either. **The job runs no third-party build code**: it downloads artifacts and hashes them, no checkout of the source, no `cargo`. Say why in a comment: the build jobs run the `build.rs` of third-party crates, and must never hold a token that can sign | Gate tests: only `checksums` has `id-token` or `attestations`; `checksums` has no `actions/checkout` and no `cargo`; the attest step is tag-only and names every file of `SHA256SUMS` |
| 7 | **The attach job's summary names the compiler.** One line, `Built with rustc <version>`, read from the `toolchain:` line of the action, so that the owner can copy it into the release notes (DEP-3 of 020o). `releasing.md` step "Check the release page" says the notes carry that line | A gate test: the summary step reads the action's `toolchain:` line |
| 8 | **Documents.** (a) REQ-RUST-002, exact text: "Releases shall build and pass tests on Linux x86_64 and aarch64, macOS arm64 and Windows x86_64. macOS x86_64 and Windows aarch64 are not built. (D-23; RFC 020, D-13)". If item 2 shows no arm runner, use the set that is built instead, and say so. (b) `getting-started.md` and `README.md`: the platforms; **how to verify a download**: `sha256sum --check --ignore-missing SHA256SUMS`, then `gh attestation verify <file> --repo nabbisen/rypsok`, with one sentence each on what it proves (integrity; origin). (c) `deployment.md`, the container section: load the image from the release (`docker load < rypsok-X.Y.Z-image-amd64.tar.gz`), verify it as in (b), and run it with the flags as today. Building the image yourself becomes: build the musl binary, place it under `dist/linux-amd64/`, run `docker build`. (d) `releasing.md`: the job order, what the checksums and the attestations prove, and the six files on the release page. (e) REQ-DEP-005: the image is attached to the release, with no registry. (f) Changelog: Added (Linux aarch64 binaries and image; provenance attestations; the image in `SHA256SUMS`), Changed (the image is built from the release binary; binaries are stripped) | `req_doc_001`; the stability and example tests |

## Explicit non-change scope

- No registry, no signing key of our own, no change to what a binary does.
- No LTO.
- No change to the release checks except the fourth target.

## Required evidence and review request

1. The gates.
2. The sizes of the binaries before and after item 1.
3. One image built and run locally (item 4).
4. The per-change rule does not apply: no module in its list changes.
5. The review request at `.git-exclude/review-requests/020p-release-path/README.md`.
6. **After the push, the architect** dispatches `release.yml` from `main`
   (the dry run: four builds, two images, `SHA256SUMS`, no attestation, no
   attach) and confirms that CI's arm job runs. A red job is a correction.
   The attestation and the attach first run for the 0.9.0 tag, and the
   readiness report says so.
