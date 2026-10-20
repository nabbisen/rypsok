# Integration guide for hosts

This page is for the author of an agent runtime that embeds rypsok.

## Transport and lifecycle

- Start `rypsok serve` as a child process and speak JSON-RPC over its
  stdin and stdout, one message per line. Standard output carries
  protocol messages only; read standard error for logs.
- rypsok implements MCP revision 2026-07-28 (`server/discover`, stateless
  requests with `_meta`) and keeps serving the handshake-based revisions
  the SDK supports. Either works; use what your client library sends.
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
- Restarting rypsok resets its in-memory state (circuit states, counters);
  nothing else is kept between calls.

## Embedding the core

`rypsok-core` is a library crate without any protocol dependency. A host
written in Rust may call `Rypsok::search` directly with its own
cancellation token and render the response itself. The interface is
unstable until 1.0.0.
