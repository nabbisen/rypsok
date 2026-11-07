# Handoff 020w — When checks run (D-23): release checks, no timer

**Created 2026-11-07.** A fix handoff of the audit (RFC 020, D3), wave 4.
Decision D-23, authorized by the owner on 2026-11-07. The owner's model: "we do not
have to have such routine of daily, weekly or monthly"; "too many times will
consume our time and stop all the following work. However, security and
release stability are very important."

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md), section D5.
**Rules.** As 020q, plus the per-change check rule of D-23
(`.git-exclude/rules/rypsok-checks-per-change.md`), which applies to every
handoff from now on. Evidence goes to `.git-exclude/evidence/020w/`.
**Depends on.** 020q item 8 (`.cargo/mutants.toml` and the pinned `cargo-mutants`
version) and 020s. Do this before 020p: 020p changes the release build, and
after this handoff that build lives in one place.

## The policy this handoff builds

A check runs when the thing it checks can have changed, and it runs once:

| When | What | Who waits |
|---|---|---|
| Every push | CI, unchanged | nobody |
| Every handoff | the dev's mutation testing and fuzzing of the changed modules (a rule, not a workflow) | nobody |
| Before every release, on the commit that is tagged | **release checks**: everything below | only the release |
| On demand | the external checks (live checks and advisories), when a user reports a broken engine or an advisory is published | nobody |

There is **no timer**: no `schedule:` trigger anywhere in `.github/`.

## Change scope

