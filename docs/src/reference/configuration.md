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

The transport is not a setting: `rypsok serve --transport stdio|http`
chooses it on the command line, where the launching host can see it.

| Key | Default | Range | Meaning |
|---|---|---|---|
| `structured_content` | `true` | boolean | Return results as `structuredContent` plus text, or as text alone |
| `user_agent_contact` | `""` | a URL or an address | Appended to the User-Agent so that providers can reach the operator |

### `[service.http]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `bind` | `"127.0.0.1:8737"` | address and port | Listener of `serve --transport http`; off loopback it needs callers |
| `allowed_hosts` | `[]` | host names, `host:port` | `Host` values accepted; empty means loopback names and the bind host; required for a bind on `0.0.0.0` or `::` |
| `allowed_origins` | `[]` | origins with a scheme, `:*` for any port, `null` | `Origin` values accepted; empty means every present `Origin` is refused |
| `legacy_revisions` | `true` | boolean | Serve the handshake-based revisions with sessions in memory; `false` refuses them (`initialize` gets error `-32022`); the 2026-07-28 revision is always stateless |
| `max_connections` | `64` | 1–4096 | Connections accepted at once (LIM-H1) |
| `max_request_bytes` | `65536` | 4096–1048576 | Size of a request body (LIM-H2); the same 413 whether declared or streamed |
| `idle_timeout_s` | `60` | 5–600 | Idle connection timeout (LIM-H3): HTTP/1 closes after it; HTTP/2 pings after it and closes when the ping goes unanswered |
| `max_streams` | `16` | 1–256 | Streams one HTTP/2 connection may open at once (LIM-H4) |
| `request_timeout_s` | `30` | 3–600, at least `fetch.deadline_ms` + 2 s | A request still running after this, body and handler included, is answered 408 (LIM-H5); a slow client cannot hold a connection |

rypsok does not terminate TLS: off loopback, put a reverse proxy or a
tunnel in front and list its host in `allowed_hosts`. Browsers never call
rypsok, so `allowed_origins` stays empty unless a browser-based host needs
it.

### `[service.http.auth]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `callers` | `[]` | array of tables | The callers and their tokens; required off loopback, optional on loopback (every request is then the caller `local`) |

Each `[[service.http.auth.callers]]` table has `id` (lowercase letters,
digits and underscores, starting with a letter, unique) and exactly one of
`token_env` (the variable holding the token) or `token_file` (the file
whose trimmed content is the token). A token is at least 32 bytes and is
never written inline. A caller presents it as `Authorization: Bearer
<token>`; rate limits apply per caller.

```toml
[[service.http.auth.callers]]
id = "mimlys"
token_env = "RYPSOK_TOKEN_MIMLYS"
```

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

## `[fetch]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `max_chars` | `3000` | 500 to `max_chars_ceiling` | Default of `max_chars` (LIM-F1) |
| `max_chars_ceiling` | `20000` | 500–100000 | Ceiling of `max_chars` (LIM-F2) |
| `max_bytes` | `5242880` | 65536–20971520 | Response bytes after decompression (LIM-F3) |
| `deadline_ms` | `10000` | 1000–30000 | Overall deadline of a fetch (LIM-F4) |
| `connect_timeout_ms` | `3000` | 500–10000 | Connect timeout (LIM-F6) |
| `max_redirects` | `5` | 0–10 | Redirects followed on the same host (LIM-F7) |
| `ports` | `[80, 443]` | ports | Ports a fetch may use (LIM-F8); another port needs an `egress.allow` rule |
| `robots_txt` | `true` | boolean | Honor robots.txt |
| `follow_cross_host_redirects` | `false` | boolean | Follow a redirect to another host instead of returning `redirected` |
| `strict_mode` | `false` | boolean | `web_fetch` accepts only URLs that `web_search` returned in this process and URLs matched by an `egress.allow` rule; anything else is `denied` with reason `strict_mode` |

## `[egress]`

The agent-directed profile (fetches and robots.txt reads) refuses every
destination that is not a globally routable unicast address: loopback,
private, link-local, unique-local, multicast, carrier-grade NAT,
documentation, benchmarking, NAT64 and cloud metadata ranges, and names
such as `localhost`, single labels, `*.local`, `*.internal` and `*.arpa`.
The judgment is made on every address that is connected to, at connect
time. Rules are applied in the order deny, allow, built-in: a `deny` rule
refuses even what an `allow` rule admits, and an `allow` rule admits a
private destination at the operator's own responsibility. A rule is a
network in CIDR notation (`10.0.0.0/8`, `fd00::/8`), an address, a host
name (`wiki.corp.example`) or a subdomain pattern (`*.corp.example`, which
does not match `corp.example` itself); never a scheme or a port.

