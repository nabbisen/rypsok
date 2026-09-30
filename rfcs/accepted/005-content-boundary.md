# RFC 005 — Content boundary

**Status.** Accepted (2026-09-30) — implementer may start after RFC 003.
**Tracks.** Milestone M1 (titles, snippets, renderings); M3 adds page
content (RFC 012). Requirements REQ-SEC-001 to -004, REQ-SEC-015, -016,
REQ-CTX-002 to -007, REQ-CTX-010, REQ-TEST-007, REQ-TEST-013; external
design §9; threats THR-01 to THR-04.
**Touches.** `rypsok-core::boundary`, the hostile corpus.
**Handoff.** [`../handoffs/005-content-boundary/README.md`](../handoffs/005-content-boundary/README.md)

## Summary

External text passes one module, `boundary`, before it enters a response.
The module cleans it, bounds it, removes what a reader cannot see, reports
every change through a notice, and never rewrites visible text. The
structural separation of the response and the JSON rendering do the rest:
no string can pretend to be rypsok's own statement.

## Design

### Where the boundary sits

```text
driver ──raw title, snippet──▶ boundary::clean_text ──▶ SearchResult
extractor ──raw content──────▶ boundary::clean_content (M3) ──▶ FetchResponse
                                       │
                                       └──▶ notices, counters
```

Drivers deliver raw text (REQ-ENG-012). Nothing reaches a `SearchResult` or
`FetchResponse` field that carries external data except through the
boundary. A unit test enforces it by constructing the types only through
the boundary's constructors (the fields are `pub(crate)` with builders).

### `clean_text` (titles and snippets, M1)

In this order (REQ-CTX-004):

1. **Markup removal.** Strip HTML tags; keep their text. Decode HTML
   entities, including numeric ones. (Provider snippets carry
   `<span class="searchmatch">`, `<strong>` and entities.)
2. **Unicode normalization** to NFC.
3. **Invisible and control characters removed** (REQ-SEC-015): every code
   point of category `Cc` except the ones replaced in step 4, every code
   point of category `Cf` (this includes U+200B–U+200F, U+202A–U+202E,
   U+2060–U+2064, U+2066–U+2069, U+FEFF), the tag characters U+E0000 to
   U+E007F, U+00AD soft hyphen, and unassigned code points. The count of
   removed characters is reported through the notice `characters_removed`
   when it is not zero.
4. **Whitespace.** Every sequence of whitespace, including newlines and
   tabs, becomes one space; leading and trailing whitespace is removed.
5. **Bound.** Cut to LIM-S10 (title) or LIM-S9 (snippet) at a character
   boundary, preferring the last word boundary within the final 20
   characters; report `title_truncated` or `snippet_truncated` with the
   count of affected results.

Titles and snippets are never rewritten beyond these steps (REQ-SEC-003).

### `clean_content` (fetched pages, M3)

The same steps except that step 4 keeps single newlines and paragraph
breaks, and step 5 cuts to `max_chars` from `start_char` and sets
`truncated` and `next_start_char`. Exclusion of non-rendered content
(REQ-CTX-005) happens in the extractor before the boundary: elements with
`hidden`, `aria-hidden="true"`, inline `display:none` or
`visibility:hidden`, `<script>`, `<style>`, `<template>`, `<noscript>` and
comments are dropped there; RFC 012 specifies it.

### Instruction-like patterns (REQ-SEC-003)

A small, reviewable list of case-insensitive phrases (initially: "ignore
previous instructions", "ignore all previous", "disregard the above",
"you are now", "system prompt", "do not tell the user") is matched against
cleaned text. A match adds the notice `instruction_like_content` with the
count of affected results, and increments a counter. The text is not
changed. When `search.drop_flagged_results = true` (RFC 004), matching
results are
removed and `results_dropped` with reason `instruction_like_content`
reports the count. The list lives in one file with a comment that it is a
heuristic, not a boundary (threat register, residual risks).

### Structural separation and the JSON rendering

`SearchResponse` and `FetchResponse` are the only carriers of external
text. Their fields are named in external design §3 with a trust class; the
JSON rendering is `serde_json::to_string` of the structure. JSON escaping
makes it impossible for a string to close itself or to introduce a field
(REQ-SEC-002). No other rendering exists before DR-008.

### Fixed vocabularies (REQ-SEC-016)

`NoticeCode`, `Reason`, `ErrorKind`, `EngineErrorClass` and `EngineState`
are Rust enums with `#[serde(rename_all = "snake_case")]`. No `String`
field exists in the envelope, in `ToolError`, in `EngineStatus` or in a log
event except `ToolError::message`, which is built from templates without
external input (RFC 003).

### Determinism

All steps are pure functions of their input; the pattern list is a constant;
no randomness (REQ-ARCH-004).

### Tests (REQ-TEST-007, REQ-TEST-013)

The hostile corpus of RFC 002 gets, in M1: a title containing `"]`, `}`
and a fake `"outcome":"ok"`; a snippet with each removed character class;
a snippet with a closing envelope imitation; NFC edge cases; a Japanese
snippet at the length bound; each instruction-like phrase in three casings.
Property tests: cleaning is idempotent; output length is within bound;
output contains no removed code point; JSON round-trips.

## Alternatives considered

- **Envelope markers such as `<external_search_result>`.** Rejected: a
  marker inside a string can be forged; the structure and JSON escaping
  cannot (R-03).
- **Random nonces as delimiters.** Rejected: non-deterministic, and still
  a string convention.
- **Rewriting or removing instruction-like text.** Rejected: it damages
  legitimate content and gives false assurance (D-11).

## Open questions

- The exact phrase list. It is a reviewable constant; the first list above
  is the starting point.
