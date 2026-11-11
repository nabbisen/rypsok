# RFC 022 — The rmcp advisories of November 2026 (RUSTSEC-2026-0461, -0457, -0462)

**Status.** Implemented (0.10.0, 2026-11-11); accepted (2026-11-11) by the architect as a security fix of
released code. The fix is the architect's (a dependency step, RFC 020,
D-14 as amended by D-23). The release that carries it, and any public
advisory, are the owner's.
**Tracks.** 0.10.0 (the owner's decision of 2026-11-11). Requirements: REQ-SEC-018 (the HTTP gate),
REQ-SEC-024 (bounded memory), REQ-DEP-002, REQ-MAIN-006.
**Handoffs.** [`022a`](../handoffs/022a-rmcp-advisories/README.md); [`022b`](../handoffs/022b-release-preparation/README.md), the preparation of 0.10.0, which carries RFC 021 and this RFC.
**Assessment.** The architect's assessment (private review folder), with
the reproduction against the released 0.9.0 binary.

## Problem

`rmcp 3.5.0` is pinned in every release with the HTTP transport, 0.4.0 to
0.9.0. **RUSTSEC-2026-0461** applies to that transport:

- **The flaw.** A process-wide map of `tools/call` tool names, with no
  bound, filled before a request is rejected.
- **The reproduction** against 0.9.0 on loopback: 4,000 requests with
  distinct 8 KB names, all answered 400, took the process from 11 to
  44 MiB; one repeated name took it to 13.
- **Who is exposed, in HTTP mode:**
  - any local process, when no callers are configured;
  - any authenticated caller, when callers are configured.
- **Who is not:** unauthenticated clients when callers are configured
  (the gate answers 401 first), and **stdio**.
- **Severity: High** by RFC 020's rubric.

-0457 and -0462 are in client code that rypsok does not enable, but they
are on the lock file.

## Design

- **D1. Upgrade `rmcp`** to the latest 3.x that fixes all three
  advisories, today `3.9.0`, pinned exactly (`=3.9.0`). The wire behavior
  must stay as it is: the same revisions, the same handshake answers, the
  same tool definitions. Any difference is either kept out or announced.
- **D2. Defense in depth at our gate**, as the advisory itself
  recommends. A `tools/call` request whose tool name is not one that
  rypsok serves is refused **before** `StreamableHttpService`, with the
  answer the SDK gives today for that request. The answer of the
  protocol does not change, and no unknown name ever reaches the SDK
  again. A future regression of the same kind in the SDK then cannot be
  reached through rypsok.
- **D3. Say it plainly.** A `### Security` entry in the changelog: what
  was exposed, which versions, in which mode, the mitigation for anyone
  who cannot upgrade at once (callers configured, or stdio), and the
  RUSTSEC ids.

## Compatibility

No change to any answer, revision or definition, by design. D1 may change
nothing on the wire unless the change is announced. The tool-definitions
rule of RFC 021 applies, and its release-check job will hold us to it.

## Alternatives considered

- **Upgrade only.** It fixes the flaw, but keeps us dependent on the SDK
  never caching per request again. D2 costs a few lines and removes the
  dependency.
- **3.6.0, the smallest fixing version.** It leaves -0457 and -0462 on the
  lock file, red in `cargo deny`, and a smaller step now means a larger one
  later. mimlys pins `=3.9.0` too.

## Amendment (2026-11-11, at the stop of 022a item 3): D2 withdrawn

- **What 022a found.** The upgrade alone removes the growth. With 4,000
  distinct names the process stayed level with the control (24 → 27 MiB
  both).
- **What D2 would cost.** To answer exactly as the protocol library does,
  the gate would have to:
  - read and parse request bodies, which it never does;
  - copy the library's matching of headers against the body;
  - change which refusal some malformed requests get (a foreign `Host`
    with an unknown tool would no longer be 403).

  That is more parsing of untrusted input and less clarity, set against a
  flaw removed at its source. **D2 is withdrawn.**
- **The guard against a regression** is `cargo deny`, on every push and in
  the release checks. That is how this advisory was found.
- **What rmcp 3.9.0 changes on the wire, all announced:**
  - unknown tool over HTTP is 200 with the same JSON-RPC error (was 400);
  - a 2026-07-28 `tools/list` carries `cacheScope` and `ttlMs`, with the
    tools unchanged;
  - float and `null` ids;
  - `Accept` quality values and case;
  - a legacy session-less POST is 400 (was 422);
  - the stdio first message without `_meta`;
  - a stdio line bound of 16 MiB, which becomes a row of the limits table.

  No tool definition changed.
