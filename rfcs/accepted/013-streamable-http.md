# RFC 013 — Streamable HTTP, listener protections, authentication, per-caller limits

**Status.** Accepted (2026-10-27) — implementer may start after M3 closes
(release 0.3.0).
**Tracks.** Milestone M4; requirements REQ-MCP-003, REQ-MCP-006 (served
revisions over HTTP), REQ-MCP-007 (cancellation over HTTP), REQ-MCP-010
and REQ-SEC-019 (per-caller limits), REQ-SEC-018, REQ-TEST-014,
REQ-DEP-002, REQ-OPS-002 (shutdown of the listener); external design §2,
§13.1, §13.2, §15; threats THR-18, THR-19; decisions D-01, D-12, D-27,
D-29, D-35.
**Touches.** `rypsok::mcp::http` (new), `rypsok::cli` (`serve --transport
http`), `rypsok::mcp` (caller identity in the adapter), `rypsok-core::config`
(`[service.http]`, `[service.http.auth]`), the limits table (three rows), a
`Dockerfile`, the operator documentation.
**Handoff.** [`../handoffs/013-streamable-http/README.md`](../handoffs/013-streamable-http/README.md)

## Summary

The same adapter, served over Streamable HTTP by the SDK's tower service
behind a thin layer that rypsok owns: it validates `Host` and `Origin`,
authenticates the caller with a static bearer token, attaches the caller's
identity to the request, and refuses to listen off loopback without
tokens. The 2026-07-28 revision is served statelessly as the protocol
requires; the handshake-based revisions keep their sessions in memory
until the operator disables them. Rate limits apply per authenticated
caller. The core is untouched: nothing of HTTP reaches `rypsok-core`.

## Design

### Listener (REQ-SEC-018, REQ-MCP-003)

`rypsok serve --transport http` binds `[service.http].bind` (default
`127.0.0.1:8737`) with `tokio::net::TcpListener` and serves each
connection with `hyper` 1 through `hyper-util`'s automatic HTTP/1 and
HTTP/2 builder, wrapping rmcp's `StreamableHttpService` with
`TowerToHyperService`. One path, `/mcp`; everything else is 404. The
HTTP+SSE transport of 2024-11-05 is not offered (D-12): there is no
separate SSE endpoint.

```rust
pub struct HttpSettings {
    pub bind: SocketAddr,                 // [service.http].bind
    pub allowed_hosts: Vec<String>,       // [service.http].allowed_hosts, default: loopback names and the bind host
    pub allowed_origins: Vec<String>,     // [service.http].allowed_origins, default: none (every present Origin refused)
    pub legacy_revisions: bool,           // [service.http].legacy_revisions, default true (REQ-MCP-006)
    pub callers: Vec<Caller>,             // [service.http.auth].callers
    pub max_connections: u32,             // LIM-H1
    pub max_request_bytes: u64,           // LIM-H2
    pub idle_timeout: Duration,           // LIM-H3
}
pub struct Caller { pub id: CallerId, pub token: Secret<String> }   // token by env or file, never inline (REQ-CFG-004)
```

rmcp's service is configured with `allowed_hosts` and `allowed_origins`
(`enforce_origin_validation()` when the list is empty, so that a present
`Origin` is always refused unless listed), `legacy_session_mode = false`,
`json_response = true`, and the process's shutdown token as its
cancellation token. Sessions for the handshake-based revisions live in the
SDK's `LocalSessionManager`; with `legacy_revisions = false` the
`NeverSessionManager` is used and those revisions are refused.

The listener accepts at most LIM-H1 connections at once (a semaphore
around `accept`), reads at most LIM-H2 bytes of a request body (a body
limit layer), and closes an idle connection after LIM-H3. TLS is not
terminated by rypsok in M4: a deployment off loopback puts a reverse proxy
or a tunnel in front, and the documentation says so (DR-010 covers a
shared service).

### Authentication (D-27, REQ-SEC-018)

A static bearer token per caller, configured by reference:

```toml
[service.http]
bind = "0.0.0.0:8737"
allowed_hosts = ["rypsok.corp.example", "rypsok.corp.example:8737"]
allowed_origins = []                        # browsers never call rypsok

[[service.http.auth.callers]]
id = "mimlys"
token_env = "RYPSOK_TOKEN_MIMLYS"

[[service.http.auth.callers]]
id = "ops"
token_file = "/run/secrets/rypsok-ops"
```

Validation (RFC 004): a caller `id` is lowercase letters, digits and
underscores; exactly one of `token_env` or `token_file`; ids unique; a
token shorter than 32 bytes is refused; `bind` off loopback with no
callers is refused with `service.http.bind: a non-loopback bind needs
[service.http.auth] callers`. On loopback, callers are optional: without
any, every request is the caller `local`; with callers configured, the
token is required on loopback too.

The layer reads `Authorization: Bearer <token>`, compares it with every
configured token in constant time, and attaches `CallerId` to the request
extensions; a missing or unknown token is `401` with
`WWW-Authenticate: Bearer realm="rypsok"` and an empty body, before any
JSON is parsed. Tokens never reach a log (`Secret<String>`); the caller id
does (`caller` field in the `call` event).

### Per-caller limits (REQ-SEC-019, REQ-MCP-010)

The adapter keeps one `ToolLimiter` set per caller id: a map from
`CallerId` to the three limiters, created on first use, bounded to the
configured callers plus `local` (there are no other ids). The caller id
comes from the request extensions that rmcp forwards into
`RequestContext`; over stdio the caller is `local`. The limits are
LIM-C6 to LIM-C8 per caller; the process-wide outbound caps (LIM-C1,
LIM-C3) stay process-wide.

