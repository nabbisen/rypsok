# Handoff — RFC 016: 1.0.0, the stable contract

**RFC.** [`../../done/016-stable-contract.md`](../../done/016-stable-contract.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-MAIN-006,
REQ-ARCH-007, REQ-MAIN-003, REQ-CTX-002, REQ-OPS-001, REQ-MCP-006),
`docs/src/maintainers/external-design.md` §2, §3, §4, §13, `ROADMAP.md`
1.0.0.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** Release 0.5.0 and the M5 review decisions.
**Milestone.** The 1.0.0 row. Deliverable 5 is the release.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | **The contract audit.** Every row of the tools page, the configuration page and the command line walked against the code, the schema fixtures and the protocol tests; every inconsistency, misreadable name or unwritten meaning listed in the review package as a question with a proposed fix; nothing changed silently | The list in the package; the owner's decisions applied before deliverable 3 |
| 2 | `#[non_exhaustive]` and constructors on the public types the RFC lists; `EngineProperties::new` and `with_*`; `RawResult::new`/`with_date`; in-crate literals unchanged; `cargo doc` clean | Every test green; a doc test on `EngineProperties::new` and `RawResult::new`; `cargo semver-checks` runs clean against 0.5.0 except for the intended `non_exhaustive` additions, which are listed in the package |
| 3 | `docs/src/reference/stability.md` with the RFC's table and rules; `SUMMARY.md`; README "Stability" paragraph (the README stays under 200 lines); changelog preamble; introduction and integration guide wording; REQ-ARCH-007 amended; `ROADMAP.md` row | `mdbook build` green; the page test of RFC 002 unchanged |
| 4 | The semver gate: a CI job running `cargo semver-checks check-release` for both crates, enabled from the commit after 1.0.0 (a condition on the workspace version or a dated switch, stated in the workflow) | The job appears green on the release commit's successor |
| 5 | Release 1.0.0: version, changelog section `[1.0.0] — <date> — Stable contract`, tag, crates, GitHub release, image job | CI green; the package records it |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | The audit (a package section, no code) | — |
| 2 | Audit fixes the owner decided | 1 |
| 3 | `non_exhaustive`, constructors, doc tests | — |
| 4 | Stability page and words; semver gate | 2, 3 |
| 5 | Release | 4, the owner's authorization |

## 3. Conventions that the reviewer will check

- No behavior change rides with 1.0.0 except audit fixes the owner
  decided; each is its own changelog line under `Changed`.
- The stability page states what may change as plainly as what may not;
  the logs rule and the limits rule are on the page, not in a footnote.
- Constructors take the fields a driver author must think about and
  default the rest conservatively (cacheable `false`, attribution
  `false`, scraping `false`, standard User-Agent, 1 MiB response bound).
- The semver gate names the baseline it compares against.

## 4. Tests

No new T requirement. The doc tests of deliverable 2, the page test of
RFC 002, the schema fixture tests of RFC 003, and the semver gate.

## 5. Review package

`.git-exclude/reviews/016-stable-contract/README.md`: the audit list
(deliverable 1), the `semver-checks` report, the gate results, and the
release record. The audit list is the one place where a change to the
contract before 1.0.0 is proposed; everything else in this handoff is
additive.
