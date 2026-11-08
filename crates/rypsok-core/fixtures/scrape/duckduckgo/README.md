# Scraping fixtures: DuckDuckGo HTML

Pages in the shape of `html.duckduckgo.com/html/` for REQ-TEST-018 (RFC 015).
All are hand-written, no third-party text, except `no-results.html`, which is
the provider's page (see its row); the markers are those of the admission
record `docs/src/maintainers/engines/duckduckgo.md`.

| Fixture | Exercises |
|---|---|
| `results.html` | Four `div.result` entries: a wrapped link (`//duckduckgo.com/l/?uddg=…`), an absolute link, a `javascript:` link (dropped), a wrapped link without a snippet; the page chrome around them |
| `no-results.html` | The page DuckDuckGo served on 2026-11-08 for a search with no results (RFC 020, D-5, NET-18), reduced: the tags, classes and nesting of the header form, the result list with its `div.result.result--no-result` block, the `span.no-results` marker and the feedback link are kept. Removed: the scripts and style links, the icon and favicon links, the lists of regions and times, the hidden state field, the tracking pixel and the comments. The query in it is a made-up string, `"zzqxvqplm8f3k2j9d7s" -rust`; there is no cookie or other value of a session |
| `challenge.html` | The anti-bot page: its sentence, the `anomaly-modal__title` element and the `/html/anomaly` form |
| `drifted.html` | A page with results in another shape: neither `div.result` nor the no-results marker |
