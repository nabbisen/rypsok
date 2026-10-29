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
   workflow, whose attach job then finds the release.
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
