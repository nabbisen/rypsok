# RFC 009 — Query privacy and logging

**Status.** Accepted (2026-09-30) — implementer may start after RFC 003.
**Tracks.** Milestone M1; requirements REQ-PRIV-001 to -005, REQ-OBS-005 to
-007, REQ-SEC-020 (redaction), REQ-TEST-008; external design §10, §13.3;
threats THR-09, THR-10, THR-16.
**Touches.** `rypsok-core::privacy`, `rypsok-core::telemetry`.
**Handoff.** [`../handoffs/009-privacy-and-logging/README.md`](../handoffs/009-privacy-and-logging/README.md)

## Summary

A secret check that refuses by default and names the kind and position of
what it found, never the value; structured logs on stderr that never carry
a query, a path, a query string or content; counters that make refused
calls visible.

## Design

### Secret check (REQ-PRIV-002)

`privacy::scan(text) -> Vec<Finding { kind, start, end }>` matches
high-confidence credential formats only. The initial list, each a regular
expression with a fixed prefix or structure:

| Kind | Pattern |
|---|---|
| `aws_access_key` | `(A3T[A-Z0-9]|AKIA|ASIA)[A-Z0-9]{16}` |
| `github_token` | `gh[pousr]_[A-Za-z0-9]{36,}` and `github_pat_[A-Za-z0-9_]{22,}` |
| `slack_token` | `xox[abpr]-[A-Za-z0-9-]{10,}` |
| `google_api_key` | `AIza[0-9A-Za-z_-]{35}` |
| `stripe_key` | `[sr]k_(live|test)_[0-9a-zA-Z]{24,}` |
| `openai_key` | `sk-[A-Za-z0-9_-]{20,}` |
| `private_key` | `-----BEGIN [A-Z ]*PRIVATE KEY-----` |
| `jwt` | `eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}` |
| `bearer_header` | `(?i)bearer [A-Za-z0-9_.-]{20,}` |
| `basic_auth_url` | credentials in a URL's userinfo |

The list is a reviewable constant; entropy heuristics are excluded because
they misfire on hashes and identifiers. The scan runs on the query of
`web_search` and on the whole URL of `web_fetch`, before any outbound
request (REQ-LIFE-001).

Modes (LIM-P1):

| Mode | Behavior |
|---|---|
| `refuse` (default) | `denied` with reason `secret_in_query` or `secret_in_url`, `detail.kind`, `detail.position` (character offset of the first finding); the value never appears |
| `mask` | Each finding replaced by `[redacted]`; notice `credential_masked` with the count |
| `warn` | Unchanged; a log event and a counter |
| `off` | No scan |

### Logging (REQ-OBS-005, REQ-PRIV-004)

`tracing` with a JSON subscriber writing to stderr, one event per line,
fields: `ts`, `level`, `request_id` (a per-call ULID), `event`, `tool`,
`engine`, `host` (only for engine hosts and for fetch hosts that passed the
egress policy), `latency_ms`, `count`, `outcome`, `reason`. Rules enforced
by type: the query, the URL path and query string, page content, provider
bodies and secrets are held in types that do not implement `Display` or
`Debug` (`Secret<String>` for credentials, `Sensitive<String>` for query
and URL) and therefore cannot reach a log field. `logging.log_queries =
true` unlocks a `debug`-level event with the query, and `serve` logs a
warning at start.

Levels: `error` for process-level failures; `warn` for policy refusals and
configuration warnings; `info` for one event per call with outcome and
timing; `debug` for per-engine events including truncated provider error
text.

### Counters (REQ-OBS-007, REQ-OBS-003)

In-process counters, logged as an `info` event every minute when non-zero
and exposed to `engine_status` in M2: calls per tool by outcome, refused
calls per tool and reason, engine errors per engine and class, characters
removed, results dropped. A rising `invalid_request` rate is documented as
a defect of the interface (clarity rule 12).

### Disclosure (REQ-PRIV-005)

The tool profiles of external design §3 are copied into
`docs/src/reference/tools.md` in M1 with the sentence: "rypsok promises
confidentiality hygiene, not anonymity: the engines and sites you reach see
your network address and your User-Agent."

### Tests (REQ-TEST-008)

For each pattern: a positive and a near-miss sample (no real credentials;
synthetic values). Each mode. A test that captures stderr for a call with a
secret in the query and asserts the secret is absent at every level except
the unlocked debug mode. A test that `Sensitive<String>` and
`Secret<String>` do not implement `Display`.

## Alternatives considered

- **Masking by default.** Rejected by decision D-10: a tool must not search
  for something other than what was asked.
- **Entropy-based detection.** Rejected: false positives on hashes and
  identifiers, which are legitimate search terms.
- **Logging to a file.** Rejected: stderr is the MCP convention and keeps
  retention with the operator (D-19).

## Open questions

None.
