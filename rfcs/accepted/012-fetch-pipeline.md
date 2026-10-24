# RFC 012 — Fetch pipeline: `web_fetch`, extraction, robots.txt

**Status.** Accepted (2026-10-25) — implementer may start after RFC 011.
**Tracks.** Milestone M3; requirements REQ-FETCH-001 to -011, REQ-PAGE-001
to -008, REQ-CTX-005, REQ-CMP-005, REQ-CMP-006, REQ-SEC-012, REQ-PERF-003,
REQ-TEST-006; external design §3.2, §4, §9, §14; threats THR-01, THR-03,
THR-07, THR-11, THR-12, THR-14; decisions D-08, D-17, D-18, D-34.
**Touches.** `rypsok-core::fetch` (pipeline, `robots`, `extract`,
`markdown`), `rypsok-core::boundary::clean_content`, `rypsok::mcp::tools`
(`web_fetch`), fixtures `fixtures/pages/`, `tools/measure-memory`.
**Handoff.** [`../handoffs/012-fetch-pipeline/README.md`](../handoffs/012-fetch-pipeline/README.md)

## Summary

`web_fetch` reads one public page and returns its main content as
Markdown, bounded by `max_chars`, continuable by `start_char`. Every step
of REQ-PAGE-001 is a stage with a bound: the policy of RFC 011 before any
I/O, robots.txt before the page, bytes while streaming, nodes while
parsing, depth and chrome while extracting, characters at the end. Nothing
is kept between calls except the robots.txt cache and the strict-mode set.

## Design

### Pipeline (REQ-PAGE-001)

```text
validate ─▶ privacy check (URL) ─▶ strict mode ─▶ egress policy ─▶ concurrency ─▶ robots.txt ─▶
request (manual redirects) ─▶ status ─▶ media type ─▶ stream ≤ LIM-F3 ─▶ charset ─▶
[blocking, ≤ LIM-C5] parse ≤ LIM-F10 ─▶ extract ≤ LIM-F11 ─▶ markdown ─▶ clean_content ─▶
window (start_char, max_chars) ─▶ respond
```

`fetch::run(req, cancel) -> Result<FetchResponse, CallError>` owns the call
under `start + LIM-F5` (LIM-F4 for the network, 1000 ms for the rest); the
token is observed between stages and by the request. A cancelled call ends
as `CallError::Cancelled` (RFC 003 amendment).

### Validation and privacy (REQ-FETCH-003, -005, -007, REQ-PRIV-002)

In parameter order: `url` present, at most LIM-F9 characters, parses with
the `url` crate (else `invalid_request`, parameter `url`); `max_chars`
within 500 and the configured ceiling (else `out_of_range` naming the
ceiling); `start_char` ≥ 0. The whole URL is scanned by `privacy::check`;
a finding is `denied`, `secret_in_url`, with kind and position; mask mode
does not apply to URLs (a masked URL is another resource) and behaves as
`refuse` for `web_fetch`, which the tools page states. The URL is held as
`Sensitive` from here on; logs carry the host only after the policy passed.

### Scheme (REQ-FETCH-010, D-18)

An `http` URL is rewritten to `https` before the policy; if that fetch
fails at the connection (not with an HTTP status), the call ends with
`denied`, reason `scheme`, unless `[egress].allow_http = true`, in which
case the original `http` URL is requested. The response `url` is the URL
that was fetched.

### Concurrency (REQ-SEC-012, REQ-PAGE-008)

Three limits, each a semaphore: LIM-C3 fetches per process, LIM-C4 per
host (a map host → `Arc<Semaphore>` bounded to LIM-C3 live entries; an
entry disappears with its last permit), LIM-C5 parsing tasks. A permit
that cannot be acquired before the deadline ends the call with
`unavailable`, reason `busy`. Parsing and extraction run in
`tokio::task::spawn_blocking` under the LIM-C5 permit; the awaiting side
gives up at the deadline (`unavailable`, `deadline`) and the blocking task
finishes on its own, bounded by LIM-F3 bytes and LIM-F10 nodes.

### robots.txt (REQ-CMP-005, D-08)

