# Handoff 022a — rmcp 3.9.0, and unknown tool names refused at our gate

> **Amended 2026-11-11 at the stop of item 3:** items 4 (the gate) and 5's
> gate are withdrawn with RFC 022 D2. The rest follows the architect's
> decisions in the 022a review: the wire differences are announced, and
> the 16 MiB stdio line bound becomes a row of the limits table with a
> test.

**Created 2026-11-11.** **RFC.** [`../../done/022-rmcp-advisories.md`](../../done/022-rmcp-advisories.md) (D1 to D3). **Priority: before anything else.** This is a security fix of released code (High).
**Rules.** As 021a: tests first and shown failing; push only after approval; evidence goes to `.git-exclude/evidence/022a/`; relative `cd`. Also:
- **The per-change rule:** `mcp/http.rs` is in its list, so run mutation testing and fuzz the `headers` target for 120 s.
- **The tool-definition rule:** if a definition changes, it is announced, and you stop and ask first.

**Depends on.** 021a (`849f96a`, approved, not pushed). Build on top of it. Both are pushed together.

## Change scope

| # | Change | Test (fails before) |
|---|---|---|
| 1 | **Record the answers of today first.** Before changing anything, record with the **released 0.9.0 binary** (from the release page, checksum checked):<br>• over stdio: `initialize` for each of `2024-11-05`, `2025-03-26`, `2025-06-18`, `2025-11-25` and `2026-07-28`, and `server/discover`;<br>• over HTTP: the same, plus the exact answer (status, headers that matter, body) to a 2026-07-28 `tools/call` for an unknown tool name, sent with and without `Mcp-Name`;<br>• `tools/list` for both transports.<br>Keep them in the evidence. The architect's probe scripts are in `.git-exclude/evidence/rmcp-0461/` | — (the baseline) |
| 2 | **D1, the upgrade.** `rmcp = "=3.9.0"` (same features), then `cargo update -p rmcp --precise 3.9.0`, and adapt the code to the API. List every change you had to make, and every entry of rmcp's changelog from 3.5.1 to 3.9.0 that touches the server, the stdio or Streamable HTTP server transports, negotiation or schemas, each with "applies" or "does not apply" and why. Show the lock-file diff. `cargo deny check` green | `cargo deny check` (red before) |
| 3 | **The wire stays the same.** Repeat item 1 with the new build and compare. **The tool definitions** (`tools_list.json`, `instructions.txt`) must be byte-identical; the fixture tests show it. **The revisions and the handshake answers** must be the same. **If anything differs, stop and report** before deciding anything: a difference is announced under `### Tool definitions` (RFC 021) or kept out, and it is the architect's call | the existing fixture and protocol tests; the comparison in the evidence |
| 4 | **D2, the gate.** In the HTTP gate (`mcp/http.rs`), before the request reaches `StreamableHttpService`, a 2026-07-28 `tools/call` whose tool name is not one of rypsok's tools is answered **exactly as item 1 recorded** (status and body), without calling the service. The name comes from the `Mcp-Name` header; the SDK validates that the header and the body agree, and so keep that order of checks if it matters. A name that is served goes on as today. The refusal is counted with the gate's other refusals, under the same once-a-minute log. No new error kind, no new HTTP status | A gate unit test with an inner service that counts its calls: an unknown name is answered as recorded, and the inner service is **not** called. A served name reaches it. A protocol test over HTTP: unknown name → the recorded answer |
| 5 | **The memory, measured.** Run the architect's `probe.py` (4,000 distinct names) and `control.py` against the new build, in HTTP mode on loopback with no callers. Expected: no growth beyond the control's. The numbers go in the review request (this is evidence, not a CI test: an RSS test would be flaky) | — |
| 6 | **D3, the changelog**, under `### Security` in `[Unreleased]`: "rmcp is upgraded from 3.5.0 to 3.9.0 (RUSTSEC-2026-0461, -0457, -0462). In HTTP mode, every release from 0.4.0 to 0.9.0 let a client that reaches the endpoint grow the process's memory without bound, by sending `tools/call` requests with distinct tool names, until it failed: any local process when no callers are configured, or any authenticated caller. Unauthenticated clients were refused first when callers are configured, and stdio was not affected. rypsok's gate now also refuses an unknown tool name before the protocol library sees it. If you cannot upgrade at once, configure callers or use stdio. -0457 and -0462 concern client code that rypsok does not use." Adjust the wording to what item 5 measured | — |
| 7 | **The threat register.** One line for the residual and the control (the gate now refuses unknown names; the SDK no longer caches), in `threat-register.md`, on the HTTP gate's threat | `req_doc_001` |

## Explicit non-change scope

- No other dependency is upgraded. The cycle's full dependency step is
  not this handoff.
- No change to any answer, revision or tool definition: item 3 holds us
  to that.
- No new limit.

## Required evidence and review request

1. The gates, with `cargo deny` green.
2. Items 1, 3 and 5.
3. Failing first.
4. Mutation of `mcp/http.rs`, and the `headers` fuzz run.
5. The review request at `.git-exclude/review-requests/022a-rmcp-advisories/README.md`.
