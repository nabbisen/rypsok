# Releasing

How a release reaches its users. The owner decides that a release happens;
the steps below are what follows. Nothing here publishes by itself: the
release workflow builds and attaches, and never creates a release.

## The procedure

1. **Preconditions.** The gates are green on the release commit, the
   reviewer has recorded a Go, and the owner has authorized the release.
2. **Tag.** Tag the release commit `X.Y.Z` (no `v` prefix) and push the
   tag. The tag must equal the workspace `version` of `Cargo.toml`; the
   attach job fails otherwise.
3. **Publish.** The owner publishes the release on GitHub from the tag, or
   publishes first: creating the release creates the tag, which starts the
   workflow, whose attach job then finds the release. When publishing
   first, choose the release commit as the target of the new tag, not the
   branch head: the tag goes on the commit chosen, and the attach job's
   version check catches a wrong version but not a wrong commit with the
   same version.
4. **Read the run.** The `release` workflow builds the binaries, writes
   `SHA256SUMS`, and builds the container image. If the release did not exist
   when its attach job ran, the job wrote a notice in the run summary and
   attached nothing. In that case run the workflow again from the tag
   (Actions, release, Run workflow, "Use workflow from: Tag X.Y.Z", or
   `gh workflow run release.yml --ref X.Y.Z`): the attach job uploads to the
   release that now exists.
5. **Check the release page.** It lists three archives and `SHA256SUMS`:
   Linux x86_64 (static, musl), macOS arm64 and Windows x86_64.
6. **Nothing is overwritten.** The attach job refuses to replace an asset
   that is already on the release. To replace one, delete it on the release
   page by hand and say so in the release notes, then run the workflow again
   from the tag.

## If the attach job fails

A failed attach job leaves the release as the owner published it: it
creates nothing and replaces nothing. The run's workflow artifacts
(`archive-*` and `checksums`) are the binaries the workflow built from the
tag. To attach them by hand:

1. Download the artifacts `archive-*` and `checksums` of the tag's run, and
   not the container image, which the run also holds. `gh` puts each
   artifact in a directory of its own (the run page's "Artifacts" gives one
   zip each), so move the files into one directory afterwards, in an empty
   directory:

   ```sh
   gh run download <run id> --repo nabbisen/rypsok --pattern 'archive-*' --pattern checksums --dir artifacts
   mv artifacts/*/* .
   rmdir artifacts/* artifacts
   ```

   The three archives and `SHA256SUMS` now lie side by side.
2. Check them: `sha256sum --check SHA256SUMS` must print `OK` for all three
   archives and exit 0. It fails when an archive is missing; do not add
   `--ignore-missing`, which passes with a file absent. All three archives
   must be there before the upload, and nothing that does not match is
   uploaded.
3. Upload the three archives and `SHA256SUMS` to the release (the release
   page, "Edit", or `gh release upload X.Y.Z <files>`).
4. Say in the release notes that the files were attached by hand from the
   artifacts of the run, naming the run.

A run again from the tag does not help while the defect is in the workflow:
it uses the workflow as the tag holds it. The defect is fixed in the next
release. Never rebuild the binaries locally for a release: the checksums
and the notes would then describe files the workflow did not build.

## The dry run

A run of the workflow from a branch (Actions, release, Run workflow, "Use
workflow from: Branch main") builds the archives and the checksums and
uploads them as workflow artifacts of that run. It attaches nothing and
builds no image: both jobs run on a tag ref only. Use it to see that the
build matrix works before tagging. What it cannot show is the attach itself,
which first runs for a real tag.

## What the checksums prove

`SHA256SUMS` shows that a download is intact. It does not show where it
came from: anyone who can replace the archive can replace the file beside
it. Attestation of the archives is a candidate of the audit milestone.