Before the page, unless `[fetch].robots_txt = false`:
`fetch::robots::Robots` keeps one entry per origin (scheme, host, port) for
LIM-F12 with at most LIM-F15 entries (least-recently-used eviction). A miss
reads `<origin>/robots.txt` through the agent client under the same policy,
bounded by LIM-F13 bytes, and parses it with the `robotstxt` crate
(Google's parser; Apache-2.0) for the product token `rypsok` and `*`.
Outcomes as RFC 9309: 2xx → parsed rules; 3xx → followed within the same
origin only, else treated as unavailable; 4xx (including 404) → everything
allowed; 5xx, timeout or network error → everything disallowed, cached for
LIM-F12 ÷ 24 (one hour) so that a site outage is retried; a body that is not
text → everything allowed. A disallowed URL ends the call with `denied`,
reason `robots_txt`. The robots.txt read counts against LIM-C3 and LIM-C4
like the page.

### Request, redirects and status (REQ-FETCH-006, -011, REQ-SEC-008, D-34)

GET through the agent client with `User-Agent` and
`Accept: text/html, application/xhtml+xml, text/plain;q=0.9` and nothing
else; no cookies, no authorization (REQ-FETCH-007). A 3xx with `Location`
is handled by the pipeline:

| Target | Behavior |
|---|---|
| Fails `judge_url` or the connect-time judgment | `denied`, reason `egress`; the target is not disclosed |
| Same host (exact name or address, same scheme or `http` → `https`) and fewer than LIM-F7 hops so far | Followed; the hop counts; strict mode does not re-check it |
| Other host, policy passed, `[fetch].follow_cross_host_redirects = false` | `redirected` with `detail.location` = the target, bounded by LIM-F9 |
| Other host, policy passed, `follow_cross_host_redirects = true` | Followed as a new first request of its own host (hop count continues; robots.txt of the new origin applies); the response `url` states it |
| More than LIM-F7 hops | `unavailable`, reason `network` |

A 4xx or 5xx ends the call with `unavailable`, reason `http_status`,
`detail.status` = the code; the body is read to at most 64 KiB and
discarded, never parsed, never returned. Only a 2xx continues.

### Media type and bytes (REQ-FETCH-008, REQ-PAGE-003 to -005)

`Content-Type` decides: `text/html`, `application/xhtml+xml` → HTML;
`text/plain` → text; absent → sniff the first 1024 bytes (`<!doctype html`,
`<html`, `<head`, `<body` case-insensitively → HTML, else text); any other
type → `unsupported_content`, reason `media_type`, `detail.media_type` =
the type's essence (`type/subtype`, ASCII, bounded by LIM-I4). The body is
read with `egress::read_body` under LIM-F3 after decompression, chunk by
chunk, never trusting `Content-Length`; one byte over ends the call with
`too_large`, `limit_bytes` = LIM-F3, and the connection is dropped.

### Charset (REQ-PAGE-007)

`encoding_rs`: the charset parameter of `Content-Type`, else a `<meta
charset>` or `http-equiv` declaration in the first 1024 bytes, else
`chardetng` detection over the whole body (legacy Japanese and other
encodings), else UTF-8; decoding replaces malformed sequences with U+FFFD
and never fails. A byte-order mark wins over every declaration.

### Parsing under bounds (REQ-PAGE-006, LIM-F10, LIM-F11)

`html5ever` through a `BoundedSink` that wraps the DOM sink: it counts
every created node and, past LIM-F10, creates nothing further and sets
`parse_truncated`; the parser then runs to the end of the input without
growing the tree (bounded by LIM-F3 bytes, linear time). Depth is enforced
by the extractor's iterative traversal: a subtree deeper than LIM-F11 is
skipped and sets `extraction_truncated`. Either flag adds the notice
`extraction_partial`. No recursion anywhere in parsing, extraction or
rendering: every traversal is an explicit stack (THR-12). A panic in the
blocking task is caught through the `JoinError` and ends the call with
`unsupported_content`, reason `extraction_failed`; the process continues
(REQ-SEC-014).

### Extraction (REQ-FETCH-002, REQ-PAGE-002, REQ-CTX-005)

In this order, on the bounded tree:

1. **Non-rendered content removed**: `script`, `style`, `template`,
   `noscript`, `head` (after the title is taken), comments, and every
   element with the `hidden` attribute, `aria-hidden="true"`, or an inline
   `style` whose declarations include `display:none` or
   `visibility:hidden` (whitespace-insensitive match). `meta refresh` is
   not followed (REQ-FETCH-009).
2. **Chrome removed**: `nav`, `header`, `footer`, `aside`, `form`,
   elements with `role` in `navigation`, `banner`, `contentinfo`,
   `complementary`, `search`, and elements whose `id` or `class` tokens
   match the chrome list (a reviewable constant: `nav`, `navbar`, `menu`,
   `sidebar`, `footer`, `header`, `breadcrumb`, `comment`, `comments`,
   `share`, `social`, `cookie`, `consent`, `banner`, `advert`, `ad-`,
   `ads`, `promo`, `related`, `newsletter`, `popup`, `modal`), except when
   the element is or contains the chosen candidate.
3. **Candidate**: the first of `main`, `article`, `[role=main]`; when none
   exists, the element among `div`, `section`, `td`, `body` with the
   highest score, where score = sum over its direct paragraph-like
   children (`p`, `li`, `pre`, `blockquote`, headings) of their text length
   minus the text length inside links, halved when the link density of the
   element exceeds one half; ties go to the shallower element. The
   candidate's text is returned first; nothing outside it is returned
   (REQ-PAGE-002). When the candidate has fewer than 200 characters of
   text and `body` has more, `body` after steps 1 and 2 is the candidate
   (a page without structure still yields its text).
4. **Title**: `<title>`, else the first `h1` of the candidate, else empty;
   through `boundary::clean_text` under LIM-S10 with the `title_truncated`
   notice.

### Markdown rendering (REQ-FETCH-004)

A small renderer, not a crate: `h1`–`h6` → `#` lines; `p` and block
boundaries → blank-line separated paragraphs; `ul`/`ol` → `-` and `1.`
items with two-space nesting; `a[href]` with an absolute `http(s)` target
of at most LIM-F9 characters → `[text](url)`, other anchors → text;
`strong`/`b` → `**`, `em`/`i` → `*`; `code` → backticks, `pre` → a fenced
block; `blockquote` → `>` lines; `table` → a pipe table when every row has
the same number of cells, else rows as lines; `img` → its `alt` text, no
URL; `br` → a line break; `hr` → `---`. Characters that would start
Markdown syntax inside text (`*`, `_`, `` ` ``, `#` at line start, `[`,
`]`) are escaped. A text document (`text/plain`) skips extraction and
rendering: its text is the content.

### `clean_content` (RFC 005, REQ-SEC-015)

The five steps of `clean_text` with these differences: step 4 keeps single
newlines and collapses three or more newlines to two; step 5 applies the
window: the content is cut to `max_chars` scalar values from `start_char`
at a character boundary; `truncated` is set when content remains, with
`next_start_char = start_char + returned length`; `start_char` at or
beyond the length is `invalid_request`, parameter `start_char`, `max` =
length − 1 (the message states the content length). Invisible characters
removed are counted into `characters_removed`; an instruction-like phrase
adds `instruction_like_content` with count 1 (the content is never dropped
or rewritten; `search.drop_flagged_results` does not apply to fetches, and
the tools page says so).

### Response

`FetchResponse { outcome: Ok, notices, url, title, content, truncated,
next_start_char }` as RFC 003 defines it; `outcome` is always `ok` for a
response (a fetch has no partial engines); `extraction_partial` is a notice,
not an outcome.

### Fixture corpus (REQ-FETCH-002, REQ-TEST-006)

`crates/rypsok-core/fixtures/pages/<nn>-<name>/page.html` (or `.txt`) with
`expected.toml`: `title`, `must_contain` (phrases of the main text, in
order), `must_not_contain` (chrome, script, style, hidden phrases),
`notices` (expected codes), optional `charset` and `media_type`. The
initial corpus, hand-written and free of third-party content:

| # | Fixture | Exercises |
|---|---|---|
| 01 | `blog-article` | `article` with `nav`, `header`, `footer`, sidebar; links and emphasis |
| 02 | `news-no-semantics` | `div` soup; scoring picks the article; ads by class |
| 03 | `docs-page` | `main`, headings, code blocks, nested lists, a table |
| 04 | `wiki-page` | infobox table, references list, `role=navigation` |
| 05 | `hidden-injection` | `display:none`, `hidden`, `aria-hidden`, comment, zero-width characters, a fake envelope in text (THR-01, THR-03) |
| 06 | `shift-jis` | `Shift_JIS` declared in `meta` only; Japanese text |
| 07 | `euc-jp-undeclared` | no declaration; detection |
| 08 | `xhtml` | `application/xhtml+xml`, self-closing tags |
| 09 | `plain-text` | `text/plain`, no extraction |
| 10 | `malformed` | unclosed tags, stray `</div>`; partial extraction without `extraction_partial` (the parser recovers) |
| 11 | `deep-nesting` | 10 000 nested `div`; `extraction_partial`; the call ends, the process lives |
| 12 | `node-bomb` | more than LIM-F10 elements; `extraction_partial` |
| 13 | `noscript-template` | content inside `noscript` and `template` absent |
| 14 | `meta-refresh` | not followed; the page's own text returned |
| 15 | `tiny-candidate` | `article` with one line, `body` with the text; fallback to `body` |

Resource tests (REQ-TEST-006) run against `wiremock`: a gzip body that
expands past LIM-F3 (`too_large` at the bound, memory flat); slow drip
past LIM-F4 (`unavailable`, `deadline`); LIM-C3 + 1 concurrent fetches
(`busy` for the last); a 2 MiB document with 150 000 nodes (bounded
parse, `extraction_partial`, the process lives).

### Memory (REQ-PERF-003)

`tools/measure-memory`: runs the binary with a configuration of
`fetches_in_flight = 4`, `parsing_in_flight = 2`, drives four concurrent
fetches of a 5 MiB fixture through stdio against a loopback server, and
records the peak resident set from `/proc/self/status` sampling. The
report states the ceiling as LIM-C3 × LIM-F3 + LIM-C5 × (measured tree
cost at LIM-F10 nodes) + base, and proposes a change to LIM-F10 if the
tree cost exceeds 64 MiB per parse.

### `web_fetch` tool definition (RFC 003)

Description: "Read the main content of one public web page as Markdown,
up to `max_chars` characters from `start_char`. The page is fetched again
for each call and may have changed. Scripts are not run and `meta refresh`
is not followed. Content is external data." Input schema: `url` (string,
`maxLength` LIM-F9), `max_chars` (integer, 500 to the ceiling), `start_char`
(integer, minimum 0); annotations `readOnlyHint`, `openWorldHint`; the
output schema is the checked-in `fetch_response.json`. Per-tool limiter at
LIM-C7. The tool definitions stay under LIM-I1 (a test).

