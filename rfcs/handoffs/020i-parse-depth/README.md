# Handoff 020i — The nesting bound is enforced while parsing

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3). Owner
decision **D-3** (authorized 2026-11-03): **one** depth bound, LIM-F11 (256),
enforced while parsing; rendering inherits it. Finding NET-5 (Medium).

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md), RFC 012.
**Applicable requirements.** REQ-PAGE-006, REQ-SEC-011, REQ-SEC-012.
**Rules.** As 020h. Evidence to `.git-exclude/evidence/020i/`.
**Depends on.** Nothing.

## The problem in one paragraph

`html5ever` keeps a stack of open elements and scans it on many tokens, so
parse time grows with the square of the nesting depth: in the audit 10,000
nested `<div>` took 0.12 s, 40,000 took 1.9 s, 80,000 took 7.8 s (release),
while the node bound (100,000) stopped nothing; the blocking task is not
cancelled at the deadline and held the two parse permits (LIM-C5) for 10 to 13
s, so a small page on another host waited 10 s. A bound in the sink alone
cannot help (html5ever does the scanning); the **feed loop must stop** when
the depth bound is passed, as it already stops at the node bound.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | In `html/dom.rs`, `BoundedSink` tracks the depth of every element it attaches (the parent's depth + 1; the document is 0). When an element would be deeper than `max_depth` (the LIM-F11 value, passed in like `max_nodes`), the sink records `depth_exceeded` and the feed loop stops at the next chunk boundary, exactly as for the node bound; the document is `partial`. The tree built so far is kept and extracted. No recursion is introduced | Unit tests in `dom` tests: a document nested to exactly the bound parses in full (`partial` false); one level deeper is `partial`; 1,000,000 nested `<div>` stops after at most the bound plus one chunk of elements (assert the node count, not the time); depth and node bounds together; text before the deep spot is extracted | RFC 012: append an amendment (the "linear time" sentence was wrong; the depth is bounded while parsing) |
| 2 | `fetch.rs`/`extract` pass `limits.html_max_depth` (the existing LIM-F11 field or constant) to `parse`. The render-time depth check in `markdown.rs` stays (defense in depth) and gets a comment saying the parse bound makes it unreachable for pages that came through `parse`. A page cut by the depth bound answers like a node bomb: content before the cut, `extraction_partial` notice, never a different error unless nothing could be extracted (then the existing `unsupported_content`/`extraction_failed`) | Integration test with a loopback server: the audit's `starve.py` shape written afresh: two pages nested 100,000 deep on two hosts fetched at once, then a tiny page on a third host: **the tiny page is answered in under 2 s** (it took 10 s); the deep pages end with `partial` or `extraction_failed`, not `deadline`; the fixture `11-deep-nesting` expectation is updated to what now happens and the change is explained in the review request | `limits-and-defaults.md` LIM-F11 row: "HTML nesting depth; enforced while parsing, a deeper page is read in part; fixed" (D-9 will make the other nine rows consistent); `reading-a-response.md` and `tools.md` `extraction_partial` row mention depth; changelog under Changed and Fixed |
| 3 | **Measure, in a release build,** parse time for a page nested to 256 (the bound) and for 100,000 nested: the first must be far below the deadline (state it), the second must end in well under a second. Put the figures and the script in the review request and `.git-exclude/evidence/020i/` | A test that fails if parsing a 100,000-deep page takes more than 5 s in debug (a generous guard, not a benchmark) | — |

## Explicit non-change scope

The node bound (LIM-F10), the byte bound, the permits, the deadline; cancelling
the blocking task at the deadline (noted by the audit; with the bound the task
ends in milliseconds; do not add a cooperative-cancel mechanism). No new LIM
row, no configuration key.

## Prohibited shortcuts

Counting depth after the parse. A test that measures only time on a fast
machine. Changing the meaning of `partial`.

## Required evidence and review request

The gates; failing-first output (the starvation test fails on the unmodified
tree); the fuzz target `html_document` run for 60 s on the result (state the
command; `cargo-fuzz` is installed on the dev machine or use the scheduled
run); the review request at
`.git-exclude/review-requests/020i-parse-depth/README.md`.
