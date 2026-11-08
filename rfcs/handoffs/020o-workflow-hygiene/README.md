# Handoff 020o — Workflow and build hygiene

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3). Findings
DEP-3, 4 (Windows part), 5, 6, 7, 8, 9, 10, 13, 14 of audit 020f and its
review `arch-003-audit/020f-review.md`. **Not in this handoff** (they wait
for owner decisions): the container image, platforms, Dependabot, branch
rules, attestations, the book's publishing (D-12 to D-17), and the weekly
mutation job (D-19).

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md).
**Applicable requirements.** REQ-SEC-020, REQ-TEST-012, REQ-TEST-003.
**Rules.** As 020g. A workflow change can only be verified on GitHub:
you commit locally; **the architect pushes after reading your review request
and then asks you to dispatch the workflows** (a dispatch writes nothing to the
repository; you may run `gh workflow run` for `release`, `scheduled` and
`Book` from `main`, nothing else). **Evidence rule.** As 020g.

## Change scope

| # | Required change | Check |
|---|---|---|
| 1 | **DEP-5.** `docs.yml`: workflow-level `permissions: contents: read`; `pages: write` and `id-token: write` only on the `deploy` job. The `mdbook` version is `0.5.4` (not `latest`) in `docs.yml` and `ci.yml` | A Book run from `main` after the push builds and deploys |
| 2 | **DEP-6.** `ci.yml` and `scheduled.yml` get `permissions: contents: read` at workflow level | `req_sec_020_*` gate test extended: every workflow declares `permissions` |
| 3 | **DEP-7.** In `scheduled.yml` remove `continue-on-error: true` from both jobs (a failed scheduled run now fails and GitHub notifies the owner; it still gates nothing). The two jobs stay independent. When `BRAVE_API_KEY` is empty the live job emits a `::warning::` annotation and writes "Brave check skipped: no key" to the step summary, so a skipped check is visible; the check does not report success silently | A dispatch of `scheduled` shows the annotation (the key is absent today, per the audit) |
| 4 | **DEP-8.** Each fuzz target runs 300 s (`-max_total_time=300`); the job's `timeout-minutes` is set to `target count × 5 + 15`. The new target `clean_decoded_text` is in the run (it is, by `cargo fuzz list`) | The dispatched run lists nine targets |
| 5 | **DEP-9.** `--locked` on `cargo build`, `test`, `clippy`, `doc` in `ci.yml` and `scheduled.yml` (not on `cargo deny`, `cargo fuzz`) | CI green |
| 6 | **DEP-10.** Remove `markup5ever` from the workspace and `rypsok-core` manifests; `Cargo.lock` and `fuzz/Cargo.lock` follow (only the one dependency edge changes; check the lockfile diff and report it) | The gates; `cargo tree` still contains `markup5ever` through `html5ever` |
| 7 | **DEP-13, 14.** Pin the floating tools: `cargo-deny@0.20.2`; `cargo-fuzz` at the version `cargo fuzz --version` reports locally (say which); `taiki-e/install-action` syntax is `tool: name@version`. `Dockerfile`: pin both base images by digest with the tag in a comment; get each digest from the registry's manifest (a public read; `podman`/`skopeo` are available or `curl` of the registry API; name the command you ran). If you cannot get a digest without a login, leave the tag, and say so | `docker build` is not available here; the container job of the next tag run is the check; say that in the review request |
| 8 | **DEP-3.** `release.yml` builds with a **named** toolchain (the version of the stable compiler on the day you do this, e.g. `1.99.1`; state it), not `stable`. `releasing.md` gets a step "bump the pinned compiler to the current stable, run the dry run, then release" and says the release notes name the compiler (`rustc --version` line in the notes). `ci.yml` keeps `stable` on purpose (new lints are seen early; the MSRV job covers the floor): a comment says so | The dry-run archive's `rypsok` is byte-identical to the previous one on Linux and macOS (DEP-4: reproducible) |
| 9 | **DEP-4 (Windows).** Build the Windows target with `RUSTFLAGS` including `-C link-arg=/Brepro` (set for that matrix entry only) | Two dry runs of one commit give a byte-identical `rypsok.exe` (the audit saw 23 differing bytes); report the hashes |
| 10 | **Gate tests.** Extend `req_test_012_ci_workflow_lists_every_gate` and `req_sec_020_*` to parse the YAML (not `contains`) for what items 1, 2, 3, 7 require: permissions declared everywhere, no `continue-on-error` in `scheduled.yml`, no `latest` tool version | The tests, seen failing first |
| 11 | **Changelog and RFC records.** `[Unreleased]` Changed: nothing user-visible except the release binaries' provenance (item 8, 9); add an RFC 002 amendment line (appended) for the gate changes | — |

## Explicit non-change scope

No new action; no change of the action pins; no Dependabot file; no change to
what the release attaches; no image change other than the digests; no
settings change on GitHub.

## Prohibited shortcuts

Marking a workflow change verified by YAML parsing alone; pinning a tool to a
version you did not run; guessing a digest.

## Required evidence and review request

The gates; `actionlint` (if installed; else say so); the lockfile diff; the
failing-first gate-test output; for items 1, 3, 4, 8, 9 the dispatched-run
URLs after the architect's push, added to the review request in a
"Verification" section; request at
`.git-exclude/review-requests/020o-workflow-hygiene/README.md`.
