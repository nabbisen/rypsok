# Releasing

How a release reaches its users. The owner decides that a release happens;
the steps below are what follows. Nothing here publishes by itself: the
release workflow builds and attaches, and never creates a release.

## The procedure

1. **Update dependencies, once per release cycle.** At the start of the cycle
   (D-14 as amended), and not on a timer: `cargo update`; the pinned actions
   (the latest release of each, by commit SHA with its version as a comment);
   the base image digest in the `Dockerfile`; and the release compiler, the
   `toolchain:` line of `.github/actions/build-release/action.yml`, the one
   place that names it (set it to the current stable: `rustup update stable`,
   `rustc --version`). One commit, through CI. The compiler is a deliberate
   change of the cycle, made once, not a surprise of the day and not a change
   after the checks. CI keeps `stable` on purpose, to see new lints early; its
   1.88 job covers the floor. Between cycles nothing updates by itself.
2. **The release commit.** The version bump and the changelog, through CI;
   the gates are green on it.
3. **Run the release checks on the release commit.** Dispatch
   `release-checks.yml` from `main` at that commit (Actions, Release checks,
   Run workflow). Every job must be green or explained as
   [When checks run](./checks.md) says.
4. **Go and authorization.** The reviewer's readiness report quotes the id of
   the release-checks run and its `head_sha` and records the Go; the owner
   authorizes the release.
5. **Tag** the commit the run checked (its `head_sha`); a later commit runs
   the checks again. Tag it `X.Y.Z` (no `v` prefix) and push the tag. The tag
   must equal the workspace `version` of `Cargo.toml`; the attach job fails
   otherwise.
6. **Publish.** The owner publishes the release on GitHub from the tag, or
   publishes first: creating the release creates the tag, which starts the
   workflow, whose attach job then finds the release. When publishing
   first, choose the release commit as the target of the new tag, not the
   branch head: the tag goes on the commit chosen, and the attach job's
   version check catches a wrong version but not a wrong commit with the
   same version.
7. **Read the run.** The `release` workflow runs four jobs in order: `build`
   (the four binaries), `image` (the two container images, made from the
   Linux binaries), `checksums` (`SHA256SUMS` over the four archives and the
   two images, and their provenance attestations) and `attach`. If the release did not exist
   when its attach job ran, the job wrote a notice in the run summary and
   attached nothing. In that case run the workflow again from the tag
   (Actions, release, Run workflow, "Use workflow from: Tag X.Y.Z", or
   `gh workflow run release.yml --ref X.Y.Z`): the attach job uploads to the
   release that now exists.
8. **Check the release page.** It lists seven files: four archives (Linux
   x86_64 and aarch64, both static musl; macOS arm64; Windows x86_64), two
   images (`-image-amd64.tar.gz`, `-image-arm64.tar.gz`) and `SHA256SUMS`. The
   release notes name the compiler: paste the line `Built with rustc …` that
   the attach job's run summary printed. Verify one file as a user does
   (`gh attestation verify <file> --repo nabbisen/rypsok`).
9. **Nothing is overwritten.** The attach job refuses to replace an asset
   that is already on the release. To replace one, delete it on the release
   page by hand and say so in the release notes, then run the workflow again
   from the tag.

## If the attach job fails

A failed attach job leaves the release as the owner published it: it
creates nothing and replaces nothing. The run's workflow artifacts
(`archive-*`, `image-*` and `checksums`) are the files the workflow built
from the tag. To attach them by hand:

1. Download the artifacts `archive-*`, `image-*` and `checksums` of the tag's
   run, and not the small `sha-*` artifacts. `gh` puts each
   artifact in a directory of its own (the run page's "Artifacts" gives one
   zip each), so move the files into one directory afterwards, in an empty
   directory:

   ```sh
   gh run download <run id> --repo nabbisen/rypsok --pattern 'archive-*' --pattern 'image-*' --pattern checksums --dir artifacts
   mv artifacts/*/* .
   rmdir artifacts/* artifacts
   ```

   The four archives, the two images and `SHA256SUMS` now lie side by side.
2. Check them: `sha256sum --check SHA256SUMS` must print `OK` for all six
   files and exit 0. It fails when a file is missing; do not add
   `--ignore-missing`, which passes with a file absent. All six files
   must be there before the upload, and nothing that does not match is
   uploaded.
3. Upload the six files and `SHA256SUMS` to the release (the release
   page, "Edit", or `gh release upload X.Y.Z <files>`).
4. Say in the release notes that the files were attached by hand from the
   artifacts of the run, naming the run. The attestations belong to the
   files of that run, so they still verify.

A run again from the tag does not help while the defect is in the workflow:
it uses the workflow as the tag holds it. The defect is fixed in the next
release. Never rebuild the binaries locally for a release: the checksums
and the notes would then describe files the workflow did not build.

## The reproducibility check

Builds of one commit are meant to be reproducible: the same compiler, the
locked dependencies and, on Windows, the linker flag `/Brepro` (without it the
linker writes a timestamp into the executable). The release checks prove it on
every release: the jobs `reproducible` and `reproducible-compare` build every
release target twice, through the same local action as the release itself,
and compare the SHA-256 of the unpacked `rypsok` binaries (the archives differ
in their own timestamps, so the binaries are compared, not the archives). A
difference fails the run.

A run of `release.yml` from a branch (Actions, release, Run workflow, "Use
workflow from: Branch main") still builds the archives and the checksums and
uploads them as workflow artifacts. It builds the two images and
`SHA256SUMS` too, so the dry run proves that the image builds, but it attests
nothing and attaches nothing: those two steps run on a tag ref only. What no
check shows is the attestation and the attach, which first run for a real
tag.

## What the checksums prove

`SHA256SUMS` shows that a download is intact. It does not show where it
came from: anyone who can replace the archive can replace the file beside
it.

The provenance attestation (RFC 020, D-16) shows the origin. On a tag, the
`checksums` job asks GitHub to sign, for each of the six files, a statement
that this repository's `release` workflow built it from the tagged commit
(`actions/attest-build-provenance`). `gh attestation verify <file> --repo
nabbisen/rypsok` checks the signature. That job is the only one that may sign
(`id-token: write`, `attestations: write`) and it runs no build code: it has
no checkout and no `cargo`, because the build jobs run the `build.rs` of
third-party crates and must never hold a token that can sign. The attestation
shows the file came out of the workflow; it does not show that the source is
free of defects, and the workflow is trusted as the owner's.

The images are built from the release binaries (RFC 020, D-12): the `image`
job checks the Linux archives it downloads against the SHA-256 that each
`build` job computed, unpacks them, and copies each binary onto the pinned
base image. Nothing is compiled in the image, so the binary in it is the one
the release checks examined. There is no registry (a later choice of the
owner).