| Key | Default | Range | Meaning |
|---|---|---|---|
| `allow_http` | `false` | boolean | Allow plain HTTP fetches; otherwise an `http` URL is retried as `https` |
| `allow` | `[]` | rules | Admitted explicitly, also under `fetch.strict_mode` |
| `deny` | `[]` | rules | Refused in addition to the built-in refusals |
| `proxy` | `""` | URL | Proxy for engine traffic only; fetches never use a proxy, because rypsok cannot verify that it applies the same profile; environment proxy variables are ignored everywhere |

## `[limits]`

| Key | Default | Range | Meaning |
|---|---|---|---|
| `outbound_in_flight` | `32` | 1–256 | Outbound requests in flight, whole process (LIM-C1) |
| `per_engine_in_flight` | `2` | 1–8 | Requests in flight per engine (LIM-C2) |
| `fetches_in_flight` | `4` | 1–64 | Fetches in flight, whole process (LIM-C3) |
| `parsing_in_flight` | `2` | 1–16 | Parsing tasks in flight (LIM-C5) |
| `web_search_per_minute` | `60` | 1–6000 | `web_search` calls accepted (LIM-C6) |
| `web_fetch_per_minute` | `30` | 1–6000 | `web_fetch` calls accepted (LIM-C7) |
| `engine_status_per_minute` | `30` | 1–6000 | `engine_status` calls accepted (LIM-C8) |
| `strict_mode_urls` | `10000` | 100–1000000 | URLs remembered for `fetch.strict_mode` (LIM-F14) |
| `robots_cache_entries` | `1000` | 10–100000 | robots.txt entries kept in memory (LIM-F15) |

## `[resilience]`

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

## `[cache]`

The result cache: one entry per engine answer, per caller, in memory
only. Off by default. Enabling it retains the queries of the last `ttl_s`
seconds in memory as cache keys; nothing is written to disk. Only
`web_search` answers of engines whose admission record allows storage
(Wikipedia; not Brave) are cached, and a response built from a cached
answer carries the notice `from_cache`.

| Key | Default | Range | Meaning |
|---|---|---|---|
| `enabled` | `false` | boolean | Whether answers are cached (LIM-K1) |
| `ttl_s` | `300` | 10–3600 | Lifetime of an entry (LIM-K2) |
| `max_entries` | `1000` | 1–100000 | Entries held at most; the least recently used goes first (LIM-K3) |
| `max_entry_bytes` | `32768` | 1024–1048576 | An answer above this size is not stored (LIM-K4); memory is at most `max_entries` × `max_entry_bytes` |

## `[scraping]`

Scraping drivers read a provider's human-facing page because no interface
for programs exists. They are off by default; a table whose driver scrapes
is refused at startup until this switch is on. Turning it on is your
acceptance of the provider's terms: rypsok sends its standard User-Agent
and never imitates a browser, so a provider that refuses it answers with
a challenge and the engine reports `blocked`. The endpoint's robots.txt is
recorded in the admission record, not consulted at run time.

| Key | Default | Range | Meaning |
|---|---|---|---|
| `enabled` | `false` | boolean | Whether scraping drivers may run (LIM-G1) |

## `[engines.<id>]`

One table per engine. The identifier is lowercase letters, digits and
underscores, starting with a letter. With no `[engines.*]` table at all,
two engines are configured: `wikipedia` (keyless) and `brave` (keyed by
`BRAVE_API_KEY`).

| Key | Default | Range | Meaning |
|---|---|---|---|
| `type` | required | `wikipedia`, `brave`, `duckduckgo` | The driver; `duckduckgo` scrapes and needs `[scraping] enabled = true` |
| `enabled` | `true` | boolean | Whether the engine is used |
| `categories` | required | `general`, `news`, `science`, `code` | The categories the engine serves; at least one |
| `preference` | `100` | integer | Lower first when results tie |
| `api_key_env` | — | variable name | Where the credential is read from (keyed drivers) |
| `api_key_file` | — | path | Alternative to `api_key_env`; the file's content, trimmed |
| `endpoint` | `"web"` | `web`, `news` | Brave only: which endpoint the engine queries |
| `base_url` | — | an absolute URL | Wikipedia only: a private MediaWiki mirror; plain HTTP is admitted on a loopback host only |

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
