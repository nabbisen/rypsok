# RFC 020 — Project audit (milestone M8)

**Status.** Accepted (2026-10-29) for Part 1, the audit: milestone M8 was
accepted on 2026-10-27, this RFC adds no behavior, and the owner asked on
2026-10-29 for the handoffs to follow. Part 2 (the questions the audit
must answer) are not decisions: each is decided by the owner after the
report that answers it.
**Tracks.** Milestone M8 (`ROADMAP.md`), release 0.9.0. Requirements: all,
by audit; REQ-MAIN-006 (contract changes announced), REQ-TEST-012,
REQ-PERF-001 to REQ-PERF-005, REQ-DEP-005, REQ-RUST-002.
**Source.** `.git-exclude/reviewed/arch-001-prepare/README.md` (the first
independent review: ids U, B, S, C, P below), its reply
`REPLY.md`, and `.git-exclude/reviewed/arch-002-review-019/M8-DEFERRED.md`
(the kept list, with what the review of RFC 019 added).
**Touches.** Nothing in the source for Part 1 except the documents of
handoff 020a. The fixes come after the audit report, as handoffs 020g and
later, each its own review.
**Handoffs.** [`020a`](../handoffs/020a-baseline-text/README.md) (known
text drift, fixed now), [`020b`](../handoffs/020b-network-audit/README.md),
[`020c`](../handoffs/020c-data-and-contract-audit/README.md),
[`020d`](../handoffs/020d-tests-and-code-audit/README.md),
[`020e`](../handoffs/020e-measurements/README.md),
[`020f`](../handoffs/020f-dependencies-and-ci-audit/README.md) (audits:
findings only).

## Problem and motivation

0.8.0 corrected what the first independent review found in a day. That
review read about a third of the source, ran the gates and probed the
edges; it said so, and listed what it did not read. M8 is the audit that
reads the rest, with the owner's two principles as the standard: the
design must end up clean, safe and secure, robust and sophisticated, and
APIs and UI/UX must not leave a user confused or mistaken.

Seven milestones were built and reviewed by one role (P3). The audit is
the first systematic check of the whole system by readers who did not
write it, and it is done with the organization the owner set up: the dev
team investigates and reports, the architect reviews and decides what the
fix list is, the owner decides what changes.

## Goals

1. An audit report (one document, `docs/src/maintainers/audit.md`, written
   by the architect from the reviewed findings) with a fix list ordered by
   severity, each fix tied to a finding, a requirement and a test.
2. Every area of the roadmap's M8 row covered: data models, networking,
   test adequacy, measurements of REQ-PERF, the dependency tree, dead code,
   drift from the RFCs, and the CI and release pipeline.
3. Every id of the kept list answered: fixed (020a), found real and put
   on the fix list, found harmless with the reason, or decided by the owner.
4. No finding without evidence a reader can reproduce, and no "all clear"
   without the list of what was checked.

## Non-goals

- No change of source in the audit handoffs. Fixing while auditing hides
  the finding from the reviewer and mixes two reviews in one diff.
- No redesign. A finding may propose a fix in one line; the design is the
  architect's, after the report.
