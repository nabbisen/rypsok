# Handoff 019b — The listener's session bound and the HTTP contract as delivered

**Revision 3 (2026-10-29)**, after the re-review (`arch-002-review-019/RE-REVIEW.md`,
N1, N2, N3, N8) and the second amendment of RFC 019. Revision 2 followed the
independent review (M3, R1, R2, R3) and the owner's decision D-A (option 1:
503). Where a revision differs from the one before, the later one holds; the
changes of revision 3 are in item 1 (the wrapper's count and error type) and
item 3 (counting that is actually emitted).

**Task title.** Bound legacy sessions (LIM-H6), answer a refused session
with 503, and make every page, test and record say what the listener
does (RFC 019 D5, D6, D7 as amended).
**RFC.** [`../../done/019-first-independent-review.md`](../../done/019-first-independent-review.md),
including its amendment of 2026-10-29.
**Applicable requirements.** REQ-SEC-018, REQ-SEC-024 (new; its text is
in item 1 and the dev team copies it verbatim), REQ-MCP-006, REQ-OBS-001,
REQ-DOC-001, REQ-DOC-002, REQ-DOC-004. Threats THR-18, THR-19, THR-26,
THR-27 (new).
**Source findings.** `arch-001-prepare` §3 items 1 and 2, U2, U3, U6, U8,
B3; `dev-001-prepare` U1, U2, U3; `arch-002-review-019` M3, R1, R2, R3, D-A.
**Rules.** As 019a. **Depends on.** 019a item 4 lands **before** item 1
of this handoff (both edit `mcp/http.rs` and its tests). **May start at
once:** item 2. **Purpose.** A legacy client cannot exhaust memory and
is told plainly when it is refused; a user who follows the book reaches
the listener; the pages name the status each refusal gets and who
answers it.

## Background

The SDK creates one session per legacy `initialize` with no count limit.
A failed `create_session` is answered by the SDK with **500** and the
body `Encounter an error when create session: <our error's Display>`,
plus one ERROR log line per refusal (rmcp 3.5.0
`transport/common/server_side_http.rs`, `internal_error_response`); the
call is awaited inline in the request's task (`streamable_http_server/tower.rs`,
the `initialize` branch of `handle_post`), and the SDK calls
`close_session` when a session's worker exits, expiry included. The
protocol requires both `Accept` media types (Streamable HTTP 2026-07-28,
"Sending Messages" 2); the SDK answers 406 without them, 403 for a
foreign `Host` or `Origin`, 400 for a missing protocol header.

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | **Bound.** `[service.http] max_sessions` (LIM-H6: default 256, range 1–65536; constant `HTTP_MAX_SESSIONS` with `// LIM-H6`; `Limits::http_max_sessions`; validation `range`); `HttpSettings::max_sessions`. **Wrapper.** `BoundedSessions<M: SessionManager>` in `mcp/http/sessions.rs` delegates every trait method to `LocalSessionManager` except `restore_session`, which returns `NotSupported` (the SDK calls it only when a session store is configured, rypsok configures none, and a delegated restore would insert sessions outside the count) and **owns the count**: one `std::sync::Mutex` guarding `{ live: HashSet<SessionId>, pending: usize }`, never held across an `.await`; a poisoned lock is recovered with `PoisonError::into_inner` (both fields stay consistent after a panic in these few lines). `create_session`: under the lock refuse when `live.len() + pending >= max`, else `pending += 1`; release the lock; delegate; under the lock `pending -= 1` and, on success, insert the id. `close_session`: under the lock remove the id from `live`; release; delegate (the SDK's is idempotent). The count is `live.len() + pending`, so it cannot drift or underflow however often the SDK closes one id: its `DELETE` handler and the session worker's exit both call `close_session` for the same session, and two can run at once, which a `has_session` check before delegating would miscount. A session is counted gone a moment before the SDK has finished closing it; the bound holds to within the sessions closing at that instant, and the page says "at most" and nothing finer. No sweep. **Error.** The wrapper's `type Error` is its own enum: `LimitReached` (Display exactly `session limit reached`) and `Delegate(LocalSessionManagerError)` (Display and source transparent), because the SDK's error type has no variant that carries text. On the bound `create_session` returns `LimitReached` and sets a `tokio::task_local!` cell `REFUSED_SESSION`. **Gate.** The gate scopes the SDK call in the cell; after the call, if the cell is set it discards the SDK response and answers **503**, empty body, `Retry-After: 30` (constant `SESSION_RETRY_AFTER_S`), logs `http_refused` with `reason = "session_limit"` and the address family, and counts it. **Requirement text** (copy verbatim into `requirements.md` after REQ-SEC-023): `### REQ-SEC-024 Session bound — M7 · T` / `In HTTP mode the handshake-based revisions shall hold at most LIM-H6 live sessions; a handshake beyond the bound shall be answered 503 with a Retry-After hint and no session, logged and counted; the stateless revision is unaffected. (THR-27; RFC 019)` | `req_sec_024_sessions_are_bounded`: `max_sessions = 3`; four legacy `initialize` calls: three get a session id, the fourth gets **503**, an empty body, `Retry-After: 30` and no session id; a stateless `server/discover` still answers; after `DELETE` with one session id (the SDK supports it for legacy sessions) a new session is created; the count equals the number of live sessions throughout (expose it read-only for tests). `req_sec_024_a_session_closed_twice_is_counted_once`: on the wrapper over a real `LocalSessionManager` with `max = 3`, fill it, then close one id from two tasks at once, 200 rounds; after each round the count equals the live sessions and exactly one more create succeeds, the next is refused. `req_sec_024_the_fallback_is_a_fixed_500`: call the wrapper's refusal path with the cell unset (unit level) and assert the error's Display is exactly `session limit reached` (the text the SDK would put in a 500 body), so an SDK change that moved the call to another task fails loudly rather than leaking | configuration page row (`max_sessions`, with the sentence that a refused handshake is 503 and `Retry-After` is a hint of 30 s); limits table row LIM-H6 (`check-limits` 50); requirements REQ-SEC-024 as above; threat register row THR-27 "session exhaustion by legacy handshakes: one client opens handshakes until memory is gone" (B6; controls REQ-SEC-018, REQ-SEC-024; verified by REQ-TEST-014) and a residual row "the bound turns memory exhaustion into slot exhaustion: one caller can hold every slot for the 300 s idle expiry; per-caller quotas and a shorter expiry deferred (M8)" and a second residual row "the SDK writes one ERROR log line per refused handshake, at a rate the caller sets (log amplification); filtering that target is an M8 candidate"; review-log line |
| 2 | — | `req_doc_002_documented_curl_gets_200` in `tests/examples.rs`: start the binary with `serve --transport http` on `127.0.0.1:0`, read the port from the `listening` log line, send the exact header set of Tutorial 4's `curl` (both `Accept` media types, the three protocol headers, the body of `examples/calls/engine-status.json`'s request) with `reqwest`; assert 200 and the `engine_status` answer | Tutorial 4 and the deployment page: `Accept: application/json, text/event-stream`, with the sentence "the protocol requires a client to list both" |
| 3 | **Observability of gate and SDK refusals (R1).** After the inner call, if the status is 403 the gate logs `http_refused` with `reason = "sdk_forbidden"`, the status and the address family (no header value). **Counting.** The listener's state holds an `Arc<Counters>` (built where `HttpSettings` is built and shared with the tool handlers, so one line reports both). `Counters` gains `refused_http(reason: &'static str)`, which records under `refusals[("http", reason)]` **and runs the same once-a-minute flush check as `call`**: today only `call` does, so a flood of refused requests with no tool call would never be logged. The gate calls it for every refusal it makes or sees: `too_many_connections`, `body_too_large`, `unauthorized`, `request_timeout`, `session_limit`, `sdk_forbidden` (403), `sdk_bad_request` (400), `sdk_not_acceptable` (406); the 400 and the 406 are counted but not logged per request. `Counters::with_flush_interval(Duration)` exists for tests | the listener tests pin: foreign `Host` → **403** and one `http_refused` `sdk_forbidden`; refused `Origin` → **403**; missing `MCP-Protocol-Version` → **400**, no log line, counter incremented; `Accept: application/json` only → **406**, no log line, counter incremented; `req_obs_007_a_flood_of_refusals_is_logged_without_a_tool_call`: three foreign-`Host` requests and no tool call; with a short flush interval, one `counters` event lists `http`/`sdk_forbidden` = 3 once the interval has passed (the next refusal triggers it) | troubleshooting "Over HTTP": `Host` or `Origin` not allowed → 403 (logged), missing protocol header → 400, `Accept` without both media types → 406, refused handshake → 503; "Logs": which refusals the gate logs (`too_many_connections`, `body_too_large`, `unauthorized`, `request_timeout`, `session_limit`, `sdk_forbidden`) and which it only counts; Tutorial 4 §3 the same statuses; **stability page** Protocol row: "the gate answers 401, 404, 408, 413, 503; the pinned SDK answers 400, 403, 406; an SDK upgrade that changes them is a contract change"; external design §2 gains the listener's contract (path, headers, statuses by owner); RFC 013 amendment dated with this handoff ("Tests" and "Logging": the delivered statuses and the logging rule) |
| 4 | — | — | one sentence about a corrected key in Tutorial 2 §4, Troubleshooting and an RFC 010 amendment line: "the key is read at startup; a corrected key takes effect at the next restart, and the engine keeps being asked meanwhile, so no cooldown delays it"; the systemd and stdio `IPAddressAllow` examples use `<the engines' addresses>` as a placeholder and say the operator lists them |

