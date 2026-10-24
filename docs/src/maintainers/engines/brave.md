# Brave Search

Driver type `brave`; keyed by `X-Subscription-Token`, given through
`api_key_env` or `api_key_file`. Facts verified on 2026-09-29 from the
Brave Search API documentation and terms.

| Field | Value |
|---|---|
| Endpoint | `https://api.search.brave.com/res/v1/web/search` (`endpoint = "web"`, the default) and `/res/v1/news/search` (`endpoint = "news"`), with `q`, `count`, `search_lang`, `country`, `safesearch` and `freshness` |
| Permitted use | Paid plan with a monthly credit; results may be used for inference; must not be used to train, evaluate or otherwise improve AI models; must not be stored beyond transient storage; must not be redistributed |
| Identification | Must not obfuscate identity or circumvent rate limits; rypsok sends its standard User-Agent and the token on one origin only |
| Rate limit | Plan-dependent; configured at 60 per minute (one per second) by default to protect the credit |
| Concurrency | 2 |
| Storage | Forbidden: `cacheable = false` |
| Attribution | Optional ("POWERED BY BRAVE" when used): `attribution_required = false` |
| Languages | `search_lang` from the primary language subtag and `country` from the configured region: `languages = All` |
| Time range | `freshness` = `pd`, `pw`, `pm`, `py`: `time_ranges = All` |
| Query length | 600 characters and 75 words; both checked before the request |
| Safe search | `safesearch` = `off`, `moderate`, `strict` from the configuration |
| Categories | `general` (web), `news` (news endpoint) |
| Date | `page_age` when present, as a date |
| Errors | 429 → `rate_limited` with `Retry-After` or `X-RateLimit-Reset`; 401 and 403 → `auth_failure`; other statuses → `request_failure`; a body that does not parse → `parse_failure`; a timeout → `timeout` |
| Response bound | 1 MiB after decompression |
| Live check | Query `rust programming language` on the web endpoint, asserting at least one result that parses; weekly from `.github/workflows/scheduled.yml`, never on a pull request and never as a gate; skipped and reported when the repository secret `BRAVE_API_KEY` is absent; a failure means format drift (the provider changed its response or its terms), not an outage of rypsok |
| Notes | `count` is at most 20; `more_results_available` is ignored (no pagination, REQ-SEARCH-007) |
