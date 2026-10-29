# Handoff 020a — The baseline text made true

**Created 2026-10-29.**

**Task title.** Correct the known drift between the baseline texts, the RFCs
and the delivered system, in documents only.
**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md).
**Applicable requirements.** REQ-DOC-001, REQ-MAIN-006. **Source findings.**
`arch-001-prepare` B1, B2, B3 (verify), B6, B7, B10, B12, B15.
**Rules.** English for documents; no source change; commit messages one-line
digest, optional body of at most 400 characters, no `Co-authored-by`; push
only after the architect's approval, and only your own commits.
**Depends on.** Nothing.
**Purpose.** The texts the audit will read as its baseline say what the
system does. The items here are facts already established; there is no
judgment to make. Where you find that an item is not as described (it was
fixed, or the text is right and the finding wrong), do not change it: say so
in the review request.

## Change scope

| # | Required change | Check |
|---|---|---|
| 1 | **B1.** `docs/src/maintainers/external-design.md` §18 (the traceability table): add a row for every requirement heading in `requirements.md` that has none (the first review found REQ-DOC-001..004, REQ-ENG-014, REQ-ENG-015, REQ-SEC-022, REQ-SEC-023, REQ-TEST-018; RFC 019 added REQ-SEC-024; re-derive the list by script, do not trust this one). Each row names the section that specifies the requirement. Add one sentence above the table that says at what granularity rows map (per requirement or per group) so a reader is not misled | The script lists no requirement without a row; the sentence exists |
| 2 | **B2.** The sample configuration in the external design uses `default_language`, which is not a key (the delivered key is `languages`, the first is the default). Correct it, and run every other sample configuration block in `docs/src/maintainers/*.md` through `rypsok check` (a scratch file in the scratchpad) and list in the review request which blocks pass | Each block that is meant to be valid passes `check` |
| 3 | **B3.** Verify that the HTTP gate contract (path `/mcp`, the statuses by owner, the three protocol headers) is in the external design (RFC 019 D6 says it was written there). Change nothing if it is | Say which lines |
| 4 | **B6.** `requirements.md`, REQ-SEARCH-004: the sentence "…where an engine supports the selection, and it / Engines that cannot honor…" is broken. Repair it so it reads as one requirement (what it selects; the engines that cannot honor it are skipped and a notice names them). Do not change the meaning, the milestone or the `T` marker | `tools/check-requirements-coverage` still prints the same count |
| 5 | **B7.** "M0 to M5" in `requirements.md` and `glossary.md` becomes "M0 to M8"; `limits-and-defaults.md` says the values "are provisional until the M1 and M3 measurements": the measurements were taken and left the values unchanged (`.git-exclude/reviewed/m1/README.md` §3 and `.git-exclude/reviewed/012-fetch-pipeline/README.md` §4); say that, with the date, and that M8 re-measures (handoff 020e). Do not change a value | `tools/check-limits` unchanged |
| 6 | **B10.** RFC 016 says `check` prints to stdout; `rypsok check` writes its report to stderr (`configuration.md` and `cli.rs` say so). Append an amendment to `rfcs/done/016-stable-contract.md` (RFC 000 amendment form: a dated section at the end) stating the delivered stream. Do not edit the earlier text | The amendment exists |
| 7 | **B12.** RFC 015 says the live check shows drift "within a day", the schedule is weekly; and RFC 015 says robots.txt for scraping "would always disallow" while the admission record of the same day (`docs/src/maintainers/engines/duckduckgo.md`) says `html.duckduckgo.com/robots.txt` is `Allow: /`. Append an amendment to `rfcs/done/015-scraping-drivers.md` correcting both; read the admission record and `scheduled.yml` first and state what they say | The amendment exists |
| 8 | **B15.** `rfcs/handoffs/004-*/README.md` says an invalid configuration "exits 2" (RFC 004, REQ-OPS-001 and the code: 1) and cites REQ-CFG-006, REQ-OPS-004 (do not exist) and REQ-CFG-002 (merged into REQ-ENG-004). Handoffs are records: do not rewrite; add a "Known errors" note at the top that lists the corrections | The note exists |

## Explicit non-change scope

No source file (`crates/`, `tools/`, `.github/`, `Cargo.*`); no value of the
limits table; no requirement's meaning, milestone or verification marker; no
edit of an earlier RFC text except by an appended amendment; B4, B5, B9, B11,
B14 are already amended (verify only: they are on `main`, commit `efe4788`).

## Prohibited shortcuts

No bulk reformatting; no "while here" edits; no table row that cites a
section you did not read.

## Required evidence and review request

`mdbook build docs` clean; `cargo test --workspace` (the documentation tests
read these files); `tools/check-requirements-coverage`;
`tools/check-limits`. The review request at
`.git-exclude/review-requests/020a-baseline-text/README.md`: per item the
commit, the lines changed, the check run with its output; and the items you
found already true or wrong. One commit per item or per small group, no push
until approved.
