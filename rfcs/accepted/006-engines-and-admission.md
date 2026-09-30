# RFC 006 — Engine abstraction, registry, admission; Wikipedia and Brave

**Status.** Accepted (2026-09-30) — implementer may start after RFC 004.
**Tracks.** Milestone M1; requirements REQ-ENG-001 to -012, REQ-CMP-001
to -004, REQ-SEC-017, REQ-RES-002, REQ-RES-008, REQ-TEST-002, REQ-TEST-015,
REQ-TEST-016; external design §8 (engine profile), §11; threats THR-08,
THR-13, THR-15, THR-20.
**Touches.** `rypsok-core::engine`, `rypsok-core::egress` (engine profile),
`fixtures/wikipedia/`, `fixtures/brave/`.
**Handoff.** [`../handoffs/006-engines-and-admission/README.md`](../handoffs/006-engines-and-admission/README.md)

## Summary

One trait, one registry, one set of properties per engine, one HTTP client
per engine under the engine egress profile, and two drivers with their
admission records: Wikipedia (keyless) and Brave Search (keyed). Provider
obligations become data that the aggregator, the boundary and, later, the
cache obey.

## Design

### Abstraction

```rust
#[async_trait]
pub trait Driver: Send + Sync {
    fn kind(&self) -> &'static str;                       // "wikipedia", "brave"
    fn declared_keys(&self) -> &'static [&'static str];   // configuration keys the driver accepts
    fn properties(&self) -> &EngineProperties;
    async fn search(&self, req: &EngineRequest, cancel: CancellationToken) -> Result<EngineResponse, EngineError>;
}

pub struct EngineProperties {
    pub max_query_chars: u32,
    pub languages: LanguageSupport,        // All | Only(Vec<Language>) | None
    pub time_ranges: TimeRangeSupport,     // All | None
    pub rate_limit: RateLimit,             // requests per interval, from the admission record (LIM-C9)
    pub max_in_flight: u8,                 // LIM-C2 or lower (LIM-C2 is the ceiling)
    pub cacheable: bool,
    pub attribution_required: bool,
    pub user_agent: UserAgentFormat,       // Standard | Custom(&'static str)
}

pub struct EngineRequest { pub query: String, pub max_results: u8, pub language: Language, pub region: Region, pub safe_search: SafeSearch, pub time_range: Option<TimeRange> }
pub struct EngineResponse { pub results: Vec<RawResult> }   // title, url, snippet, date: raw strings
pub enum EngineErrorClass { Timeout, RateLimited, Blocked, ParseFailure, AuthFailure, RequestFailure }
pub struct EngineError { pub class: EngineErrorClass, pub retry_after: Option<Duration> }  // no provider text
```

`EngineError` carries no message. Provider text is logged at `debug` level
only, truncated, never at the default level (REQ-SEC-016, REQ-PRIV-004).

### Registry

Built once from the `[engines.*]` tables: identifier → `Engine { id,
driver, categories, preference, enabled, limiter, client, health }`. An
unknown `type` or an undeclared key fails validation (RFC 004). An engine
whose secret is missing is registered as `Disabled` with a log line
(REQ-OPS-003). Selection for a category returns enabled engines of that
category ordered by `preference`, then identifier (REQ-ENG-007).

### Per-engine limiter (REQ-CMP-002, REQ-ENG-010)

Each engine owns a `governor` token bucket at its `rate_limit` and a
semaphore of `max_in_flight`. The aggregator (RFC 007) acquires both before
a request; if neither can be acquired within the time left before the
per-engine timeout, the engine is skipped with `engine_skipped`, reason
`rate_limited`. `Retry-After` on a 429 sets the bucket empty until that
time (LIM-B4).

### HTTP client per engine (engine egress profile, REQ-SEC-017)

One `reqwest::Client` per engine, built by `egress::engine_client`:
`redirect(Policy::none())`, `no_proxy()` unless `[egress].proxy` is set,
`https_only(true)` unless the operator configured a plain-HTTP endpoint
explicitly, a default `User-Agent` of REQ-CMP-004 (or the driver's custom
format), `connect_timeout(LIM-S12)`, `timeout(LIM-S5)`, and the
credential header set on the client so that it can never be sent to another
origin (there are no redirects). A response body is read with a bound
declared by the driver (`max_response_bytes`, 1 MiB for both drivers).

### Admission records

