# Changelog

All notable changes to rypsok are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). Release tags carry no `v` prefix.

Until 1.0.0 the external contract (tool names, parameters, response fields,
outcomes, error kinds) may change between minor versions; every such change
is announced here.

## [Unreleased]

## [0.3.0] — 2026-10-27 — Secure fetch (milestone M3)

### Added

- `web_fetch`: the main content of one public page as Markdown, bounded by
  `max_chars`, continuable by `start_char`; HTML, XHTML, plain text and
  Markdown; charset from the header, a `meta` declaration or detection;
  parsing bounded in nodes and depth with `extraction_partial` when a
  bound is hit; the error kinds `redirected`, `unsupported_content`,
  `too_large` and the fetch reasons of `denied` and `unavailable` (RFC 012).
- The agent-directed egress profile: a policy that refuses loopback,
  private, link-local, metadata and other internal destinations, judged
  on the URL before any request and on every address at connect time
  through the client's resolver; operator `allow` and `deny` rules;
  address literals in every notation; names that never name a public site
  (RFC 011).
- Strict fetch mode: only URLs `web_search` returned in this process, or
  the operator's allow list, can be fetched.
- robots.txt honored per RFC 9309 for the product token `rypsok` and `*`,
  cached per origin for a day (one hour for a failed read), bounded.
- Manual redirects: same-host redirects followed up to 5 hops, every
  target re-judged by the policy; a cross-host target returned as
  `redirected` unless the operator allows following.
- Concurrency limits for fetches: per process, per host (one at a time)
  and for parsing, which runs off the I/O path.
- An extraction corpus of 15 hand-written pages, resource tests for byte,
  node and depth bombs, and a memory measurement script.

### Changed

- A `<` followed by any letter starts a tag for the title and snippet
  cleaner, so that cleaning stays idempotent under NFC.

## [0.2.0] — 2026-10-24 — Health and circuit breaking (milestone M2)

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

[Unreleased]: https://github.com/nabbisen/rypsok/compare/0.3.0...HEAD
[0.3.0]: https://github.com/nabbisen/rypsok/releases/tag/0.3.0
[0.2.0]: https://github.com/nabbisen/rypsok/releases/tag/0.2.0
[0.1.0]: https://github.com/nabbisen/rypsok/releases/tag/0.1.0
