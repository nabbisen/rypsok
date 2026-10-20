# Configuration

rypsok reads one TOML file. It is located by `--config <path>`, then by
the environment variable `RYPSOK_CONFIG`, then not at all: with no file,
every setting takes the default below and the keyless engines are used.
There is no other source of settings and no hot reload; a change takes
effect at the next start.

`rypsok check` validates a file and prints the effective settings on
standard error. Every problem names its key. Unknown keys and tables are
refused.

Every default is a value of the [limits table](../maintainers/limits-and-defaults.md);
the `LIM-` identifier in the right column names the row.

## Top level

| Key | Default | Range | Meaning |
|---|---|---|---|
| `config_version` | required | `1` | Schema version; this release reads 1 |

## `[service]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `transport` | `"stdio"` | `stdio`, `http` | `http` is refused before release 0.4.0 |
| `structured_content` | `true` | boolean | Return results as `structuredContent` plus text, or as text alone |
| `user_agent_contact` | `""` | a URL or an address | Appended to the User-Agent so that providers can reach the operator |

### `[service.http]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `bind` | `"127.0.0.1:8737"` | address and port | Listener of the HTTP transport (release 0.4.0) |

## `[search]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `languages` | `["en"]` | language tags | Offered to callers; the first is the default |
| `region` | `"US"` | two uppercase letters | Passed to engines that support it |
| `safe_search` | `"moderate"` | `off`, `moderate`, `strict` | Passed to engines that support it |
| `max_results` | `5` | 1 to `max_results_ceiling` | Default of the `max_results` parameter (LIM-S2) |
| `max_results_ceiling` | `20` | 1–20 | Upper bound of `max_results` (LIM-S2) |
| `query_max_chars` | `400` | 1–600 | Length of `query` (LIM-S1) |
| `deadline_ms` | `2500` | 500–10000 | Overall deadline of a search (LIM-S4) |
| `engine_timeout_ms` | `2000` | 100 to `deadline_ms` | Timeout of one engine request (LIM-S5); `engine_timeout_ms + reserve_ms` must not exceed `deadline_ms` |
| `reserve_ms` | `200` | 50–1000 | Reserve for post-processing (LIM-S6) |
| `engines_per_request` | `8` | 1–16 | Engines queried per request (LIM-S8) |
| `results_per_engine` | `10` | 5–20 | Results requested from each engine (LIM-S3) |
| `snippet_max_chars` | `300` | 50–1000 | Length of a snippet (LIM-S9) |
| `title_max_chars` | `150` | 50–300 | Length of a title (LIM-S10) |
| `drop_flagged_results` | `false` | boolean | Drop results with instruction-like content instead of only noticing them |
| `engine_attribution` | `false` | boolean | Always carry `results[].engines` |

## `[fetch]` (used from release 0.3.0; accepted now)

| Key | Default | Range | Meaning |
|---|---|---|---|
| `max_chars` | `3000` | 500 to `max_chars_ceiling` | Default of `max_chars` (LIM-F1) |
| `max_chars_ceiling` | `20000` | operator | Ceiling of `max_chars` (LIM-F2) |
| `max_bytes` | `5242880` | 65536–20971520 | Response bytes after decompression (LIM-F3) |
| `deadline_ms` | `10000` | 1000–30000 | Overall deadline of a fetch (LIM-F4) |
| `connect_timeout_ms` | `3000` | 500–10000 | Connect timeout (LIM-F6) |
| `max_redirects` | `5` | 0–10 | Redirects followed on the same host (LIM-F7) |
| `ports` | `[80, 443]` | ports | Ports a fetch may use (LIM-F8); another port needs an `egress.allow` rule |
| `robots_txt` | `true` | boolean | Honor robots.txt |
| `follow_cross_host_redirects` | `false` | boolean | Follow a redirect to another host instead of returning `redirected` |
| `strict_mode` | `false` | boolean | Fetch only hosts the operator listed |

## `[egress]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `allow_http` | `false` | boolean | Allow plain HTTP fetches |
| `allow` | `[]` | CIDRs or host names | Admitted explicitly |
| `deny` | `[]` | CIDRs or host names | Refused in addition to the built-in rules |
| `proxy` | `""` | URL | Proxy for engine traffic only; empty means none |

## `[limits]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `outbound_in_flight` | `32` | 1–1024 | Outbound requests in flight, whole process (LIM-C1) |
| `per_engine_in_flight` | `2` | 1–32 | Requests in flight per engine (LIM-C2) |
| `fetches_in_flight` | `4` | 1–256 | Fetches in flight, whole process (LIM-C3) |
| `parsing_in_flight` | `2` | 1–64 | Parsing tasks in flight (LIM-C5) |
| `web_search_per_minute` | `60` | 1–100000 | `web_search` calls accepted (LIM-C6) |
| `web_fetch_per_minute` | `30` | 1–100000 | `web_fetch` calls accepted (LIM-C7) |
| `engine_status_per_minute` | `30` | 1–100000 | `engine_status` calls accepted (LIM-C8) |

## `[resilience]` (used from release 0.2.0; accepted now)

| Key | Default | Range | Meaning |
|---|---|---|---|
| `failures_to_open` | `3` | 1–10 | Consecutive failures that open an engine's circuit (LIM-B1) |
| `first_cooldown_s` | `60` | 10–600 | First cooldown (LIM-B2) |
| `max_cooldown_s` | `900` | 10–3600 | Longest cooldown (LIM-B3); not below `first_cooldown_s` |

## `[privacy]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `secret_detection` | `"refuse"` | `off`, `warn`, `mask`, `refuse` | What happens when a query or URL looks like it carries a credential (LIM-P1) |

## `[logging]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `level` | `"info"` | `error`, `warn`, `info`, `debug` | Log level (LIM-P2) |
| `log_queries` | `false` | boolean | Debug mode: query text in `debug` events; `serve` warns at start (LIM-P3) |

## `[status]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `enabled` | `true` | boolean | Offer `engine_status`; when false it is absent from the tool list |

## `[engines.<id>]`

One table per engine. The identifier is lowercase letters, digits and
underscores, starting with a letter. With no `[engines.*]` table at all,
two engines are configured: `wikipedia` (keyless) and `brave` (keyed by
`BRAVE_API_KEY`).

| Key | Default | Range | Meaning |
|---|---|---|---|
| `type` | required | `wikipedia`, `brave` | The driver |
| `enabled` | `true` | boolean | Whether the engine is used |
| `categories` | required | `general`, `news`, `science`, `code` | The categories the engine serves; at least one |
| `preference` | `100` | integer | Lower first when results tie |
| `api_key_env` | — | variable name | Where the credential is read from (keyed drivers) |
| `api_key_file` | — | path | Alternative to `api_key_env`; the file's content, trimmed |
| `endpoint` | `"web"` | `web`, `news` | Brave only: which endpoint the engine queries |

A credential is never given inline: `api_key = "…"` is refused at
startup. When the variable is unset or the file unreadable, the engine is
inactive and the log says so; the process still starts.

## Example

```toml
config_version = 1

[search]
languages = ["en", "ja"]
max_results = 8

[engines.wikipedia]
type = "wikipedia"
categories = ["general", "science"]
preference = 10

[engines.brave]
type = "brave"
categories = ["general"]
preference = 20
api_key_env = "BRAVE_API_KEY"

[engines.brave_news]
type = "brave"
categories = ["news"]
endpoint = "news"
api_key_env = "BRAVE_API_KEY"
```
