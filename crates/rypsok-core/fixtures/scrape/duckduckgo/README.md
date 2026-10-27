# Scraping fixtures: DuckDuckGo HTML

Hand-written pages in the shape of `html.duckduckgo.com/html/` for
REQ-TEST-018 (RFC 015). No copied page, no third-party text; the markers
are those of the admission record `docs/src/maintainers/engines/duckduckgo.md`.

| Fixture | Exercises |
|---|---|
| `results.html` | Four `div.result` entries: a wrapped link (`//duckduckgo.com/l/?uddg=…`), an absolute link, a `javascript:` link (dropped), a wrapped link without a snippet; the page chrome around them |
| `no-results.html` | The provider's `div.no-results` marker and nothing else |
| `challenge.html` | The anti-bot page: its sentence, the `anomaly-modal__title` element and the `/html/anomaly` form |
| `drifted.html` | A page with results in another shape: neither `div.result` nor the no-results marker |
