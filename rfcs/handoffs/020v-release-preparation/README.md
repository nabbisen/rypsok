# Handoff 020v — Release preparation for 0.9.0

**Created 2026-11-08.** The last handoff of the audit (RFC 020, D3), after
020x. As with 019d, the preparation of a release is a change of the tree
and goes through implement → review.

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md).
**Requirements.** REQ-MAIN-006 (every contract change announced), REQ-DEP-005.
**Rules.** As 020w. Evidence goes to `.git-exclude/evidence/020v/`.
**Depends on.**
- The approval of 020x.
- The architect's audit report, `docs/src/maintainers/audit.md`, landed on
  `main` before this starts; verify it.

**Purpose.** The changelog tells a user what 0.9.0 delivers, in the words
the pages use; the version is bumped. Nothing else changes, except the one
record item 5 completes.

## Change scope

| # | Work | Evidence |
|---|---|---|
| 1 | **The changelog, assembled.** `[Unreleased]` becomes `## [0.9.0] — <the date of the day you prepare it> — The audit (milestone M8)`. (The release commit may change only the date, and only if the day differs.) **(a) Read every line against the final tree.** The cycle wrote lines as each handoff landed, and some describe a state a later handoff replaced: the weekly `scheduled.yml` and its 240 or 300 s, which became the release checks of 020w; `mutants.toml` at the root, which moved to `.cargo/mutants.toml`; the memory figures 119/184/224/289, refitted in 020x; and others. Such lines are merged into **one line that says what 0.9.0 does**; a reader of 0.8.0 does not need the route. **(b) Order.** `### Changed` first, opened by one paragraph, "**Contract changes** (announced as REQ-MAIN-006 requires; they are why this is a minor release):", followed by one line each for every change of a tool's answer, an outcome, an error kind, a notice or a default (from the audit: RFC 020 D-1, D-2, D-3, D-7, D-10, D-20, D-21, the 16 KiB header bound, idle connections, and any other the lines name). Then the other Changed lines, then `### Added`, `### Removed` (the public items 020s removed), `### Fixed`, and the internal lines last under their own heading `### Internal`. **(c)** Every audit decision is cited `RFC 020, D-n`. **(d)** A link line at the top of the section to the audit report. **(e)** The comparison links at the foot, `0.8.0...0.9.0` | the diff; in the review request, the list of lines you merged or dropped, each with the reason |
| 2 | **Versions.** The workspace `version` and the `rypsok-core` dependency version become `0.9.0`; run `cargo update --workspace`; `cargo publish --workspace --dry-run --locked` is green | the dry run's output |
| 3 | **`tools/CURRENT_MILESTONE` becomes `M8`**, so that the M8 requirements (REQ-TEST-019 and any other tagged M8) are enforced, and the coverage gate passes | the gate's output |
| 4 | **`ROADMAP.md`**: M8 is marked delivered in 0.9.0, in the form the earlier milestones use | — |
| 5 | **RFC 020, D-5, still open: one deliberate DuckDuckGo no-results request.** It is a single request, once, with the live check's contact header, and a query chosen to have no results (say which one). Record what the page held: is `div.no-results` there? The admission record `engines/duckduckgo.md` row "No results" changes from "Not verified at admission" to what was seen, with the date. If the marker is absent, **stop and say so**: a no-results page without the marker counts toward the circuit (NET-18), and that is a decision, not a record edit | the request and the relevant part of the page, without any personal data |
| 6 | **A consistency pass, editing only what it finds.** Every page that names an error kind, a notice, an outcome, a status or a limit agrees with the delivered behavior and the limits table. That means the tools page, "Reading a response", troubleshooting, configuration, deployment, the integration guide and the stability page. Run `tools/regenerate-example-answers` (compare mode): the four answers are identical. If one differs, regenerate it by the documented method and say so. `tests/examples.rs` is green | findings fixed in place and listed in the review request |

## Explicit non-change scope

- No source change.
- No tag, no publish, no GitHub release.
- No dispatch of the release checks: the architect does that on the pushed
  release commit (RFC 020, D5).

## Prohibited shortcuts

- No changelog line that paraphrases a handoff instead of what landed.
- No line kept because it was written, if the tree no longer does what it
  says.

## Required evidence and review request

1. The gates.
2. The dry run.
3. The merged-lines list.
4. Item 5's record.
5. The review request at `.git-exclude/review-requests/020v-release-preparation/README.md`.

On approval the architect pushes, dispatches the release checks on that
commit, and writes the 0.9.0 readiness report with the run and its
`head_sha`.
