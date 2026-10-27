# RFC 015 — Scraping drivers as an opt-in engine type

**Status.** Accepted (2026-10-27) — implementer may start after RFC 014
PR 2 (the parsing module is shared).
**Tracks.** Milestone M5; deferred requirement DR-006; requirements
REQ-ENG-008 (official interfaces first), REQ-ENG-009 (anti-bot responses),
REQ-ENG-013 to REQ-ENG-015 (new, below), REQ-CMP-001, REQ-CMP-002,
REQ-CMP-004, REQ-RES-002, REQ-TEST-003, REQ-TEST-018 (new), REQ-MAIN-004;
external design §11; limit LIM-G1 (new); threats THR-01, THR-20 (new);
decisions D-06, X-08.
**Touches.** `rypsok-core::engine` (the `scraping` property, the
`[scraping]` switch at validation), `rypsok-core::html` (new: the bounded
DOM and charset code moved out of `fetch`), `rypsok-core::engine::scrape`
(new: result extraction from a page), the first driver `duckduckgo` with
its admission record, the scheduled live check, the configuration and
engines pages.
**Handoff.** [`../handoffs/015-scraping-drivers/README.md`](../handoffs/015-scraping-drivers/README.md)

## Summary

A scraping driver asks a provider's human-facing page and extracts results
from its HTML. It is the route to general web search without an account
or a key, which the owner's local-first consumer wants (X-08), and it is
also use outside what most providers permit. rypsok therefore ships such
drivers under one explicit switch, off by default, that the operator turns
on at the operator's own risk; it never pretends to be a browser; it bounds
what it reads and parses exactly as `web_fetch` does; and it reports a
block as a block. The first driver is DuckDuckGo's HTML endpoint. Nothing
changes for the official-interface engines or for the default set.

## Design

### The engine type (REQ-ENG-013)

`EngineProperties` gains `scraping: bool`. A driver with `scraping = true`:

- is refused at configuration validation unless `[scraping] enabled =
  true`, with the message
  `engines.<id>: type "<type>" scrapes a human-facing interface; it runs only with [scraping] enabled = true, at the operator's risk under the provider's terms`.
  Refused, not silently disabled: an operator who wrote the table must
  decide, and `rypsok check` says what the decision is;
- is never part of the zero-configuration default set (REQ-OPS-003);
- is listed on the configuration page in its own table with the sentence
  that the provider's terms are the operator's responsibility and that
  the endpoint's robots.txt is not consulted;
- logs `warn` `scraping_enabled` with the engine identifiers at startup
  when the switch is on.

```toml
[scraping]
enabled = false      # LIM-G1

[engines.ddg]
type = "duckduckgo"
categories = ["general"]
```

### Honesty (REQ-ENG-014, REQ-CMP-004)

A scraping driver sends the standard User-Agent of REQ-CMP-004 and no
header meant to look like a browser (no `Sec-Fetch-*`, no browser
`Accept-Language` imitation, no cookies). When the provider answers with a
challenge, a CAPTCHA, a 403, 429 or 503, the driver classifies it
`blocked` or `rate_limited` (REQ-ENG-009, REQ-RES-002), the circuit of RFC
010 opens as for any engine, and the result is honest: the provider does
not want rypsok, and rypsok says so. A driver that only works by
pretending is not admitted.

### Bounds (REQ-ENG-015)

The same bounds as the fetch pipeline, with the same code:

- the response is streamed to at most the driver's `max_response_bytes`
  (256 KiB for the first driver);
- the page is parsed with the bounded DOM of RFC 012 (LIM-F10 nodes,
  LIM-F11 depth) on the blocking pool under LIM-C5, after charset
  decoding; `fetch::dom` and `fetch::charset` move to `rypsok_core::html`
  so that `engine` does not depend on `fetch`;
- the driver follows no redirect off the provider's host and at most
  LIM-F6 redirects;
- rate and concurrency come from the record (LIM-C9, at or below LIM-C2),
  conservative by default: 10 requests per minute, 1 in flight.

### Extraction (`engine::scrape`)

A driver declares a **result shape**: which element is a result container,
and within it which elements carry the link, the title and the snippet.
The shape is a small set of selectors over tag name, class token and
attribute equality, matched by a matcher of a few dozen lines in
`engine::scrape`; no CSS engine is added. The page is walked iteratively
(no recursion, as RFC 012 requires). Raw titles and snippets go to the
pipeline as every driver's do (REQ-ENG-012): the content boundary applies,
and THR-01 is handled where it is handled for every engine.

Three page states are told apart, each with a marker the record names:

| Page | Driver result |
|---|---|
| Results present | `EngineResponse` with the results in page order; the provider's redirect wrapper is unwrapped (REQ-ENG-001) |
| The provider's own "no results" marker | an empty `EngineResponse` |
| Neither results nor the marker | `parse_failure`: the format drifted |
| A challenge, a CAPTCHA, 403, 429, 503 | `blocked` or `rate_limited` |

