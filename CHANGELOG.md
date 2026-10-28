# Changelog

All notable changes to rypsok are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). Release tags carry no `v` prefix.

Until release 1.0.0 the external contract (tool names, parameters,
response fields, outcomes, error kinds) may change between minor
versions; every such change is announced here. From 1.0.0 the contract is
stable as the [stability page](docs/src/reference/stability.md) defines
it: what it lists as frozen changes only with a major version, additions
arrive in minor versions, and every change of a default value is
announced here with both values.

## [Unreleased] — Corrections from the first independent review (0.7.1)

RFC 019: the fixes of handoffs 019a to 019c are listed here as they land.

### Changed

- RFCs 001 to 018 moved to `rfcs/done/` with the release each shipped in,
  as RFC 000 prescribes; no design changed.

## [0.7.0] — 2026-10-28 — Security (milestone M7)

### Added

- The threat model page and the register revised against the delivered
  system: THR-26 (slow or stream-flooding HTTP clients), THR-22 extended
  to CI actions, the scraping threat renumbered THR-25, two residual
  risks recorded, the review log caught up (RFC 018).
- `[service.http] request_timeout_s` (LIM-H5) and `max_streams` (LIM-H4):
  a request still running after the bound is answered 408; one HTTP/2
  connection opens at most `max_streams` streams.
- `fuzz/`: eight fuzz targets on every parser that reads external bytes,
  run by the scheduled workflow, report only.
- Every CI action pinned to a commit hash, with a gate test.
- Hardened systemd and container profiles on the deployment page.

### Changed

- Invariant I5 states the one exception it always had: a driver may log
  at most 200 characters of a provider's error body, at `debug` only.

## [0.6.0] — 2026-10-27 — Users (milestone M6)

### Added

- The book's part for new users: five tutorials (a first search by hand
  and with a runtime, a keyed engine, reading a page in parts, network
  mode behind a proxy, the scraping switch), "Reading a response" for
  the author of a host's prompt, and "Troubleshooting" (RFC 017).
- `examples/`: configurations, host entries and real call answers that
  the tutorials include and CI checks; the first tutorial runs end to
  end in CI.
- A workflow that builds the book on every push and publishes it to
  GitHub Pages once Pages is enabled.

- The stability page: what is frozen from 1.0.0, what may be added in a
  minor version, and what may change with notice (defaults, logs, the
  output of `check`) (RFC 016).
- A CI job running `cargo semver-checks` on both crates from the first
  commit after 1.0.0.
- `rypsok-core`: constructors for the driver interface
  (`EngineProperties::new` and `with_*`, `RawResult::new`,
  `EngineRequest::new`, `EngineResponse::new`, `DriverContext::new`),
  `FetchRequest::new`, `ConfigError::new`.

### Changed

- `rypsok-core`: the requests, responses, driver types, configuration
  tables and the growable vocabularies (`NoticeCode`, `Reason`,
  `DetailReason`, `SecretKind`, `Category`, `EngineErrorClass`,
  `LanguageSupport`, `TimeRangeSupport`, `UserAgentFormat`) are
  `#[non_exhaustive]`: build them through their constructors and match
  them with a wildcard arm. The frozen vocabularies (`Outcome`,
  `ErrorKind`, `CallError`, `EngineState`, `TimeRange`) stay exhaustive.
- A `max_results` outside 1 to 255 is refused with the operator's
  ceiling in the message, as a value within that range already was.
- `serve --help` no longer mentions a milestone.

## [0.5.0] — 2026-10-27 — Optimization (milestone M5)

### Added

- `[cache]`: an in-memory result cache, off by default; one entry per
  engine answer per caller, only for engines whose admission record
  allows storage; a hit is reported by the notice `from_cache`; bounded
  by `ttl_s`, `max_entries` and `max_entry_bytes` (LIM-K1 to LIM-K4,
  RFC 014). `web_fetch` is never cached.
- `CallerId` is a core type; `Rypsok::search_as` searches on behalf of a
  caller (the caller of the HTTP gate, or `local`).
- Scraping drivers as an opt-in engine type behind `[scraping] enabled`
  (off by default; a scraping table is refused without it; never in the
  default set); the standard User-Agent and no browser imitation; a
  challenge is `blocked`; bounded like `web_fetch` and parsed under the
  shared parsing permits (LIM-G1, RFC 015). First driver: `duckduckgo`
  (the HTML endpoint), with its admission record.
- `rypsok_core::html`: the bounded DOM and charset decoding, shared by
  the fetch pipeline and the scraping drivers (moved out of `fetch`).

### Changed

- `legacy_revisions = false` now refuses the handshake-based revisions:
  `initialize` gets the JSON-RPC error `-32022` and `server/discover`
  lists the 2026-07-28 revision alone (RFC 013 amendment).
- A bind on an unspecified address (`0.0.0.0`, `::`) requires
  `allowed_hosts`; the bind host is no longer admitted as a `Host`.
- HTTP/2 connections get a keep-alive ping after `idle_timeout_s` and are
  closed when it goes unanswered (LIM-H3 on both HTTP versions).

### Documentation

- The headers a host must send over HTTP (`MCP-Protocol-Version`,
  `Mcp-Method`, `Mcp-Name`) with a complete example.

## [0.4.0] — 2026-10-27 — Network transport (milestone M4)

### Added

- `serve --transport http`: MCP over Streamable HTTP on `/mcp`, loopback
  by default; the 2026-07-28 revision served statelessly, the
  handshake-based revisions 2024-11-05 to 2025-11-25 with sessions in
  memory until `legacy_revisions = false` (RFC 013).
- `Host` and `Origin` validation; a bind off loopback requires callers.
- Static bearer tokens per caller, read from the environment or a file,
  compared in constant time; rate limits per caller.
- Bounds on connections, request bodies and idle time (LIM-H1 to LIM-H3).
- A `Dockerfile` (static binary, distroless, non-root) and the deployment
  page.

### Supported protocol revisions

2026-07-28 (current, stateless) and the handshake-based revisions the SDK
serves: 2025-11-25, 2025-06-18, 2025-03-26, 2024-11-05.

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

[Unreleased]: https://github.com/nabbisen/rypsok/compare/0.7.0...HEAD
[0.7.0]: https://github.com/nabbisen/rypsok/releases/tag/0.7.0
[0.6.0]: https://github.com/nabbisen/rypsok/releases/tag/0.6.0
[0.5.0]: https://github.com/nabbisen/rypsok/releases/tag/0.5.0
[0.4.0]: https://github.com/nabbisen/rypsok/releases/tag/0.4.0
[0.3.0]: https://github.com/nabbisen/rypsok/releases/tag/0.3.0
[0.2.0]: https://github.com/nabbisen/rypsok/releases/tag/0.2.0
[0.1.0]: https://github.com/nabbisen/rypsok/releases/tag/0.1.0