| # | Change | Test (fails before) |
|---|---|---|
| 1 | **One build definition.** Move the build steps of `release.yml`'s `build` job into a local composite action, `.github/actions/build-release/action.yml`. Its inputs are `target` and `rustflags`. It holds the pinned compiler (`1.99.1`) and the packaging steps, and it is the only place that names the release compiler. `release.yml`'s `build` job calls it with no change of behavior: same artifact names, same archives. `releasing.md` step 2 names the action's `toolchain:` line | The 020o test of the named toolchain reads the action; a new test asserts that no workflow under `.github/workflows/` names the release compiler itself |
| 2 | **`external-checks.yml`**: the live checks job of today's `scheduled.yml` (unchanged: the Brave warning step, then the `--ignored` run), plus a job `advisories` that runs `cargo deny check advisories` (fresh database; cargo-deny pinned as in `ci.yml`). Triggers: `workflow_dispatch` and `workflow_call`, nothing else. `permissions: contents: read`; no `continue-on-error`; the two jobs are independent | The 020o test `req_test_003_scheduled_checks_…` is renamed for the new file; it asserts the triggers are exactly these two, and asserts the jobs `live-checks` and `advisories` |
| 3 | **`release-checks.yml`**: trigger `workflow_dispatch` only; `permissions: contents: read`; `concurrency: release-checks` (`cancel-in-progress: false`); no `continue-on-error`; every job independent except `reproducible-compare`. Jobs: **(a)** `reproducible`: matrix of the release targets × `build: [a, b]` (`max-parallel: 3`). Each job uses the action of item 1 and uploads `repro-<target>-<build>`. **(b)** `reproducible-compare` (needs (a)): unpack both builds of each target and compare the `rypsok` binary's SHA-256, not the archive's. Write a table to the step summary; fail on any difference. **(c)** `fuzz`: matrix of every fuzz target, a **static list** (`max-parallel: 4`, `fail-fast: false`). Each job builds only its target and runs it 300 s from `fuzz/corpus/<target>`; `timeout-minutes: 20`; upload `fuzz/artifacts` on failure. **(d)** `mutants`: matrix of the six security modules (`egress-policy`: `egress/policy.rs` with `policy/`; `privacy`; `boundary`; `robots`: `fetch/robots.rs`; `http`: `mcp/http.rs` with `http/detect.rs`; `sessions`: `mcp/http/sessions.rs`), `max-parallel: 3`, `fail-fast: false`. It runs the pinned `cargo-mutants` with `--jobs 2` and the committed `.cargo/mutants.toml` (the tool's default place, so no `--config`), and `df` is checked first; `timeout-minutes: 90`. **Red only when a mutant is missed** (a timeout is listed in the summary and counts as caught); upload `mutants.out` on failure. **(e)** `external`: `uses: ./.github/workflows/external-checks.yml` | New tests: the trigger is `workflow_dispatch` only; the `fuzz` matrix equals the `[[bin]]` names of `fuzz/Cargo.toml` (a target added without the workflow fails the test); the `mutants` matrix names the six modules; each matrix's `max-parallel` is set and the sum of the four is at most 12; every job has `timeout-minutes`; no job has `permissions` above `contents: read` |
| 4 | **Delete `scheduled.yml`.** Its fuzz job is replaced by (c), its live checks by item 2. The 020u test `req_sec_023_every_fuzz_target_runs_240_s…` becomes "each target runs 300 s in the release checks" | A test that no workflow has `on.schedule` |
| 5 | **Why the caps.** A runner pool is shared: at most 12 release-check jobs at once leaves room for CI's jobs, so a push during the release checks is not queued behind them. Say so in a comment at the top of `release-checks.yml` | — |
| 6 | **`docs/src/maintainers/checks.md`** (new, in `SUMMARY.md` under maintainers): "When checks run". The table above; what each release-check job proves; **what red means**. Red in build, reproducibility, fuzz, mutation or advisories is a No-Go until fixed. Red in the live checks: re-run the failed job once (a provider's bad day). A second failure means the provider's format changed: No-Go until the driver is fixed, or until the owner accepts in writing to ship it failing, which the release notes then state. There is no timer, and say why. The accepted residual: drift or an advisory between releases is seen at the next release, or sooner on demand. Dependabot security alerts, if the owner turns them on, are raised by GitHub with no run of ours | `req_doc_001` (book links) |
| 7 | **`releasing.md`** (amended in the review, C3: the compiler is bumped once, at the start of the cycle; the Go follows the release checks). The steps: **1. Update dependencies, once per release cycle**: at the start of the cycle (D-14 as amended), `cargo update`, the pinned actions (the latest release of each, by commit SHA with the version in a comment), the base image digest, and the compiler of item 1, in one commit through CI. Nothing updates on a timer. **2. The release commit**: the version bump and the changelog, green gates. **3. Run the release checks on the release commit**: dispatch `release-checks.yml` from `main` at that commit; every job green or explained as `checks.md` says. **4. Go and authorization**: the reviewer's readiness report quotes the run id and its `head_sha`; the owner authorizes. **5. Tag the commit the run checked** (`head_sha`); a later commit runs the checks again. The "dry run" section becomes "The reproducibility check", which is job (a)+(b) | `req_doc_001` |
| 8 | **Requirements** (exact text). REQ-TEST-003: "Each driver shall have a live check, outside the CI gate, within the provider's terms, that detects format drift. It runs in the release checks before every release and on demand, never on a timer. (R-12; RFC 020, D-23)". REQ-SEC-023: replace "that runs on the schedule of REQ-TEST-003, report only; a finding becomes a test" with "that runs in the release checks (REQ-TEST-019); a crash is a No-Go for the release and becomes a test". New **REQ-TEST-019 Release checks — M8 · T**: "Before every release, on the commit to be tagged, one manual workflow shall run in parallel the reproducibility check of the release builds, every fuzz target, mutation testing of the security modules against a committed exclusion list, the live checks and the advisory check; no job may be ignored, and the release is tagged on the commit the run checked. (RFC 020, D-23)". The external design's requirement table gains the row (Internal), and REQ-SEC-023's line there drops "the scheduled workflow". `tools/check-requirements-coverage` must pass: the tests of items 1 to 4 name REQ-TEST-019 | `tools/check-requirements-coverage` |
| 9 | Changelog `[Unreleased]`, under Changed (internal): "Checks run per change and before every release, not on a timer: `release-checks.yml` (reproducibility, all fuzz targets, mutation testing, live checks, advisories) replaces the weekly `scheduled.yml`" | — |

## Explicit non-change scope

- `ci.yml` is not changed. No `paths:` filters; nothing added to every push.
- No Dependabot configuration file (D-14 as amended).
- No change to `release.yml`'s behavior on a tag: the attach, image and
  checksums jobs are untouched apart from item 1. 020p changes them.
- No source change under `crates/`.

## Required evidence and review request

1. The gates.
2. GitHub dispatches a workflow only once its file is on the default branch,
   so you cannot run the new workflows before they are pushed. Run the
   commands of one `fuzz` job (`judge_url`, 300 s) and of one `mutants` job
   (`robots`) locally, exactly as the jobs run them, and report the output.
3. Write the review request at `.git-exclude/review-requests/020w-check-policy/README.md`.
   Push only after approval, as usual. After the push, the architect
   dispatches both workflows. That first run is the acceptance: its run id,
   `head_sha`, each job's result and duration, the wall time and the peak
   number of jobs running at once go into the review. A red job is a
   correction of this handoff.
