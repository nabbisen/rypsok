# Integration guide for hosts

This page is for the author of an agent runtime that embeds rypsok.

## Transport and lifecycle

- Start `rypsok serve` as a child process and speak JSON-RPC over its
  stdin and stdout, one message per line. Standard output carries
  protocol messages only; read standard error for logs.
- rypsok implements MCP revision 2026-07-28 (`server/discover`, stateless
  requests with `_meta`) and keeps serving the handshake-based revisions
  the SDK supports. Either works; use what your client library sends.
- Over HTTP (`serve --transport http`, see [deployment](./deployment.md)),
  every request of the 2026-07-28 revision carries the standard headers
  beside the `_meta` of the body: `MCP-Protocol-Version: 2026-07-28`,
  `Mcp-Method` with the JSON-RPC method, and `Mcp-Name` with the tool
  name on `tools/call`. A request without them is refused with 400. With
  `legacy_revisions = false` the handshake-based revisions are refused:
  `initialize` gets the JSON-RPC error `-32022` and `server/discover`
  lists the 2026-07-28 revision alone.
- Closing stdin or sending `SIGTERM` starts a graceful shutdown; the
  process exits within 3 seconds and aborts in-flight calls.
- Cancel a call with `notifications/cancelled`; rypsok aborts its engine
  requests within 500 ms and sends no response for it.

## What to expect from a call

- Exactly one outcome per call: `ok`, `partial`, `empty`, or a tool
  execution error (`isError: true`) with one error object. Protocol
  errors occur only for an unknown tool or a malformed message.
- Errors say what the caller can change; `invalid_request` names the
  parameter and the accepted range or values. A model can act on them.
- Results are bounded and complete: at most `max_results`, no pagination.
- `structuredContent` carries the response object and `content[0].text`
  the same JSON. For a host that would feed both to a model, set
  `service.structured_content = false` to get the text alone.

## What a host should do

- Treat every string under `results` as external data: show it, store it,
  quote it, but never execute it. The server instructions say so to the
  model; the host should say so in its own prompt too.
- Keep `engine_status` away from the model unless the agent is meant to
  diagnose connectivity; it is for operators.
- Expect `partial` often: one engine failing is not an error.
- Do not retry a `denied` error; retry `unavailable` after
  `retry_after_ms` when present.
- Restarting rypsok resets its in-memory state (circuit states, counters,
  the result cache when enabled); nothing else is kept between calls. A
  result served from the cache is marked by the `from_cache` notice and
  may be up to `[cache] ttl_s` old; over HTTP the cache is per caller.

## Restricting egress at the operating-system level

The egress policy is rypsok's own line of defense; a deployment should
add the operating system's. The policy cannot see what the kernel sees,
and a bug in one does not reach the other.

With systemd, deny every address and allow the engines and the sites the
deployment needs:

```ini
[Service]
ExecStart=/usr/local/bin/rypsok serve --transport stdio
IPAddressDeny=any
IPAddressAllow=93.184.216.0/24
RestrictAddressFamilies=AF_INET AF_INET6
PrivateNetwork=no
```

In a container, attach rypsok to a dedicated bridge network whose firewall
forwards only to the Internet and never to the host, the container network
or a metadata address (for example `169.254.169.254/32` and
`fd00:ec2::254/128` dropped in the forward chain), and give it no
capability to change its own routes.

Either way, keep `[egress].allow` empty unless a private destination is
intended, and prefer `fetch.strict_mode = true` where the agent should only
read what it searched for.

## Embedding the core

`rypsok-core` is a library crate without any protocol dependency. A host
written in Rust may call `Rypsok::search` directly with its own
cancellation token and render the response itself, or `Rypsok::search_as`
to name the caller whose cache entries and limits the call belongs to. A
call ends with a response, with `CallError::Tool` (the error object to
return), or with `CallError::Cancelled` when the host cancelled it and
there is nothing to return. A host that assembles its own `Registry` and
`FetchService` shares the parsing permits between them with
`FetchService::with_parsing`, as `Rypsok::from_parts` does, so that one
bound (LIM-C5) holds for the process. From 1.0.0 the interface follows
the [stability page](./stability.md): the public types are
`#[non_exhaustive]` and grow through new fields, variants and setters,
so match them with a wildcard arm and build requests and driver types
through their constructors.
