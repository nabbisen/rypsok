# Handoff 020l — `check` tells the truth; configuration messages a user can act on

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3), wave 2.
Owner decisions **D-6, D-20, D-21** (authorized 2026-11-03,
`arch-003-audit/DECISIONS-REVIEWED.md`). Findings DAT-1, DAT-2 (Medium);
DAT-11, DAT-12, DAT-14 (Low); PRF-2, PRF-3 (Medium).

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md), RFC 004 (configuration), RFC 016 (`check` writes to stderr).
**Applicable requirements.** REQ-OPS-001, REQ-CFG-*, REQ-PERF-003, REQ-DOC-001.
**Rules.** As 020h (tests first and shown failing; commit your own work; push only after approval; evidence to `.git-exclude/evidence/020l/`; relative paths with `cd`).
**Depends on.** Wave 1 (020i changed LIM-F11; the memory figure below uses the measured costs of audit 020e).

## Change scope

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | **D-20, D-21: lower two ceilings.** `fetch.max_bytes` range becomes 64 KiB to **10 MiB**; `service.http.max_sessions` becomes 1 to **2048**. A value above is refused with the usual "must be between" message. Defaults unchanged | Range-edge tests for both keys (the existing config range table test) | `limits-and-defaults.md` LIM-F3 and LIM-H6 rows (and "about 60 KiB per session" beside LIM-H6); `configuration.md`; changelog **Changed**, announced as a contract change |
| 2 | **DAT-1. `check` resolves what `serve --transport http` resolves.** Build `HttpSettings::from_config` in `check` when the file configures callers (`[service.http.auth.callers]`): a configuration with callers is a network configuration by intent, so an unset or too-short token is an error with the same message `serve` gives, exit 1. A file without callers and with a loopback bind resolves nothing | A caller whose `token_env` is unset: `check` exits 1 with the `serve` message; a 5-byte token likewise; a good token: exit 0; no callers: unchanged | `configuration.md` and `deployment.md`: what `check` verifies |
| 3 | **DAT-2. Print every section.** After the existing lines, one compact line per section that `check` prints nothing of today: `service.http` (bind, number of callers (never ids' tokens), `legacy_revisions`, LIM-H1 to H6 values), `fetch` (ports, max_bytes, max_chars, robots_txt, follow_cross_host_redirects), `egress` (counts of allow and deny rules, `allow_http`, strict mode), `privacy` (`secret_detection`), `logging` (level, `log_queries`), `cache` (enabled, entries), `scraping` (enabled), `resilience`. Never a secret, a token or a rule's text | A test per line: the effective value appears; a token and an API key value never appear in the output | `configuration.md`: the output, with a sample |
| 4 | **D-6 warnings.** Each is one stderr line `rypsok: warning: <key>: <what happens>; <the safer setting>`, exit stays 0: an `egress.allow` rule wider than /16 (IPv4) or /32 (IPv6) or `0.0.0.0/0`/`::/0`; `privacy.secret_detection = "off"`; a plaintext listener (any bind) with callers off loopback; `logging.log_queries`; `fetch.robots_txt = false`; `egress.allow_http`; `fetch.follow_cross_host_redirects`; a scraping engine enabled (named as scraping); `cache.enabled` with no storable engine; the derived worst-case memory above 256 MiB (item 5). **No warning for `legacy_revisions`.** The shipped example configurations under `examples/configs/` must produce **no warning** except the ones their README says they provoke (a test) | A table test: one configuration per warning, asserting the line; the default configuration and every example: no unexpected warning | `configuration.md` lists each warning and its cure |
| 5 | **D-20, D-21 derived memory.** `check` prints `memory: worst case about N MiB (…)` computed from the effective settings: `13 MiB base + fetches_in_flight × max_bytes + parses_in_flight × (4.5 × max_bytes + 13 MiB) + max_sessions × 60 KiB (+ the cache bound LIM-K3 × LIM-K4 when the cache is enabled)`. The constants come from audit 020e (idle base 11 MiB; one parse = 34 MiB at 5 MiB and 99 MiB at 20 MiB; 58.5 KiB per session; rounded up); define them once in `limits.rs` with a comment naming the measurement, and add them to the limits table as a small "Measured costs" block so `check-limits` compares them. Warn above 256 MiB (the documented unit's `MemoryMax`) and say the figure is an estimate measured on the musl release build | Unit tests of the formula (the defaults give about 100 MiB, near the measured 90 to 98; `max_bytes` 10 MiB with the defaults gives under 256; `parsing_in_flight` 16 gives a warning); the printed line for the default config | `deployment.md` (the formula and its source); `limits-and-defaults.md` |
| 6 | **DAT-11. Type errors name the key.** A value of the wrong type (a string for a number, a negative number, a float for an integer, an overflow, a bad boolean, an invalid duplicate key, a malformed language tag, a TOML syntax error) is reported as `<key path>: expected <an integer from 1 to 20 | text | true or false | a list of …>, found <what>` in user words, never `u8`, `u32`, `i128` or `at byte N` alone (give `line N, column M` as well). The key path can come from walking the parsed document to the failing span (the `toml` crate's spanned/`DeTable` API) or from a second typed pass; **no new dependency unless you show the cost** (the lockfile diff) and the architect agrees | A table test of the 31 cases of audit 020c area 3 (rebuild them: wrong type, negative, huge, empty region, duplicate engine table, unknown engine type, secret literal, CRLF, BOM, bad language, `"60"`, `2500.5`, `ports = [0, 70000]`, `"yes"`, not TOML ...): each message names the key path and no Rust type name | `troubleshooting.md` |
| 7 | **DAT-12. A range that depends on another key says so.** Where a key's valid range depends on another (for example `search.engine_timeout_ms` at most `deadline_ms − reserve_ms`; `fetch.max_chars` at most `max_chars_ceiling`; `search.max_results` at most `max_results_ceiling`), the message names the other key and its current value: `search.engine_timeout_ms: must be between 100 and 300 (search.deadline_ms 500 minus search.reserve_ms 200)`. Find every such pair by reading `validate` | A test per dependent pair, including the audit's seven cases | `configuration.md` ranges say "(depends on …)" |
| 8 | **DAT-14.** A deny/allow rule that ends in a dot gets: `egress.deny[0]: a host pattern must not end with a dot (write blocked.invalid, not blocked.invalid.)` | The case | — |
| 9 | **Changelog.** Changed: the two ceilings; `check` output and warnings; messages. Fixed: `check` accepting a configuration `serve` refuses | — | `CHANGELOG.md` |

## Explicit non-change scope

No new configuration key; no change of any default; `check` stays stderr-only
with exit 0/1/2 (RFC 016 amendment); no warning that fires on the default
configuration; the log lines `serve` writes.

## Prohibited shortcuts

Printing a secret. A formula constant that is not tied to a measurement. A
warning on `legacy_revisions`. Messages with Rust type names.

## Required evidence and review request

The gates; failing-first output; sample `check` output for the default,
the network example and a deliberately risky configuration (copied to
evidence); the review request at
`.git-exclude/review-requests/020l-check-and-configuration/README.md`.
