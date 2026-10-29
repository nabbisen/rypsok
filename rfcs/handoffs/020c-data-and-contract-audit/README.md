# Handoff 020c — Audit of the data models and the contract

**Created 2026-10-29.** An audit handoff: findings only, no source change.

**Task title.** Audit the model, configuration and driver types, the tool
schemas and the error surface against the external design, the stability
page and what a user is told, and report findings in the format of RFC 020 D1.
**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md),
D1 and D4.
**Applicable requirements.** REQ-CTX-*, REQ-SEC-003, REQ-CFG-*, REQ-RANK-*, REQ-ERR-*,
REQ-OPS-*, REQ-STATUS-*, REQ-MAIN-* (Appendix A of `requirements.md` lists the
merged ones), the stability page. **Seeds.** `arch-001-prepare` B8, B9, B13, S6, S7, S8,
U7; `M8-DEFERRED.md` rows U7, S7, "Per-caller session quota".
**Rules.** As 020b. **Depends on.** Nothing (020a corrects some texts this
audit reads; read the current ones).
**Purpose.** Show where a type, a key or an answer differs from what the
design and the pages promise, and where a user could be confused or
mistaken by what the product says or accepts.

## Change scope (what to audit)

| # | Area | Questions the report answers |
|---|---|---|
| 1 | **Types against the external design.** `model*`, `config*`, `engine`, driver types | A table, type × field: the external design's name, type and default; the code's; equal or not. Cover every type the external design names and every public type of `rypsok-core`; for public types it does not name, say so (these are the "public items" the stability page freezes: list which are frozen and by what words). Which are `#[non_exhaustive]` (B9: RFC 016 listed `Outcome`, `ErrorKind`, `EngineState`, `TimeRange`, `CallError`; CHANGELOG 0.6.0 and the code keep them exhaustive; RFC 016 was amended in 0.8.0 or is to be: say what the code does today) |
| 2 | **Construction invariants (S6).** Can a value exist that the boundary or a validator should have prevented? | Specifically: `SearchResult` derives `Deserialize` with a skipped proof field: write a scratchpad test that builds one by deserialization and say whether any input of the running system (config, a tool argument, a cache file, an engine response) reaches that path. Same for `Host`, `Limits`, `Secret`, `Config` (validated at load?), and every type whose docs say "the only way to build X is Y". List each such claim and its enforcement (private field, `non_exhaustive`, or only convention) |
| 3 | **Configuration.** The schema, the refusal of unknown keys, the keys behind the limits (B8) | Every key of `configuration.md` against the code: present, default equal, range checked, error message tells the user the key. Every LIM row with an operator range (the ten the first review listed — LIM-S11, S12, F9 to F13, C9, O1, O2 — and any other you find): does a key exist? what does the code do with an out-of-range value? what does the table promise? Report the mismatches; do not choose. Try 25 invalid configurations (wrong type, negative, huge, empty string, duplicate engine, an unknown engine, a secret literal in the file, CRLF, a BOM) and for each report: exit code, stream, message; is the message one a user can act on? |
| 4 | **`check` (U7).** What the command prints and what it should warn about | A table: configuration section × printed by `check` (yes/no). List the configurations a user is likely to get wrong that `check` accepts silently (plaintext listener off loopback with callers; `legacy_revisions` default; `allow` rules that open the policy; a rule with a trailing dot; a keyed engine with no key). State what the user would see in each case today. Propose nothing |
| 5 | **Tool schemas and answers.** `tools/list` against what the tools return | For each tool: the input schema (required, enums, bounds) against the code's validation; the output envelope against the schema in the external design; every `Outcome` and `ErrorKind` value: is it reachable (name the test or the probe), documented in "Reading a response", and shown with the words the page uses? One row per kind. Look for answers a user could misread: the same words for two causes, a kind that suggests a wrong action (retry vs do not retry), a notice whose text names the wrong thing |
| 6 | **Ranking and URLs (B13).** RFC 008 / REQ-RANK-002 | State the comparison key's rules (fold `http`→`https`, strip `www.`, trailing slash, sort query parameters), then REQ-RANK-002's list and its sentence "never change resource identity"; give three real pairs of URLs the key treats as equal that can be different resources (e.g. `http://x/` and `https://x/` serving different content; `?a=1&b=2`/`?b=2&a=1` where order matters), and say which URL the user is shown. Report; do not choose |
| 7 | **Text handling (S7, S8).** The secret check and the boundary | S7: list ten strings of ordinary words that the secret check refuses and ten that it should refuse and passes (build a corpus from the patterns in `privacy.rs`; run it through the check via a scratchpad test); give the pattern for each case. S8: list the Unicode categories the boundary removes and the scripts or emoji sequences whose text changes; is the change reported (`characters_removed`) and does the page tell the author? Report; do not choose |
| 8 | **Stability.** `reference/stability.md` against the code | Each promise (statuses, protocol headers, kinds, fields, exit codes, file formats): the code that keeps it, the test that fails if it breaks (name or "none"). A promise with no test is a finding (Medium) |

## Explicit non-change scope

No source file, test, workflow or document is changed. Scratch tests live in
the scratchpad (a copy of the crate, or a test in a scratch crate depending
on `rypsok-core` by path). No commit, no push.

## Prohibited shortcuts

Comparing names without comparing types and defaults. "All present" without
the table. A corpus for S7 that you took from the code's own tests only.

## Required evidence and review request

`.git-exclude/review-requests/020c-data-and-contract-audit/README.md` in the
format of RFC 020 D1: findings (`DAT-n`), then the tables of areas 1, 3, 4, 5
and 8, then the four closing lists.