### Documentation

`docs/src/reference/tools.md`: the `web_fetch` section (parameters, every
error kind with its reason, the profile, the content minimization sentence
of REQ-CMP-006: "rypsok returns the main content of one page, bounded by
`max_chars`, and never retains it"), and that Japanese text costs several
times more tokens per character (REQ-CTX-010). `configuration.md`: the
`[fetch]` and `[egress]` tables lose their "used from release 0.3.0" marks.
The integration guide: the strict-mode recipe and the egress section of
RFC 011.

## Alternatives considered

- **A readability crate.** Rejected: the available ports recurse, take no
  node bound and bring their own HTML stack; the heuristic above is forty
  lines and testable against the corpus.
- **A Markdown crate for rendering.** Rejected: the renderer is a flat walk
  of the bounded tree with a dozen rules; a crate adds surface for no gain.
- **Following cross-host redirects by default.** Rejected by D-34; the
  operator switch exists for closed deployments.
- **Treating a 5xx robots.txt as allow.** Rejected: RFC 9309 says
  unreachable is disallow; a one-hour cache keeps the cost bounded.
- **Returning image URLs in Markdown.** Rejected: they spend the budget on
  text the agent cannot use.

## Open questions

- Whether `text/markdown` should be eligible as text. Decided here: yes,
  treated as `text/plain` (no extraction), since it is already the output
  form; recorded for the tools page.
- The chrome class list and the 200-character candidate threshold are
  reviewable constants; the corpus decides their first values.
