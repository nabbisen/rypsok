# RFC 014 — Result cache

**Status.** Implemented (0.5.0); accepted 2026-10-27.
review decisions (`f869eaf`).
**Tracks.** Milestone M5; requirements REQ-CACHE-001 to REQ-CACHE-007,
REQ-CMP-007, REQ-TEST-010, REQ-PRIV-003, REQ-ENG-010 (cacheability),
REQ-OBS-001 (counters); external design §3.1 (notices), §12; limits
LIM-K1 to LIM-K4; threat THR-17; decision D-28.
**Touches.** `rypsok-core::cache` (new), `rypsok-core::search::pipeline`
(lookup and store), `rypsok-core::config` (`[cache]`),
`rypsok-core::model` (`CallerId` moves into the core; the `from_cache`
notice), `rypsok-core::telemetry` (counters), `rypsok::mcp` (the caller
passed to the service), the tools and configuration pages.
**Handoff.** [`../handoffs/014-result-cache/README.md`](../handoffs/014-result-cache/README.md)

## Summary

An in-memory cache of engine answers, off by default. The unit is one
engine's raw answer to one engine request for one caller: the admission
record governs storage per engine, so the cache stores per engine and
never stores what an engine's record forbids. Aggregation, the content
boundary and the notices run on every call, so a response built from
cached answers obeys the configuration in force. A hit is visible to the
caller as a `from_cache` notice naming the engine. The cache holds at most
LIM-K3 entries of at most LIM-K4 bytes for at most LIM-K2 seconds, in the
process's memory only, and a failure of the cache degrades nothing but
latency.

## Design

### Unit and key (REQ-CACHE-002, REQ-CACHE-003, REQ-CMP-007)

An entry is the `EngineResponse` (the raw results, before the content
boundary) that one engine returned for one `EngineRequest`. Its key is the
exact tuple

```text
(caller, engine id, query, language, region, safe_search, time_range, max_results)
```

where `query` is the validated query after the privacy check (the masked
text in mask mode). Keys are compared by equality, never by a hash alone:
a collision between two callers would be an isolation failure, however
improbable, so the design does not admit one. The memory cost of exact
keys is bounded by LIM-K4 with the entry.

The "engine set, configuration version and policy" of REQ-CACHE-003 are
carried by construction: the key names the engine, and the configuration
of a process is immutable for its lifetime (there is no reload). Should a
reload ever exist, clearing the cache is part of it; the invariant is
recorded in external design §16.

Only `web_search` has a cache. `web_fetch` cannot reach it: the cache type
is private to the search side and the fetch pipeline has no handle to it
(REQ-CACHE-002).

An engine whose properties say `cacheable = false` (Brave, by its record)
is neither looked up nor stored. The flag is the driver's, read from its
admission record, never from configuration (REQ-ENG-010).

### Caller isolation (REQ-CACHE-004)

`CallerId` moves from `rypsok::mcp::rate` to `rypsok_core::model::ids`
with the same interface (`local()`, `new`, `as_str`); the binary re-exports
it. `Rypsok::search` keeps its signature and means the caller `local`;
`Rypsok::search_as(&self, caller: &CallerId, req, cancel)` is the general
form, and the adapter calls it with the caller the HTTP gate attached (or
`local` over stdio). Two callers never share an entry; over stdio the one
caller is the process (REQ-CACHE-004).

### Place in the pipeline

```text
select ─▶ circuit admission ─▶ [cache lookup] ─▶ outbound permit ─▶ engine permit ─▶ driver ─▶ [cache store]
```

In the fan-out task of each queried engine:

| Situation | Lookup | Store |
|---|---|---|
| Admitted `Allowed`, `cacheable = true` | yes | yes, on `Answered` |
| Admitted `Probe` (RFC 010) | no: the probe exists to reach the network | yes, on `Answered` |
| Skipped by cooldown, limiter or properties | no: the engine is reported skipped, and it is | no |
| `cacheable = false` | no | no |
| Privacy verdict `Warned` (RFC 009) | no | no: a query that looks like it carries a credential is never retained |
| Cache disabled | no | no |

A hit ends the task as `Ended::Cached(response)`: no outbound or engine
permit is taken, the health table is not informed, no latency is recorded,
and the log event `engine_answered` carries `cached = true`. The outcome
decision counts a cached engine as answered. A cancelled call stores
nothing (its tasks are aborted before `Answered`).

Serving from the cache during a cooldown was considered and rejected: the
notices and `engine_status` would say skipped while results arrived.
Honest reporting wins over a few more results.

### The store (REQ-CACHE-005, REQ-CACHE-006)

`cache::ResultCache` in `rypsok-core::cache`: a map from key to entry with
an expiry instant and a last-use order, behind a `std::sync::Mutex` that
is never held across an `await` (the workspace denies it). Every
operation is O(log n) or better; no new dependency unless the package
justifies one. Time comes from the tokio clock, as in the pipeline, so
tests pause it.

