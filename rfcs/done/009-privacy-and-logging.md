# RFC 009 — Query privacy and logging

**Status.** Implemented (0.1.0); accepted 2026-09-30.
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

## Amendment to RFC 009 (2026-10-20, M1 review)

- **Foreign log targets are capped.** The MCP SDK logs every request it
  receives at `debug`, query included. The subscriber therefore gives the
  configured level to the crates `rypsok` and `rypsok_core` only and caps
  every other target at `warn`, whatever the level. A new dependency
  cannot leak a query or a URL through the log.
- `Sensitive` wraps the query from validation to the drivers
  (`ValidatedSearch::query`, `EngineRequest::query`); a driver reads it
  through `expose()`.
- `logging.log_queries = true` emits one `debug` event `query` with the
  text; `serve` warns at start.
- Warn mode counts under `warnings`, a counter of its own; a warning is
  not a refusal.
- The request identifier is the start time in milliseconds and a process
  sequence number, in hexadecimal: unique within a process, which is what
  log correlation needs, without a randomness dependency.

## Amendment to RFC 009 (2026-11-03, RFC 020 handoff 020n, DAT-15)

Audit 020c tried 12 ordinary strings and 17 credential-shaped ones: the check
refused 9 of the ordinary ones (hyphenated words after `sk-` and after
`xoxb-`, a phrase after `bearer`, `ASIA` and a 16-letter word, `sk_test_` and a
plain word, an empty password in `ssh://git:@host/…`, a host and a port before
the `@` of a URL, three long dotted words starting `eyJ`) and passed 14 of the
others. **The corpus is the specification**: it is
committed as the test table of `privacy/tests.rs`, both lists, with made-up
keys, and every rule below is decided by lines of it. The principle: a false
refusal of ordinary text confuses the user and trains them to switch the check
off; a miss lets an accidental credential leave. Both are fixed by making a
tail look like a credential: one unbroken run of the key alphabet with the
length of the real format, not hyphenated words.

- **Changed formats.** AWS key id: `AKIA`, `ASIA` or `A3T[A-Z0-9]` and
  sixteen base-32 characters (`[A-Z2-7]`), case-insensitive. For `ASIA` and
  `A3T` the tail needs **at least one digit** (`ASIA` and a 16-letter word has
  none); for `AKIA` the pattern decides, since `AKIA` is not a word and about
  1 in 27 real ids has no digit in its tail (review of 2026-11-03). Slack: `xox[abprs]-`, one to three runs of 8 to 13 digits,
  a 20 to 64 character secret. Stripe: the tail needs a digit, a capital and a
  lower-case letter. OpenAI: the legacy key is `sk-` and 40 or more base-62
  characters with a digit, a capital and a lower-case letter; the project,
  service account, admin, Anthropic and OpenRouter keys are `sk-proj-`,
  `sk-svcacct-`, `sk-admin-`, `sk-ant-`, `sk-or-v1-` and 30 or more characters
  that are not hyphenated words. JWT: the first two segments must decode
  (base64url) to JSON objects and the first must name `"alg"` or `"enc"`.
  Bearer: the token follows `Bearer` and a space, `%20` or `+`, is 20 or more
  characters and is not hyphenated words (two or more parts of three or more
  lower-case letters). Basic-auth URL: a password that is not at most five
  digits after a dotted name (that is `host:port@`), and not empty.
- **New formats** (new `detail.kind` values, snake case): `npm_token`
  (`npm_` and 36 base-62 characters), `gitlab_token` (the `glpat` prefix, a hyphen and 20 or more),
  `huggingface_token` (`hf_` and 34 letters of both cases), `sendgrid_key`
  (`SG.` 22 `.` 43), `telegram_bot_token` (8 to 10 digits, `:`, 35
  characters), `twilio_key` (`SK` and 32 hexadecimal), `aws_secret_key` (named
  by `aws_secret_access_key` and the like, then 40 characters),
  `password_assignment` (`password`, `passwd` or `secret`, also as the end of
  a name such as `client_secret`, then `=` or `:` and a value of 8 or more
  characters with a digit or a symbol, which is not a placeholder such as
  `<…>`, `${…}` or `****`), `basic_auth_header` (`Authorization: Basic` and a
  payload that decodes to `user:password`), `azure_account_key` (`AccountKey=`
  and 40 or more base-64 characters), and a GitHub token split by one space or
  line break.
- **Invisible characters are read through.** The scan runs on the text with
  control and format characters (a zero width space or joiner) taken out, so
  that `AKIA<ZWSP>IOSFODNN7EXAMPLE` is found; the positions reported, and the
  range `mask` replaces, are those of the original text and cover the
  invisible character inside.
- **Overlap.** The table is ordered, specific before general; a finding that
  shares a character with one already found is dropped (`Bearer ghp_…` is a
  GitHub token), while findings that only touch are both kept. This is one
  small function with its own test.
- **No per-pattern override**; the modes of LIM-P1 stay global.