### Cancellation and shutdown (REQ-MCP-007, REQ-OPS-002)

A closed response stream cancels the request's token through the SDK; the
pipelines already observe it. `SIGTERM` and Ctrl-C cancel the shutdown
token: the listener stops accepting, in-flight requests are cancelled, and
the process exits within LIM-O1.

### Logging (RFC 009)

One `info` event per call as today, with `caller`. Refusals of the layer
log `warn` `http_refused` with `reason` in `host`, `origin`, `unauthorized`,
`too_many_connections`, `body_too_large` and the remote address's family,
never the token and never the `Origin` value.

### Deployment (REQ-DEP-002, REQ-DEP-003)

A `Dockerfile` builds a static binary (`x86_64-unknown-linux-musl`) into a
distroless image that runs as a non-root user, exposes 8737, and takes the
configuration from `/etc/rypsok/rypsok.toml`. The book gains
`reference/deployment.md`: local mode, network mode with a reverse proxy
terminating TLS, the systemd unit with the egress restrictions of RFC 011,
the container, and the token handling. The CI builds the image on a tag.

### Tests (REQ-TEST-014)

Protocol tests over HTTP with a loopback listener on an ephemeral port:
`server/discover` and a `tools/call` with the 2026-07-28 metadata are
stateless and return `application/json`; the legacy `initialize` handshake
works with a session and is refused with `legacy_revisions = false`; a
request with a foreign `Host` is 400 and one with any `Origin` is 403 by
default; the loopback default without callers answers as `local`; a bind
off loopback without callers is refused at `rypsok check`; with callers, a
missing token is 401 and the right token passes; two callers have separate
rate limits (the second is not refused when the first is exhausted); a
request body above LIM-H2 is 413; dropping the connection mid-call cancels
the fetch (the mock server sees no further read); `SIGTERM` closes the
listener within LIM-O1. A unit test asserts the token comparison takes
constant time in structure (no early return on length or content).

## Amendment to the baseline

The limits table gains three rows, applied with this RFC:

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-H1 | HTTP connections accepted at once | 64 | 1–4096 | REQ-SEC-018; bounded memory |
| LIM-H2 | Size of an HTTP request body | 65536 bytes | 4096–1048576 | A tool call is small; THR-11 |
| LIM-H3 | Idle HTTP connection timeout | 60 s | 5–600 s | Resource hygiene |

The configuration schema (RFC 004) gains `[service.http]` keys
`allowed_hosts`, `allowed_origins`, `legacy_revisions`,
`max_connections`, `max_request_bytes`, `idle_timeout_s`, and the table
array `[[service.http.auth.callers]]`.

## Alternatives considered

- **OAuth 2.1 / the SDK's auth module.** Rejected for M4 by D-27 and D-29:
  one trust domain, one or a few callers; a static token by reference is
  what the deployment needs. A shared service is DR-010.
- **TLS in the binary.** Rejected: certificate handling is the proxy's or
  the tunnel's job in one trust domain; the design says so instead of
  shipping a half-configured TLS stack.
- **A separate HTTP crate (axum).** Rejected: rmcp exposes a tower service
  and hyper is already in the tree through reqwest; one more framework
  adds surface for one path.
- **Per-IP limits instead of per-caller.** Rejected: the requirement is
  per authenticated caller; addresses are shared behind proxies.

## Open questions

- Whether `allowed_hosts` should default to the bind host only, refusing
  `localhost` when the bind is an address. Decided here: loopback names and
  the bind host, which is what a local host uses.
- Whether the container image should be published to a registry by CI or
  only built. Decided here: built on a tag and attached as a workflow
  artifact; publication needs the owner's registry decision.

## Amendment to RFC 013 (2026-10-27, M4 review)

Decided by the architect under the owner's two principles (a clean, safe,
robust design; interfaces that cannot be misread), after the review of
the M4 implementation and release 0.4.0.

- **The standard headers are part of the contract over HTTP.** A request
  of the 2026-07-28 revision carries `MCP-Protocol-Version`, `Mcp-Method`
  and, on `tools/call`, `Mcp-Name` beside the `_meta` of its body; the SDK
  refuses a request without them with 400. The integration guide and the
  deployment page state it with a complete example.
- **`legacy_revisions = false` refuses, as the design said.** The SDK
  answers `initialize` statelessly on its own; rypsok therefore advertises
  the 2026-07-28 revision alone when the switch is off
  (`ServerHandler::supported_protocol_versions`), so that `initialize`
  gets the SDK's `UNSUPPORTED_PROTOCOL_VERSION` (`-32022`) and
  `server/discover` lists one revision. Over stdio nothing changes. A key
  named "legacy revisions" whose `false` still served them would be a
  misreading waiting to happen.
- **LIM-H3 on both HTTP versions.** HTTP/1: the header read timeout, the
  time a connection may sit between requests. HTTP/2: a keep-alive ping
  after the same idle time, the connection closed when the ping goes
  unanswered within 10 s, so a dead peer frees its LIM-H1 permit; an idle
  peer that answers pings holds its permit, which only an authenticated
  caller (or a loopback client) can do.
- **LIM-H2 is judged twice, with one answer.** The gate refuses a declared
  `Content-Length` above the bound and the SDK refuses a streamed body
  that grows past it; both answer 413. Confirmed as the split.
- **The image stays a workflow artifact.** Publication to a registry
  needs the owner's registry and account; nothing in the design depends
  on it.
- **An unspecified bind names its hosts.** A bind on `0.0.0.0` or `::`
  is refused at startup unless `allowed_hosts` is set: the default list
  would admit no name a client sends, and every request would end in a
  400 that looks like a client fault. The bind host is no longer added to
  the default list when it is unspecified.
