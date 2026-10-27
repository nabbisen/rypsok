# DuckDuckGo (HTML)

Driver type `duckduckgo`; keyless; **a scraping driver** (RFC 015): it
reads the human-facing endpoint, runs only with `[scraping] enabled =
true`, and is never in the default set. Facts verified on 2026-10-27 by
the architect from the live endpoint, its robots.txt, the terms and the
Acceptable Use Policy; what could not be verified is marked.

| Field | Value |
|---|---|
| Endpoint | `https://html.duckduckgo.com/html/?q=<q>&kl=<region-language>&kp=<safe>&df=<period>` (GET) |
| Permitted use | The terms (updated 2025-07-01) say nothing about automated access and point to the Acceptable Use Policy, which forbids access "without authorization (that is, any form of hacking)" and says nothing about bots or scraping. The endpoint is meant for people; using it from a program is the operator's decision under `[scraping]` |
| Identification | The standard User-Agent with the operator's contact and `Accept: text/html`; no cookie, no `Accept-Language`, no `Sec-Fetch-*`, no browser imitation (REQ-ENG-014) |
| Observed at admission | On 2026-10-27 a request through a generic fetching tool was answered with the challenge page ("Unfortunately, bots use DuckDuckGo too."); the live check from a workstation with the standard User-Agent and the query below was answered with results that parsed. The provider challenges some agents or addresses; a `blocked` answer is its decision and the live check reports the state weekly |
| Rate limit | Unknown; configured at 10 requests per minute |
| Concurrency | 1 |
| Storage | Unknown: `cacheable = false` |
| Attribution | Not required: `attribution_required = false` |
| Languages | `languages = All`; the region-language code `kl` is `<region lower>-<primary language>`, `us-en` for region `US` and language `en` |
| Time range | `df` = `d`, `w`, `m`, `y`: `time_ranges = All` |
| Safe search | `kp` = `1` strict, `-1` moderate, `-2` off |
| Query length | `max_query_chars = 500`; not measured against the live endpoint |
| Categories | `general` |
| Result | `div.result`; the target in `a.result__a` `href`, a redirect wrapper `//duckduckgo.com/l/?uddg=<target>&rut=…` unwrapped to `uddg` or an absolute `http(s)` link; the title is the text of `a.result__a`; the snippet the text of `a.result__snippet`. Verified by the live check on 2026-10-27 (results parsed and unwrapped) and pinned by the fixtures |
| No results | `div.no-results`. Not verified at admission |
| Challenge | The sentence "bots use DuckDuckGo too", an element with class `anomaly-modal__title`, or a form with `action="/html/anomaly"`. The sentence and the `/anomaly/` path were observed on 2026-10-27 |
| robots.txt | `html.duckduckgo.com/robots.txt` on 2026-10-27: `User-agent: *`, `Allow: /`, a sitemap. Recorded as a fact; not consulted at run time (RFC 015) |
| Errors | A challenge page or 403 → `blocked`; 429 and 503 → `rate_limited` with `Retry-After`; a redirect (never followed) and other statuses → `request_failure`; a page in another shape or above the bounds → `parse_failure`; a timeout → `timeout` |
| Response bound | 256 KiB; parsing under LIM-F10 nodes on the shared parsing permits (LIM-C5) |
| Live check | Query `Rust programming language`, asserting at least one result; weekly from `.github/workflows/scheduled.yml`, never a gate. A `blocked` answer is the provider's refusal of the honest agent and is reported, never worked around |
| Fixtures | `crates/rypsok-core/fixtures/scrape/duckduckgo/` (hand-written) |
