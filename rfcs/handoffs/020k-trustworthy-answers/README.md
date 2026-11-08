# Handoff 020k — Answers a user can trust

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3). Owner
decision **D-7** (authorized 2026-11-03) and the wording findings of audit
020c/020b. Findings DAT-3, DAT-4 (Medium); DAT-17, DAT-18, DAT-19, DAT-20,
NET-6 (Low). The owner's second principle: an answer must not leave a user
confused or mistaken.

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md), RFC 003 (error object), RFC 010 (`engine_status`).
**Applicable requirements.** REQ-ERR-*, REQ-CTX-*, REQ-STATUS-*, REQ-DOC-003.
**Rules.** As 020h. Evidence to `.git-exclude/evidence/020k/`.
**Depends on.** 020h approved (it changes what a redirect answers; the Reading page is edited by both).

## Change scope

Tests first for each row (the failing output goes in the review request).
Messages stay **fixed templates** of at most LIM-I4 (200) characters, never a
URL, host or address (REQ-ERR). Check what `reference/stability.md` says about
the text of `message` and report; the wording below is not a field change.

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | **DAT-3.** `denied`/`scheme` has two causes and gets two messages (same kind and reason): a scheme that is not `http`/`https` (message: that only `https` URLs, and `http` where the operator allows it, can be fetched) and the failed upgrade (today's text, true only for this cause). Find which input reaches `Denial::Scheme` from `judge_url` and say so | `ftp://127.0.0.1/x` and the upgrade-failed case (019a A2) each assert their own message and the shared `reason` | `reading-a-response.md` and `tools.md`: the two forms |
| 2 | **DAT-4, D-7.** `ToolError::http_status(status)`: for 408, 425, 429 and 5xx the message ends "retry later"; for every other 4xx it says the site refused or does not have the page and that retrying the same URL will not help. `detail.status` unchanged. When the response carries `Retry-After` as delta-seconds (an integer, at most 3,600 s), set `detail.retry_after_ms`; ignore a date form. **The doc comment of `ErrorKind::Unavailable` stops saying "the caller may retry"**: it says whether retrying helps depends on `detail.reason`. No new kind, no new field | A table test over 404, 403, 410, 408, 425, 429, 500, 502, 503 with and without `Retry-After` | `reading-a-response.md` `unavailable` row and the error table: the exact rule; the regenerated output schema fixtures if a doc comment is in them (`req_mcp_011`) |
| 3 | **NET-6.** A fetch ended by its deadline says so in fetch terms (a second constructor `ToolError::fetch_deadline()`, same kind and `reason: deadline`: "the page was not retrieved before the deadline; retry later"); `deadline()` stays for search | Both messages asserted at their call sites | — |
| 4 | **DAT-17.** (a) `category` not served: the error lists only the categories the process serves (it lists four, two are served); (b) the URL-too-long message states the number (the effective LIM-F9) instead of "LIM-F9"; (c) the Reading page states which URL form gives `secret_in_url` (`user:password@host` and key-shaped text) and which gives `credential_in_url` (`user@host`): verify by running both | Tests for (a), (b); (c) a test that runs both URLs | `reading-a-response.md` |
| 5 | **DAT-18.** `engine_status` entries for `state: "disabled"` carry an optional `reason`: `disabled` (operator) or `no_credential`; absent for the other states. Additive, from a closed vocabulary, no secret. The output schema fixture, the design (§ engine_status), `tools.md`, `stability.md` (a new optional field is the allowed growth) and the test that asserts the key set are updated deliberately | A configuration with Brave and no key: `disabled`/`no_credential`; an engine disabled in the file: `disabled`/`disabled`; the key set of the other states unchanged | as listed |
| 6 | **DAT-19.** The `web_fetch` description in code reads "starting at `start_char`" like the design; the design's parameter table lists `max_chars` minimum 500 and says `language` appears only when two or more languages are configured | The existing description tests updated | `external-design.md` |
| 7 | **DAT-20.** The doc of `NoticeCode::ExtractionPartial` says what the code and the Reading page say: the document was larger than rypsok reads (nodes or depth) and was read in part | — | regenerate the schema fixture if it carries the doc |
| 8 | **Changelog** `[Unreleased]`: Changed (the messages, the retry rule, the new optional field) and Fixed | — | `CHANGELOG.md` |

## Explicit non-change scope

No new `ErrorKind` or `DetailReason`; the kinds' meanings; the notice
vocabulary; other messages. DAT-21 and DAT-23 are audit notes only.

## Prohibited shortcuts

A message that contains the URL, host or status text of the site. Changing
`reason` values. A fixture edited by hand instead of regenerated (use the
documented regeneration command).

## Required evidence and review request

The gates; failing-first output; the review request at
`.git-exclude/review-requests/020k-trustworthy-answers/README.md` with a
"Questions" section (including what `stability.md` says about `message`).
