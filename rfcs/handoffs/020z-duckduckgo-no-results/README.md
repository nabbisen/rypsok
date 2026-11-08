# Handoff 020z — DuckDuckGo: a page with no results is "no results", not a parse failure

**Created 2026-11-08.** A fix handoff of the audit (RFC 020, D3), wave 4.
Finding **NET-18**, confirmed by the second request of RFC 020, D-5 (020v,
correction C1, 2026-11-08). The owner decided to fix it before 0.9.0.

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md), RFC 015 (scraping drivers).
**Rules.** As 020w, including the per-change rule (`scrape_read` reads the
changed shape: run it). Evidence goes to `.git-exclude/evidence/020z/`. **No
request to DuckDuckGo**: the saved page of 020v is the evidence.
**Depends on.** 020v (`d94635b`). This commit becomes the release commit.

## The defect

- **The page.** DuckDuckGo's real no-results page (saved in 020v,
  `ddg-request-2-body.html`) marks itself with `span.no-results`, inside
  `div.no-results__container`. Its result list also holds a
  `div.result.result--no-result` with no link.
- **The driver.** It reads the marker as `div.no-results`, so the page is
  `Page::Drifted`, and a `parse_failure` counts toward LIM-B1. Three searches
  with no results take the engine offline for 60 s, and callers are told the
  engine failed. That is untrue output.
- **The synthetic fixture.** The fixture `no-results.html` was written by
  hand with a `div` and never matched a real page.

## Change scope

| # | Change | Test (fails before) |
|---|---|---|
| 1 | **The marker.** `SHAPE.no_results` becomes the class `no-results` on any element: `Selector { tag: None, class: Some("no-results"), attr: None }`. It matches the real `span` and the earlier `div` alike, and not `no-results__container` or `result--no-result`, because those are other class tokens. Confirm that the class match is by whole token | A unit test of the selector: `span.no-results` and `div.no-results` match; `div.no-results__container` and `div.result--no-result` do not |
| 2 | **The real page as the fixture.** Replace `fixtures/scrape/duckduckgo/no-results.html` with the page DuckDuckGo served, reduced to what the reader looks at and the structure around it. Keep the tags, classes and nesting of the result list, the `result--no-result` block and the marker. Remove scripts, styles, tracking parameters, any cookie-like value and anything unrelated. Say in the fixture's `README.md` row that it is the provider's page of 2026-11-08, reduced, and what was removed | The driver's no-results test reads the fixture as `Page::NoResults`; it fails before item 1 (`Drifted`). A pipeline test: a DuckDuckGo search on this page ends `Success` with no results and **no circuit effect** (three in a row leave the circuit closed) |
| 3 | **The record.** `docs/src/maintainers/engines/duckduckgo.md`, row "No results": "`span.no-results` (the class on any element), seen 2026-11-08 (RFC 020, D-5)". The record's other rows are unchanged | — |
| 4 | **Seeds.** Add the reduced page as a `scrape_read` seed (`seed-020z-no-results`), and run `scrape_read` for 120 s from the corpus | the run in the evidence |
| 5 | **Changelog** `[0.9.0]`, under Fixed: "A DuckDuckGo search with no results answers with no results, instead of counting as a parse failure that could open the engine's circuit (RFC 020, NET-18)" | — |

## Explicit non-change scope

- No other selector changes.
- No change to the circuit's limits.
- No request to the provider.

## Required evidence and review request

1. The gates.
2. Failing first.
3. Mutation of `engine/drivers/duckduckgo.rs` and `engine/scrape.rs`.
4. The 120 s fuzz run.
5. The review request at `.git-exclude/review-requests/020z-duckduckgo-no-results/README.md`.