`parse_failure` counts toward the circuit, and the scheduled live check
(REQ-TEST-003) sends one fixed query to the real endpoint and asserts at
least one result, so drift is seen within a day, never by a caller alone.

### The first driver: `duckduckgo`

The HTML endpoint of DuckDuckGo (`html.duckduckgo.com`), category
`general`. Its admission record (`docs/src/maintainers/engines/duckduckgo.md`)
is written by the implementer against the live site at admission time and
carries what the RFC does not restate: the request parameters for the
query, region and language, the time range and safe search; the markers
of the three page states; the storage rule (`cacheable`, decided by the
provider's terms as read at admission, `false` when in doubt); the
robots.txt status of the endpoint, recorded as a fact; the rate the
record allows. The RFC fixes only what the type requires: `scraping =
true`, the standard User-Agent, `attribution_required = false`,
`max_response_bytes = 262144`, 10 requests per minute, 1 in flight.

Further scraping drivers follow REQ-MAIN-004: a driver, its record and its
configuration entry, no RFC. The operator's own upstream (a self-hosted
SearXNG with its JSON interface) is not scraping and is not this RFC; it
is the natural next engine and gets a short RFC of its own.

### Tests (REQ-TEST-018)

Fixtures under `fixtures/scrape/duckduckgo/`, hand-written in the page's
shape and carrying no third-party content: a results page, a no-results
page, a challenge page, a drifted page, and a page above the node bound.
Tests: the shape yields the expected results in order with wrappers
unwrapped; the no-results marker yields empty; the drifted page is
`parse_failure`; the challenge page and the three statuses are `blocked`
or `rate_limited`; the bound ends parsing with the results found so far
or `parse_failure`, never a panic; the table is refused without the
switch and accepted with it, with the exact message; the default set
never contains a scraping engine; the User-Agent sent to the mock is the
standard one and no browser header is present; a redirect off the host is
not followed.

## Amendment to the baseline

Applied with this RFC:

- Requirements §7 gains REQ-ENG-013 (opt-in switch), REQ-ENG-014 (no
  browser imitation; blocks reported), REQ-ENG-015 (bounded scraping);
  §21 gains REQ-TEST-018 (scraping driver tests). All M5 · T.
- Limits: the section "Scraping (M5)" with LIM-G1 (`off`).
- Threat register: THR-20, the operator's exposure under a scraped
  provider's terms, controlled by REQ-ENG-013 and REQ-ENG-014.
- External design §11: the row for scraping drivers.
- Glossary: "Scraping driver".
- Configuration schema (RFC 004): the `[scraping]` table.

## Alternatives considered

- **A browser-like User-Agent, as SearXNG uses.** Rejected: REQ-CMP-004
  forbids imitation, and a project that identifies itself everywhere else
  cannot lie in one place. A provider that blocks the honest agent is a
  provider rypsok cannot use; the record says so.
- **Per-engine acknowledgement instead of one switch.** Rejected: one
  switch with one sentence is harder to misread than a key per engine,
  and the decision is one decision.
- **Consulting robots.txt for the search endpoint.** Rejected as a
  mechanism: it would always disallow and the switch would mean nothing.
  The fact is recorded in the admission record instead, where the
  operator reads it.
- **A headless browser.** Rejected: outside every bound this project
  keeps, and imitation by construction.
- **Caching scraped results by default to spare the provider.** Rejected
  as a rule: the record decides `cacheable` per provider, like every
  other engine.

## Open questions

- Which driver is second. Decided here: none in M5; the operator's
  upstream (SearXNG JSON) is the next engine RFC, and further scraping
  drivers come through REQ-MAIN-004 when someone needs them.

## Amendment to RFC 015 (2026-10-27, M5 review)

Decided by the architect under the owner's two principles after the
review of the implementation and release 0.5.0.

- **One parsing bound for the process.** The parsing permits (LIM-C5)
  are owned by the `Registry` and shared with the `FetchService` by
  `Rypsok::from_parts`, so the scraping drivers and `web_fetch` parse
  under one bound. A host that assembles its own `Registry` and
  `FetchService` shares them with `FetchService::with_parsing`; the
  integration guide's embedding section says so.
- **No redirect is followed**, on or off the provider's host. The
  provider answers its results page directly; a branch for an on-host
  redirect would serve no observed case. The design's "no redirect off
  the host" is tightened to "no redirect".
- **503 is `rate_limited`**, with `Retry-After` when present; 403 and a
  challenge page are `blocked`.
- **The header test is a whitelist.** The test asserts that every header
  the driver sends is one of `host`, `user-agent`, `accept` and
  `accept-encoding`; a blacklist of browser headers could be outrun by a
  new one.