## Explicit non-change scope

`legacy_revisions` default (`true`, the owner's decision); the SDK's idle
expiry; per-caller session quotas (deferred, M8); the gate's other
bounds; the tool contract; the stdio transport.

## Prohibited shortcuts

No fork or patch of `rmcp`; no parsing of the SDK's 500 body to detect
the refusal (the cell is the mechanism); no global static for the count
or the refusal (the counter lives in the wrapper, the refusal in the
task-local); no pre-check of the body in the gate to guess which POST
creates a session; no test that passes by sending the stateless
revision where a legacy session is meant; no `continue-on-error`.

## Boundaries

`rypsok::mcp::http` (and a new `mcp/http/sessions.rs`) and
`rypsok-core::config`/`limits`/`telemetry::counters` for items 1 and 3;
the book and `tests/examples.rs` for the rest. The core gains no HTTP
dependency.

## Compatibility and security constraints

A new key with a default is additive. 503 for a refused handshake is a
new status the gate owns and is documented as such. Item 1 must not
count the stateless revision's requests; the SDK's own ERROR log line
per refusal remains (an SDK matter) and is noted on the troubleshooting
page.

## Known risks

The counters of the tool handlers and the gate now share one handle:
check that no existing counters test depends on a fresh `Counters` per
listener. If the SDK's `create_session` were ever not awaited in the request's
task, the cell would stay unset and the client would see the fixed 500;
the fallback test exists so that this is a loud failure, not a leak.

## Required evidence and review request

The gates as in 019a; `check-limits` 50; the coverage gate with
REQ-SEC-024 named; the tests above green; the memory measurement before
and after the bound (the review's method: handshakes over a loopback
listener, RSS from `/proc`, 5 000 attempts with `max_sessions = 256`).
Review request at `.git-exclude/review-requests/019b-listener-and-http-docs/README.md`.
