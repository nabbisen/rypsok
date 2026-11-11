# When checks run

A check runs when the thing it checks can have changed, and it runs once.
There is no timer: nothing in `.github/` runs on a schedule. A check that
runs every day or every week would cost the maintainers time, answer a
question that has not changed, and stop the work that follows when it goes
red for a cause that has nothing to do with the change at hand. Security and
release stability are what matter; they are checked where they can be lost,
which is the change and the release (RFC 020, D-23).

| When | What | Who waits |
|---|---|---|
| Every push | CI (`ci.yml`): format, lints, tests on three platforms and the 1.88 floor, each feature alone, the dependency rule, requirement coverage, the limits table, supply chain, the book | nobody |
| Every handoff that changes a parser, the egress policy, the boundary, the privacy check, robots.txt or the HTTP listener | The developer's own mutation testing and fuzzing of the changed modules, reported in the review request (a rule of the process, not a workflow) | nobody |
| Before every release, on the commit to be tagged | The **release checks** (`release-checks.yml`), dispatched by hand, everything below | only the release |
| On demand | The **external checks** (`external-checks.yml`): the live driver checks and the advisory check, when a user reports a broken engine or an advisory is published | nobody |

## The release checks

One manual workflow of independent jobs, run from `main` at the release
commit. At most 12 jobs run at once, so that a push during the release checks
is not queued behind them (a runner pool is shared). What each job proves:

| Job | What it proves |
|---|---|
| `reproducible` (3 targets × builds `a` and `b`) | The release builds run, with the compiler the release names (the local action `.github/actions/build-release`), on every release target |
| `reproducible-compare` | The two builds of each target give the same `rypsok` binary: the SHA-256 of the unpacked binary, not of the archive (archives carry their own timestamps). A table is written to the run summary |
| `tool-definitions` | The pinned tool definitions (`tools_list.json` and the server instructions) are compared with those at the latest release tag. When they differ, the newest changelog section must have a `### Tool definitions` heading (RFC 021, D3). It starts after the comparison of the builds, so it adds nothing to the peak |
| `fuzz` (one job per fuzz target, 300 s each, from the target's corpus) | No parser of untrusted input panics, runs out of memory or hangs on what the fuzzer finds in five minutes |
| `mutants` (the six security modules: `egress-policy`, `privacy`, `boundary`, `robots`, `http`, `sessions`) | Every mutant of those modules is caught by a test, except those listed with a reason in `.cargo/mutants.toml`. Timeouts count as caught and are listed in the summary |
| `external` | The live checks and the advisory check below |

The external checks are two independent jobs: `live-checks` (each driver
sends its fixed query to the real endpoint and the answer must parse; a
Brave check without a key says so in the run summary) and `advisories`
(`cargo deny check advisories` against a fresh advisory database).

## What red means

- **Red in the build, the reproducibility check, a fuzz target, a mutation
  job or the advisories is a No-Go** until it is fixed. A crash is not
  fixed silently: it becomes a test and a finding in the review. A missed
  mutant is killed by a test, or listed in `.cargo/mutants.toml` with the
  reason it cannot change behaviour.
- **Red in the live checks:** run the failed job once more (a provider has
  bad days). A second failure means the provider's format changed: that is a
  No-Go until the driver is fixed, or until the owner accepts in writing to
  ship with the driver failing, which the release notes then state.
- **Nothing may be ignored.** No job is allowed to fail without failing the
  run (`continue-on-error` is forbidden by a test), and the release is tagged
  on the commit the run checked, the `head_sha` of the run. A later commit
  runs the checks again.

## What is accepted

Drift of a provider, or an advisory against a dependency, that appears
between two releases is seen at the next release, or sooner on demand. That
is the price of having no timer. If the owner turns on Dependabot security
alerts for the repository, GitHub raises them with no run of ours.

## Updating dependencies

Nothing updates on a timer either. Dependencies, pinned actions, the base
image and the release compiler are updated once per release cycle, at its
start (see [Releasing](./releasing.md)).