- **Expiry.** An entry older than LIM-K2 is never returned; expired
  entries are removed on lookup and swept before eviction.
- **Bound.** At most LIM-K3 entries; when full, expired entries go first,
  then the least recently used.
- **Entry size.** An answer whose raw bytes (titles, snippets, URLs)
  exceed LIM-K4 is not stored; the memory of the cache is therefore at
  most LIM-K3 × LIM-K4 (32 MiB at the defaults).
- **Failure.** A poisoned lock, or any other internal failure, disables
  the cache for the rest of the process with one `warn` event
  `cache_disabled`; every later lookup is a miss and every store a no-op.
  A lookup or store never returns an error and never panics
  (REQ-CACHE-006).

### Visibility (external design §3.1)

A response that used a cached answer carries one notice per such engine:

```json
{ "code": "from_cache", "engine": "wikipedia" }
```

The tools page says that a result marked `from_cache` may be as old as
`[cache] ttl_s`. Nothing else in the response differs from a live call.

### Configuration (REQ-CACHE-001, REQ-CACHE-007)

```toml
[cache]
enabled = false          # LIM-K1
ttl_s = 300              # LIM-K2, 10–3600
max_entries = 1000       # LIM-K3, 1–100000
max_entry_bytes = 32768  # LIM-K4, 1024–1048576
```

Validation (RFC 004): ranges as above. `enabled = true` with no configured
engine whose record allows storage is not an error but a startup `warn`
`cache_without_storable_engines`, so a cache that can never hit is named.

The configuration page states, next to `enabled`, that a cache keyed by
query retains the queries of the last `ttl_s` seconds in memory
(REQ-CACHE-007). Nothing is written to disk; a persistent cache is DR-003
and needs its privacy RFC.

### Observability (REQ-OBS-001)

The counters gain `cache` with the outcomes `hit`, `miss`, `stored`,
`too_large` (not stored by LIM-K4) and `bypassed` (the privacy verdict);
they are logged with the other counters. `engine_status` is unchanged:
the tool's contract stays as RFC 010 defined it, and the cache is an
operator concern read from the log.

### Tests (REQ-TEST-010)

With a mock engine behind `wiremock` and the paused clock:

- off by default: the same query reaches the mock twice;
- `web_fetch` of the same URL twice reaches the mock twice with the cache
  on;
- a driver with `cacheable = false` is requested twice and never stored; a
  cacheable one is stored and the second call carries `from_cache` with
  its identifier;
- two callers with the same query: the second caller misses, the first
  hits;
- a hit before `ttl_s`, a miss after; LIM-K3 + 1 distinct queries evict the
  least recently used, not the most recently used;
- a poisoned lock (a thread panics while holding it) leaves the search
  working through the network and emits `cache_disabled` once;
- a query that the privacy check warned about is never stored; a masked
  query is stored under its masked text;
- a probe bypasses the lookup; a cancelled call stores nothing;
- an answer above LIM-K4 is not stored.

## Amendment to the baseline

Applied with this RFC:

- Limits: LIM-K3 gains the range 1–100000; the row LIM-K4 (size of one
  entry, 32768 bytes, 1024–1048576) is added.
- External design §3.1: the notice code `from_cache` (with `engine`).
- External design §12 restated: entries per engine and per caller, the
  table above; the invariant that a configuration reload clears the
  cache (§16).
- Configuration schema (RFC 004): the `[cache]` table.
- `CallerId` is a core type (external design §5).

## Alternatives considered

- **One entry per response.** Rejected: one non-cacheable engine in the
  set would forbid the whole entry, and a cached response would carry the
  notices and boundary of the moment it was stored rather than of the
  call.
- **Hashed keys.** Rejected: a collision across callers breaks isolation
  by design, and the saving is small once LIM-K4 bounds the entry.
- **Serving stale entries while an engine is in cooldown.** Rejected, as
  above: the notices would lie.
- **Cache statistics in `engine_status`.** Rejected: the tool's contract
  is stable since 0.2.0 and the counters serve the operator.
- **A persistent or semantic cache.** DR-003, DR-004: only with the
  privacy RFC of REQ-CACHE-007.

## Open questions

- Whether the key should include `max_results`. Decided here: yes. An
  engine asked for fewer results returns a different answer, and
  LIM-S3 makes the value a constant per process in practice.

## Amendment to RFC 014 (2026-10-27, M5 review)

Decided by the architect under the owner's two principles after the
review of the implementation and release 0.5.0.

- **A sixth counter outcome, `disabled`.** A store that fails because the
  cache disabled itself is counted as `disabled`, beside `hit`, `miss`,
  `stored`, `too_large` and `bypassed`. It names the one failure mode
  the cache has; folding it into another outcome would hide it.
- **A probe stores.** The design's table said so; recorded here because
  a reviewer asked: a probe's answer is a fresh network answer and is
  stored, while a probe never reads the cache.
- **Expiry before recency.** With one lifetime for every entry the
  expired entry is also the least recently used; the implementation
  keeps the order the design states, at no cost.
