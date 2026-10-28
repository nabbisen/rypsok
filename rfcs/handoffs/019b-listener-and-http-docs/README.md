# Handoff 019b — The listener's session bound and the HTTP contract as delivered

**Task title.** Bound legacy sessions (LIM-H6) and make every page, test
and record say what the listener does (RFC 019 D5, D6, D7).
**RFC.** [`../../accepted/019-first-independent-review.md`](../../accepted/019-first-independent-review.md).
**Applicable requirements.** REQ-SEC-018, REQ-SEC-024 (new, added by this
handoff), REQ-MCP-006, REQ-DOC-001, REQ-DOC-002, REQ-DOC-004. Threats
THR-19, THR-26, THR-27 (new, added by this handoff).
**Source findings.** `arch-001-prepare` §3 item 2 and item 1, U2, U3, U6,
U8, B3; `dev-001-prepare` U1, U2, U3.
**Rules.** As 019a. **Depends on.** Nothing; may run beside 019a.
**Purpose.** A legacy client cannot exhaust memory, and a user who
follows the book reaches the listener.

## Background

The listener creates one SDK session per legacy `initialize` with no
count limit (5 000 sessions in 1.4 s, 23 to 270 MiB in the review's
measurement). The documented `curl` lacks `text/event-stream` in
`Accept` and gets 406. The documented status for a foreign `Host` (400)
is the SDK's 403. The SDK answers `Host`, `Origin` and missing-header
refusals before the gate sees them, so they are not logged as
`http_refused`, against the troubleshooting page and RFC 013.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | `[service.http] max_sessions` (LIM-H6: default 256, range 1–65536; constant `HTTP_MAX_SESSIONS` with the `// LIM-H6` comment; `Limits::http_max_sessions`; validation `range`); `HttpSettings::max_sessions`; a `BoundedSessions<M: SessionManager>` in `mcp/http.rs` (or a sibling module) that delegates every trait method to `LocalSessionManager` and refuses `create_session` and `initialize_session` when the count of live sessions is at the bound, decrementing on `close_session` and on the SDK's expiry (read `rmcp` 3.5.0 `streamable_http_server/session.rs` for the trait and `session/local.rs` for how expiry is signaled; if expiry cannot be observed, count by querying `has_session` on a periodic sweep of known ids and say so in the review request); the refusal logs `http_refused` with reason `too_many_sessions` and the address family | `req_sec_024_sessions_are_bounded`: `max_sessions = 3`; four legacy `initialize` calls: three get a session id, the fourth gets no session id and a non-2xx status (record the exact status the SDK returns); the stateless revision still answers; after a session is closed (`DELETE` with its id, if the SDK supports it, else after the test advances past expiry with a paused clock if possible, else skip with a stated reason) a new one can be created | configuration page row; limits table row LIM-H6 (`check-limits` 50); requirements: `REQ-SEC-024 Session bound — M7 · T` after REQ-SEC-023; threat register row THR-27 and a review-log line; stability page unchanged except the new key's existence is additive |
| 2 | — | `req_doc_002_documented_curl_gets_200` in `tests/examples.rs`: start the binary with `serve --transport http` on `127.0.0.1:0` (read the port from the `listening` log line), send the exact header set of Tutorial 4's `curl` (both `Accept` media types, the three protocol headers, the JSON body of `examples/calls/engine-status.json`'s request) with `reqwest`; assert 200 and the `engine_status` answer; a second request with `Accept: application/json` only must get 406, pinning the reason the page says both | Tutorial 4 and the deployment page: `Accept: application/json, text/event-stream`; the sentence "the SDK answers 406 without both" |
| 3 | — | the listener test for a foreign `Host` asserts **403** exactly; a test for a missing `MCP-Protocol-Version` asserts **400** | troubleshooting "Over HTTP": `Host` not allowed → 403, missing header → 400; Tutorial 4 §3 the same; the stability page's Protocol row: 400, 401, 403, 404, 408, 413 with one clause each; external design §2 gains the listener's contract (path, headers, statuses) as a short table; RFC 013's "Tests" and "Logging" paragraphs get an amendment dated with this handoff: the SDK answers `Host`, `Origin` and header refusals with 403/400 and they are not logged as `http_refused`; the gate logs `too_many_connections`, `body_too_large`, `unauthorized`, `request_timeout`, `too_many_sessions` |
| 4 | — | — | one sentence about a corrected key, used in Tutorial 2 §4, Troubleshooting and RFC 010's amendment: "the key is read at startup; a corrected key takes effect at the next restart, and the engine keeps being asked meanwhile, so no cooldown delays it"; the systemd and stdio `IPAddressAllow` examples use `<the engines' addresses>` as a placeholder with the sentence that the operator lists them |

## Explicit non-change scope

`legacy_revisions` default (`true`); the SDK's idle expiry; the gate's
other bounds; the tool contract; the stdio transport.

## Prohibited shortcuts

No fork or patch of `rmcp`; no global static for the session count (it
lives in the wrapper); no test that passes by sending the stateless
revision where a legacy session is meant; no `continue-on-error` on the
new test.

## Boundaries

`rypsok::mcp::http` and `rypsok-core::config`/`limits` for item 1; the
book and `tests/examples.rs` for the rest. The core gains no HTTP
dependency.

## Compatibility and security constraints

A new key with a default is additive. The 403/400 corrections change
documents, not behavior. Item 1 must not count the stateless revision's
requests.

## Known risks

The SDK's error path for a refused session creation decides the status
the client sees; report it exactly, do not pick one.

## Required evidence and review request

The gates as in 019a; `check-limits` 50; the coverage gate with
REQ-SEC-024 named; the four new or extended tests green; the review
request at `.git-exclude/review-requests/019b-listener-and-http-docs/README.md`
with the status the SDK returned for a refused session and the memory
measurement before and after (the review's method: sessions over a
loopback listener, RSS from `/proc`).
