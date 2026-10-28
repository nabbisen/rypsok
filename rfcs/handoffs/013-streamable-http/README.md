# Handoff — RFC 013: Streamable HTTP, listener protections, authentication

**RFC.** [`../../done/013-streamable-http.md`](../../done/013-streamable-http.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-MCP-003,
REQ-MCP-006, REQ-MCP-007, REQ-MCP-010, REQ-SEC-018, REQ-SEC-019,
REQ-TEST-014, REQ-DEP-002, REQ-OPS-002), `docs/src/maintainers/external-design.md`
§2, §13, §15, `docs/src/maintainers/limits-and-defaults.md` LIM-C6 to
LIM-C8, LIM-H1 to LIM-H3, LIM-O1, `docs/src/maintainers/threat-register.md`
THR-18, THR-19, `ROADMAP.md` M4.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md`.
**Depends on.** M3 closed (release 0.3.0).
**Milestone.** M4. Deliverable 8 closes the milestone.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | `[service.http]` keys and `[[service.http.auth.callers]]` in `config`, validation of RFC 013 (ids, token references, length, non-loopback bind needs callers), LIM-H1 to LIM-H3 in `Limits`, the page and `check-limits` | `rypsok check` names the key on each refusal; the page test of RFC 002 green |
| 2 | `rypsok::mcp::http`: the listener (ephemeral port in tests), the hyper service around rmcp's `StreamableHttpService` with `allowed_hosts`, `allowed_origins`, stateless 2026-07-28, legacy sessions switchable, the connection cap, the body limit, the idle timeout, shutdown on the token | Protocol tests over HTTP: discover and call stateless as JSON; legacy handshake with and without `legacy_revisions`; foreign `Host` 400; any `Origin` 403 by default and an allowed one passes; body above LIM-H2 is 413; LIM-H1 + 1 connections: the last waits or is refused |
| 3 | The authentication layer: bearer token in constant time, `401` with `WWW-Authenticate`, `CallerId` in the request extensions, `local` on loopback without callers | Tests: missing, wrong and right tokens; loopback without callers answers; the token appears in no log line (stderr capture) |
| 4 | Per-caller limiters in the adapter keyed by `CallerId` from the request context; `local` over stdio | Two callers, one exhausted, the other served; the stdio path unchanged |
| 5 | `serve --transport http` in the CLI, exit codes, `SIGTERM` closing the listener within LIM-O1 | CLI tests; a protocol test that sends `SIGTERM` during an open connection |
| 6 | Cancellation over HTTP: a dropped connection cancels the call | A fetch against a slow mock is cancelled when the client drops; the mock sees no further activity; counted `cancelled` |
| 7 | `Dockerfile` (static musl build, distroless, non-root), `reference/deployment.md`, the CI job building the image on a tag | Image builds in CI; the page covers local mode, network mode with TLS in front, systemd with egress restriction, the container, token handling |
| 8 | M4 closing: `tools/CURRENT_MILESTONE = M4`; coverage gate green; `CHANGELOG.md` for 0.4.0 with the supported revisions listed (REQ-MCP-006); threat register review of THR-18, THR-19; the security audit recorded in the package | CI green with the M4 gate |

## 2. Pull request plan

| PR | Content | Depends on |
|---|---|---|
| 1 | Configuration keys, validation, limits, page | — |
| 2 | Listener, hyper service, host and origin validation, caps, shutdown; protocol tests over HTTP | 1 |
| 3 | Authentication layer and per-caller limiters; tests | 2 |
| 4 | Cancellation test, CLI, `SIGTERM` | 2 |
| 5 | Dockerfile, deployment page, CI image job, milestone closing | 3, 4 |

## 3. Conventions that the reviewer will check

- `rypsok-core` gains no HTTP dependency; `tools/check-core-dependencies`
  stays green. Everything of this RFC lives in the binary crate and the
  configuration.
- The token is a `Secret<String>`; the comparison is constant time; no
  log field, error message or response carries a token or an `Origin`
  value.
- The adapter's tool list and behavior do not vary per caller or per
  transport (REQ-MCP-009); only the limiter instance does.
- No new framework: hyper, hyper-util, http, tower are the only
  HTTP-side crates, all already in the tree; `cargo deny` green.
- Tests bind ephemeral loopback ports and never a fixed one.

## 4. Review request package

Hand back `.git-exclude/reviews/013-streamable-http/README.md` with links to
this handoff and RFC 013, the PR list, the listener test log, the
constant-time comparison test, the threat review, the security audit, the
container build result, and any deviation as a question.