- No new features and no 1.0.0 decision (the owner's, after real use).
- No contact with any third-party site except through the existing
  opt-in live checks, and no probe that the owner's rules would not allow a
  user to run against their own machine (D1 safety rules).

## Design

### D1. Method, common to every audit handoff

**Finding format.** One row per finding in a table with these columns, in
this order, and nothing in a column that belongs in another:

| Column | Content |
|---|---|
| id | `<area>-<n>`, e.g. `NET-3`; an id of the kept list (U7, S4 …) is cited in "seed" |
| fact | One or two sentences of what is true, no adjectives |
| evidence | `path:line` for code or text; the exact command and its output for a run |
| how | `ran` (you executed it), `read` (you read it, nothing executed), `grep` |
| reading | Your inference of the consequence, **labelled as inference** |
| severity | The rubric below, as a candidate; the architect sets the final one |
| fix | Optional, at most one line |

**Severity rubric.**

- **High**: an unauthenticated or steered caller can obtain data, reach a
  network target the policy denies, or make the process fail, with the
  default configuration; or a stated guarantee is false.
- **Medium**: the same with an unusual configuration, or a user is told
  something untrue or misleading by the product's own output or pages, or a
  requirement is not verified by any test.
- **Low**: drift between texts, a latent defect that no input reaches, a
  missing test for a behavior that is correct today.
- **Note**: an observation that needs no change (record it; the reviewer
  decides).

**Evidence rules.** A claim is `ran` only if you executed it in this
checkout and the output is in the report. A claim from reading says the
file and lines. A claim that something is absent says the search you made
(`grep` pattern and scope). Never write "appears", "seems" or "probably"
as a fact: write what you did not verify under "Not verified".

**Every audit report ends with four lists**, as the first review did:
(1) what you checked and found as documented (so the reviewer sees the
coverage); (2) what you did not read or run, and why; (3) the commands and
scripts you used, kept in the scratchpad and named; (4) questions for the
architect (not proposals).

**Safety rules.** Probes use loopback, reserved `.invalid` names, unroutable
literals or a closed port, and mock servers you start yourself. No
third-party site is contacted; the live checks stay opt-in and are not run
unless the handoff says so. Scratch code lives in the scratchpad, never in
the tree; the audit handoffs commit nothing except the report's place
(none, the report is a review request in `.git-exclude/`). No `cargo
publish`, tag, push or release.

**The reviewer re-performs.** For each report the architect re-runs at
least a fifth of the findings (all High and Medium that rest on `ran`) and
a sample of the "checked and found as documented" list; a report whose
sample does not reproduce is returned whole, not patched.

### D2. Order and independence of the handoffs

| Handoff | Kind | Needs |
|---|---|---|
| 020a baseline text | fix (documents only) | nothing |
| 020b network | audit | nothing |
| 020c data models and the contract | audit | nothing |
| 020d tests, dead code, drift from the RFCs | audit | 020a (the texts are the baseline) |
| 020e measurements | audit | nothing |
| 020f dependencies, CI, release pipeline | audit | nothing |

The dev team does them in the order a, b, c, f, d, e, one at a time, and
stops after each for the architect's review; findings of one report can
change the scope of the next, and the architect then amends that handoff
before it starts.

### D3. What happens after the reports

1. The architect reviews each report (four lenses), sets final severities
   and writes the **audit report** (`docs/src/maintainers/audit.md`,
   public): findings, fix list, what was checked, what was not.
2. The questions of Part 2 that a report answered go to the owner as
   decision requests with a recommendation each.
3. The fixes become handoffs 020g and later, grouped so each is
   independently reviewable (a defect fix is never grouped with a
   documentation change). Contract changes are announced in the changelog
   as REQ-MAIN-006 requires; they are why M8 is a minor, 0.9.0.
4. Exit: the roadmap's criteria, and every High and Medium finding is fixed
   or accepted by the owner in writing with the reason recorded in the
   audit report.

### D4. Part 2 — questions the audit must answer (not decisions)

Each is answered with evidence in the report named; the decision follows.

| Question | Seed | Report |
|---|---|---|
| Does a same-host redirect need a robots.txt re-check? (REQ-CMP-005; the architect leans yes) | S4 | 020b |
| Which IPv6 ranges does the operating system actually connect to that the built-in table admits? | S2 | 020b |
| Is a poisoned std lock reachable, and what is the cost of each panic site? | S9 | 020b, 020d |
| Can a `SearchResult` be built by deserialization without the boundary? If so, is that reachable from any input? | S6 | 020c |
| Which of the ten LIM rows with an operator range get a key, and which lose the range? | B8 | 020c |
| Is the comparison key of RFC 008 consistent with REQ-RANK-002's "never change resource identity"? | B13 | 020c |
| What should `check` print beyond limits, tools and engines, and what should warn? | U7 | 020c |
| Per-pattern override of the secret check; what else is a false positive? | S7 | 020c |
| Which T requirements are named by a test that does not verify them? | — | 020d |
| Do the measured numbers meet REQ-PERF-003 to -005, and do the LIM defaults still hold? | B7 | 020e |
| Which floating tools and bases, permissions and `continue-on-error` settings should be pinned or removed? aarch64 in the matrix? | C2, C3, C5, C6 | 020f |
| What does a pace of releases cost, and how long should fuzzing run? | C7 | 020f |
| Residuals recorded in 0.8.0 (SDK log lines with caller-chosen text; per-caller session quota; latency of a refused versus an unreachable name; the public book running ahead of releases) | review M3, N7, N8, B2, C1 | 020b, 020f |

## Compatibility

No behavior changes in Part 1. The fixes that follow may change the
external contract; each is announced under REQ-MAIN-006, and the release is
a minor (0.9.0) for that reason, as in 0.8.0.

## Security considerations

An audit that probes the system is itself an attack surface on whoever
runs it: the safety rules of D1 bound it. Findings are written for the
private review folder; the public audit report is written after the fixes
for High findings have landed, and names a High finding only as fixed.

## Tests

None in Part 1. Each fix handoff names its own tests; a High or Medium
finding is closed by a test that failed before the fix.

## Acceptance

The six handoffs are reviewed; the audit report is published; the fix list
is empty or accepted; 0.9.0 is released by the owner's authorization after
the reviewer's Go.

## Alternatives considered

- **The architect audits.** Rejected: the architect now writes and reviews
  the design (P3); the audit's independence is the reviewer's re-performance
  of the dev team's findings, not a second reading by the same role.
- **One handoff for the whole audit.** Rejected: unreviewable; a report that
  fails the sample would fail whole.
- **Fix while auditing.** Rejected (non-goals).
- **An external audit.** The owner's decision; not assumed. The audit report
  is written so that an external reader could repeat every finding.

## Risks

1. **A mid-capability reader misses what a deeper reader would find.**
   Mitigation: the seeds, the "checked" list that shows coverage, the
   reviewer's re-performance, and the mutation runs of 020d that do not
   depend on judgment.
2. **Findings turned into churn.** Mitigation: severity rubric, and the
   rule that fixes are grouped, reviewed and released as 0.9.0, not as
   releases per finding (C7).
3. **Private details in public texts.** Mitigation: D1 and the security
   section.
