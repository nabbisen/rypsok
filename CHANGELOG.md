# Changelog

All notable changes to rypsok are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). Release tags carry no `v` prefix.

Until 1.0.0 the external contract (tool names, parameters, response fields,
outcomes, error kinds) may change between minor versions; every such change
is announced here.

## [Unreleased] — Health and circuit breaking (milestone M2, 0.2.0)

### Added

- A circuit per engine, driven by real requests and the clock: `open` after
  3 consecutive failures of class `timeout`, `rate_limited`, `blocked` or
  `parse_failure`, cooldown 60 s doubling up to 900 s, `Retry-After`
  honored within that cap; one real request probes the engine after the
  cooldown; `auth_failure` and `request_failure` never open it (RFC 010).
- `engine_status` reports the circuit state and the median latency of the
  last 20 completed requests per engine; state lives in memory and is
  reset when the process restarts.
- Circuit transitions are logged as `circuit_opened`, `circuit_half_open`
  and `circuit_closed` events with the engine, the cooldown and the
  failure count.

### Changed

- A cancelled probe, or a probe that ends with an uncounted failure,
  reopens the circuit with the same cooldown instead of counting as a
  verdict.

## [0.1.0] — 2026-10-20 — Search kernel (milestone M1)

### Added

- `web_search` over MCP stdio (revision 2026-07-28 with `server/discover`,
  and the handshake-based revisions 2024-11-05 to 2025-11-25): one
  structured response with a declared output schema, the same JSON as text,
  outcomes `ok`, `partial`, `empty`, and tool execution errors of kinds
  `invalid_request`, `denied`, `unavailable` (RFC 003).
- `engine_status` reporting `disabled`, `closed`, `open`, `half_open` per
  configured engine; the operator can remove the tool (RFC 003).
- Engines: Wikipedia (keyless) and Brave Search (keyed; web and news
  endpoints), each with an admission record, fixtures and a scheduled live
  check (RFC 006). A `base_url` key lets the Wikipedia driver query a
  private mirror.
- Concurrent aggregation under one deadline with per-engine timeouts, early
  completion, cancellation through `notifications/cancelled`, panic
  isolation, and no retries (RFC 007).
- URL normalization with tracking parameters removed, deduplication by
  comparison key, reciprocal rank fusion in integer arithmetic (RFC 008).
- The content boundary: markup and entities, NFC, removal of invisible and
  control characters, bounded titles and snippets, notices for every change,
  instruction-like content noticed or dropped (RFC 005).
- The secret check on queries with modes `refuse` (default), `mask`, `warn`,
  `off`; structured JSON logs on standard error that never carry a query,
  a URL path or content; in-process counters (RFC 009).
- Configuration in one TOML file with every default from the limits table,
  validation that names the key, secrets by environment variable or file,
  `rypsok check` (RFC 004).
- Per-tool invocation rate limits; per-engine rate and in-flight limits
  honoring `Retry-After`.
- Workspace, CI gates, requirement coverage gate, limits consistency gate
  (RFC 001, RFC 002).
- Log events of dependencies are capped at `warn` whatever the configured
  level, so that no library can log a query or a URL at `debug`.

### Limits (provisional, to be confirmed by measurement)

Search deadline 2500 ms, engine timeout 2000 ms, reserve 200 ms, up to 8
engines per request, 10 results per engine, snippets 300 and titles 150
characters, query 400 characters, 60 `web_search` calls per minute.

[Unreleased]: https://github.com/nabbisen/rypsok/compare/0.1.0...HEAD
[0.1.0]: https://github.com/nabbisen/rypsok/releases/tag/0.1.0
