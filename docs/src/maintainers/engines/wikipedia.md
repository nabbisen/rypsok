# Wikipedia

Driver type `wikipedia`; keyless. Facts verified on 2026-09-29 from the
Wikimedia API and User-Agent policy pages; the query length was measured
on 2026-10-20 against the live API.

| Field | Value |
|---|---|
| Endpoint | `https://<lang>.wikipedia.org/w/api.php?action=query&list=search&srsearch=<q>&srlimit=<n>&srprop=snippet\|timestamp&format=json` |
| Permitted use | Public API; content CC BY-SA 4.0; attribution satisfied by the page URL; no white-labelling of the source |
| Identification | Descriptive User-Agent with contact information required; browser imitation is treated as malicious; requests must not be spread over several user agents. rypsok sends its standard User-Agent with the operator's contact |
| Rate limit | 200 requests per minute with a compliant User-Agent; configured at 100 per minute for headroom |
| Concurrency | At most 3; configured at 1 for the Action API |
| Storage | No restriction; `cacheable = true` |
| Attribution | Not mandatory beyond the URL; `attribution_required = false` |
| Languages | Every Wikipedia language edition: `languages = All`, mapped to the subdomain from the primary language subtag |
| Time range | Not supported: `time_ranges = None`; the engine is skipped when a period is asked for |
| Query length | `max_query_chars = 250`. The API refuses a query above 300 characters (`cirrussearch-query-too-long`) counted on its rewritten form: a single 300-character token is reported as 345, a 255-character token passes, a 299-character multi-word query passes. 250 keeps every shape of query under the limit |
| Categories | `general`, `science` by default |
| Date | The revision timestamp of the page, as a date |
| Result URL | `https://<lang>.wikipedia.org/wiki/<Title_with_underscores>`, percent-encoded |
| Errors | 429 → `rate_limited` with `Retry-After`; 403 → `blocked` (a bad User-Agent); other statuses and an `error` object → `request_failure`; a body that does not parse → `parse_failure`; a timeout → `timeout` |
| Response bound | 1 MiB after decompression |
| Live check | Query `Rust (programming language)` on `en`, asserting at least one result that parses; weekly |
| Notes | Avoid the `api.wikimedia.org` Core endpoints (deprecation announced from July 2026) |