Stored in `docs/src/maintainers/engines/<id>.md` (a new directory; each
record is one page of the maintainers' part) with the fields of
REQ-CMP-001. The records below are the content to write; facts were
verified on 2026-09-29 from the providers' pages.

**Wikipedia**

| Field | Value |
|---|---|
| Endpoint | `https://<lang>.wikipedia.org/w/api.php?action=query&list=search&srsearch=<q>&srlimit=<n>&format=json` |
| Permitted use | Public API; content CC BY-SA 4.0; attribution satisfied by the page URL; no white-labelling of the source |
| Identification | Descriptive User-Agent with contact information required; browser imitation "assumed malicious"; requests must not be spread over several user agents |
| Rate limit | 200 requests per minute with a compliant User-Agent; configured at 100 per minute for headroom |
| Concurrency | At most 3; configured at 1 for the Action API |
| Storage | No restriction |
| Attribution | Not mandatory beyond the URL; `attribution_required = false` |
| Languages | Every Wikipedia language edition: `languages = All`, mapped to the subdomain |
| Time range | Not supported: `time_ranges = None` |
| Query length | 300 characters, conservative; to be confirmed against the API's own limit in M1 |
| Categories | `general`, `science` |
| Date | Not returned by the search list |
| Notes | Avoid `api.wikimedia.org` Core endpoints (deprecation from July 2026) |

**Brave Search**

| Field | Value |
|---|---|
| Endpoint | `https://api.search.brave.com/res/v1/web/search` with header `X-Subscription-Token`; `/res/v1/news/search` for the `brave_news` engine |
| Permitted use | Paid plan with a monthly credit; results may be used for inference; must not be used to train, evaluate or "otherwise improve" AI models; must not be stored beyond transient storage; must not be redistributed |
| Identification | Must not obfuscate identity or circumvent rate limits |
| Rate limit | Plan-dependent (50 per second on the Search plan); configured at 1 per second by default to protect the credit |
| Concurrency | 2 |
| Storage | Forbidden: `cacheable = false` |
| Attribution | Optional ("POWERED BY BRAVE" format when used): `attribution_required = false` |
| Languages | `search_lang` and `country`: `languages = All` |
| Time range | `freshness` = `pd`, `pw`, `pm`, `py`: `time_ranges = All` |
| Query length | 600 characters and 75 words; `max_query_chars = 600`, and the driver also counts words |
| Categories | `general` (`brave`), `news` (`brave_news`) |
| Date | `page_age` when present |
| Notes | HTTP 429 carries `X-RateLimit-Reset`; treat as `Retry-After` |

The drivers implement the parsing of exactly these endpoints and classify
errors as: 429 → `RateLimited`; 401/403 → `AuthFailure` (Brave) or
`Blocked` (Wikipedia 403 for a bad User-Agent is classified `Blocked`);
5xx and network errors → `RequestFailure`; timeout → `Timeout`; any body
that does not parse → `ParseFailure`. A Brave query longer than 75 words
is refused by the driver before the request (`engine_skipped`,
`query_too_long`).

### Fixtures (REQ-TEST-002)

For each driver: `search-ok.json`, `search-empty.json`, `error-429`,
`error-auth` (Brave), `error-403.html` (Wikipedia), `error-malformed`.
Recorded by hand with a credential removed; the review checks that no key
or personal data is inside.

### Live checks (REQ-TEST-003)

A scheduled job runs one search per driver against the real endpoint with
a fixed query and asserts the parse succeeds. It needs `BRAVE_API_KEY` as
a repository secret; when absent, the Brave check is skipped and reported.

## Amendment to the baseline

External design §3.1 gains the `engine_skipped` reason `rate_limited`
(the engine's own limiter or in-flight cap could not be acquired before the
fan-out deadline). Applied with this RFC.

## Alternatives considered

- **DuckDuckGo HTML.** Rejected: no syndication rights, robots.txt disallows,
  bot challenge served to an honest client (startup review R-07).
- **One shared HTTP client.** Rejected: a credential header on a shared
  client can reach another origin; per-engine clients make the engine
  profile enforceable (R-01).
- **Scraping drivers now.** Deferred to DR-006 by the owner's decision.

## Open questions

- Wikipedia's exact query length limit (to be measured in M1 and written
  into the admission record).
- Whether `brave_news` should be a separate driver type or a mode of the
  Brave driver. Decided here: same driver, `endpoint = "web" | "news"` as
  a declared configuration key.
