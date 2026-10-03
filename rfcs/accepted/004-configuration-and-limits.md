# RFC 004 — Configuration schema and limits

**Status.** Accepted (2026-09-30) — implementer may start after RFC 001.
**Tracks.** Milestone M1; requirements REQ-CFG-001, -003, -004, -005,
REQ-MAIN-003, REQ-OPS-001, -002, -003, REQ-SEARCH-008, REQ-ENG-003;
external design §13; `docs/src/maintainers/limits-and-defaults.md`.
**Touches.** `rypsok-core::config`, `rypsok-core::limits`, `rypsok::cli`.
**Handoff.** [`../handoffs/004-configuration-and-limits/README.md`](../handoffs/004-configuration-and-limits/README.md)

## Summary

One TOML file, one typed `Config`, one typed `Limits`. Every limit of the
limits table is a field with the table's default; a configuration key
overrides it within the table's range. Validation runs at startup and stops
the process with the offending key. Secrets are references, never values.

## Design

### Precedence

`--config <path>` → `RYPSOK_CONFIG` → no file. Command-line flags override
nothing else; there is exactly one source of settings besides the two ways
to name it (REQ-CFG-001).

### Schema

Serde structures with `deny_unknown_fields` on every table. `config_version`
is required and must be `1` (REQ-MAIN-003).

```toml
config_version = 1

[service]
transport = "stdio"                   # "stdio" | "http" (http refused before M4)
structured_content = true             # REQ-MCP-011
user_agent_contact = ""               # appended to the User-Agent; a URL or an address (REQ-CMP-004)

[service.http]                        # M4; refused before
bind = "127.0.0.1:8737"

[search]
languages = ["en"]                    # offered to callers; first is the default
region = "US"
safe_search = "moderate"              # off | moderate | strict
max_results = 5                       # LIM-S2 default, within its range
max_results_ceiling = 20              # LIM-S2 upper bound
query_max_chars = 400                 # LIM-S1
deadline_ms = 2500                    # LIM-S4
engine_timeout_ms = 2000              # LIM-S5
reserve_ms = 200                      # LIM-S6
engines_per_request = 8               # LIM-S8
results_per_engine = 10               # LIM-S3
snippet_max_chars = 300               # LIM-S9
title_max_chars = 150                 # LIM-S10
drop_flagged_results = false          # REQ-SEC-003: drop results with instruction-like content
engine_attribution = false            # REQ-CTX-001: results[].engines always present

[fetch]                               # M3; keys accepted from M1, used from M3
max_chars = 3000                      # LIM-F1
max_chars_ceiling = 20000             # LIM-F2
max_bytes = 5242880                   # LIM-F3
deadline_ms = 10000                   # LIM-F4
connect_timeout_ms = 3000             # LIM-F6
max_redirects = 5                     # LIM-F7
ports = [80, 443]                     # LIM-F8
robots_txt = true                     # REQ-CMP-005
follow_cross_host_redirects = false   # REQ-FETCH-006
strict_mode = false                   # REQ-SEC-021

[egress]
allow_http = false                    # REQ-FETCH-010
allow = []                            # CIDRs or host names admitted explicitly
deny = []                             # CIDRs or host names refused in addition
proxy = ""                            # engine traffic only; empty = none (REQ-RES-008)

[limits]
outbound_in_flight = 32               # LIM-C1
per_engine_in_flight = 2              # LIM-C2
fetches_in_flight = 4                 # LIM-C3
parsing_in_flight = 2                 # LIM-C5
web_search_per_minute = 60            # LIM-C6
web_fetch_per_minute = 30             # LIM-C7
engine_status_per_minute = 30         # LIM-C8

[resilience]                          # M2 (RFC 010); keys accepted from M1
failures_to_open = 3                  # LIM-B1
first_cooldown_s = 60                 # LIM-B2
max_cooldown_s = 900                  # LIM-B3

[privacy]
secret_detection = "refuse"           # off | warn | mask | refuse (LIM-P1)

[logging]
level = "info"                        # error | warn | info | debug (LIM-P2)
log_queries = false                   # debug mode of REQ-PRIV-004; warns at start

[status]
enabled = true                        # REQ-STATUS-003

[engines.wikipedia]
type = "wikipedia"
enabled = true
categories = ["general", "science"]
preference = 10                       # lower first (REQ-ENG-007)

[engines.brave]
type = "brave"
enabled = true
categories = ["general"]
preference = 20
api_key_env = "BRAVE_API_KEY"         # or api_key_file = "/run/secrets/brave"
```

Engine tables carry the keys their driver declares (RFC 006); unknown keys
are refused like everywhere else.

### `Limits`

`rypsok-core::limits::Limits` is a plain struct whose fields are the `LIM-`
identifiers with their default values as constants; `Config::limits()`
builds one from the file. The doc comment of every field cites its
identifier so that the table and the code can be compared by a script
(`tools/check-limits`, M1: it extracts every `LIM-` row of the table and
every field default and fails on a mismatch).

### Validation (REQ-CFG-005)

Performed by `Config::validate()`, run by `rypsok check` and before `serve`.
Every failure names the key. Rules:

| Rule | Message |
|---|---|
| `config_version != 1` | `config_version: only 1 is supported` |
| Unknown key or table | `unknown key <path>` |
| Value outside the range of its `LIM-` row | `<key>: must be between <min> and <max>` |
| `engine_timeout_ms + reserve_ms > deadline_ms` | `search.engine_timeout_ms + search.reserve_ms exceeds search.deadline_ms` |
| Engine `type` without a driver | `engines.<id>.type: no such driver` |
| Engine key that the driver does not declare | `engines.<id>.<key>: not a key of driver <type>` |
| Inline secret (`api_key = …`) | `engines.<id>.api_key: give the secret by api_key_env or api_key_file` |
| `api_key_env` unset or `api_key_file` unreadable | The engine is marked inactive with a log line; not a failure (REQ-OPS-003) |
| `service.transport = "http"` before M4 | `service.transport: http is not available in this release` |
| A port in `fetch.ports` not 80 or 443 and not covered by `egress.allow` | `fetch.ports: <port> needs an egress.allow rule` |
| `logging.log_queries = true` | Accepted; `serve` logs a warning at start |

### Secrets (REQ-CFG-004)

A secret is read once at startup from the environment variable or the file,
held in a `Secret<String>` type whose `Debug` prints `[redacted]`
(REQ-SEC-020), and never logged. A file secret must not be world-readable
on Unix; otherwise a warning is logged.

### Command line (`rypsok::cli`)

`clap` derive with subcommands `serve` (`--transport stdio|http`) and
`check`, plus `--config` and `--version`. Exit codes: 0 success, 1
configuration error (message on stderr), 2 usage (REQ-OPS-001). `serve`
installs a `SIGTERM` handler (Unix) and a Ctrl-C handler, and treats end of
file on stdin as shutdown; shutdown aborts calls through the cancellation
token and exits within LIM-O1 (REQ-OPS-002). Nothing is ever written to
stdout by the command line.

## Alternatives considered

- **YAML.** Rejected: the drafts used it, but the maintained YAML crate for
  serde is deprecated, and TOML is the Rust convention (D-22).
- **Limits as configuration only.** Rejected: constants with a script that
  compares them with the table keep the documentation honest.
- **Hot reload.** Deferred: restart applies changes (D-22).

## Open questions

- Whether `preference` should be replaced by list order. Decided here:
  explicit integers, because TOML tables have no reliable order.
