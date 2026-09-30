# Handoff — RFC 002: Test strategy and CI gates

**RFC.** [`../../accepted/002-test-strategy.md`](../../accepted/002-test-strategy.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-TEST-001,
-012; REQ-SEC-020; REQ-ARCH-005; REQ-RUST-002), `ROADMAP.md` M0.
**Depends on.** The workspace of RFC 001 (PR 1 at least).
**Milestone.** M0.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `.github/workflows/ci.yml` with the gates of RFC 002, on Linux, macOS and Windows with stable, and on Linux also with Rust 1.88 | All gates green on the M0 workspace |
| 2 | `tools/check-requirements-coverage` and `tools/CURRENT_MILESTONE` (content `M0`) | The script parses `docs/src/maintainers/requirements.md`, lists every `T` requirement at or below the current milestone, and fails when one has no test naming it. Proven by a deliberate temporary breakage in the PR description. At M0 the three `T` requirements of M0 must be named by tests: REQ-ARCH-005 (the dependency-rule script has a test that runs it), REQ-SEC-020 (a test that `cargo deny` configuration exists and `unsafe_code` is forbidden), REQ-TEST-012 (the workflow file lists every gate) |
| 3 | `deny.toml` policy as RFC 001 lists it, wired into CI | `cargo deny check advisories licenses sources` passes |
| 4 | Fixture and hostile-corpus directories with a `README.md` each stating the naming rule and the no-credentials rule | Present; empty apart from the READMEs |
| 5 | A scheduled workflow (weekly) with placeholders for fuzz and live checks that report only | Present; does not gate |
| 6 | `docs/src/maintainers/` builds with `mdbook build docs` in CI | Green |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | CI workflow with format, lints, tests, doc build, `cargo deny` | RFC 001 PR 1 |
| 2 | `tools/check-requirements-coverage`, `tools/CURRENT_MILESTONE`, the three M0 tests | RFC 001 PR 4 |
| 3 | Fixture and corpus directories, scheduled workflow, mdBook build | 1 |

## 3. Conventions that the reviewer will check

- The test naming rule of RFC 002: doc comment and function name carry the
  requirement identifier.
- No network access in any gating job; the scheduled job is the only place
  that reaches providers.
- Scripts in `tools/` are portable: they run on the three platforms or are
  marked Linux-only and skipped elsewhere.

## 4. Review request package

Hand back `.git-exclude/reviews/002-test-strategy/README.md` with links to
this handoff and RFC 002, the PR list, a run link or log of the green
workflow on all platforms, the output of the coverage script, and any
deviation as a question.
