# Handoff 020r — A request identifier and the caller in every line of a call

**Created 2026-11-07.** A fix handoff of the audit (RFC 020, D3), wave 3.
Finding TST-2 (Medium): REQ-OBS-005 promises structured lines "with a request
identifier, engine, host name, timing, counts, outcome and policy decision";
RFC 009 lists `request_id`, RFC 013 the `caller` in the `call` event; no line
carries either, and `telemetry::request_id()` is used by no code. Also RFC 009
"one `call` event per call": `engine_status` logs none (TST-15).

**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md), RFC 009, RFC 013.
**Rules.** As 020q. Evidence to `.git-exclude/evidence/020r/`.
**Depends on.** Nothing.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | Each tool call gets a request id from `telemetry::request_id()` (the existing hex format of the RFC 009 amendment) at the core's entry (`Rypsok::search`, `fetch`, `status`), carried by a `tracing` span around the call so that **every** event of the call (engine events, refusals, `fetched`, `call`) has `request_id` as a field without each macro naming it. The id is **not** returned to the caller and not part of any response (no contract change) | Capture the log of one search with two engines and one fetch: every event between the call's start and its `call` event has the same `request_id`; two calls have different ids; concurrent calls do not mix ids (two calls interleaved by a mock delay) | `reference/configuration.md` (logging), `troubleshooting.md` ("follow one call in the log") |
| 2 | The `call` event carries `caller` (the caller id over HTTP with callers; `local` otherwise), as RFC 013 says; the id is the operator's label, never a token | Over HTTP with two callers: the `call` events carry their ids; over stdio: `local` | RFC 013 is right already; nothing |
| 3 | `engine_status` logs one `call` event like the other tools (outcome, latency, request id, caller) | The event exists | — |
| 4 | The SDK's own events (rmcp, hyper) are outside the span; say which in the review request and do not try to reach them | — | — |
| 5 | Changelog: Added (the two fields; `engine_status` call event) | — | `CHANGELOG.md` |

## Explicit non-change scope

No response field; no change of the id's format; no log level change; no new
dependency (the `tracing` span machinery is already in the tree).

## Required evidence and review request

The gates; failing-first output; a sample of the captured log lines; the
review request at `.git-exclude/review-requests/020r-request-id/README.md`.
