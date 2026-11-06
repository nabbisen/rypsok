# Troubleshooting

Start with `rypsok check --config <file>`: it validates the file, prints
the effective settings and each engine's state, and names the key of
every problem. Then look at the logs: standard error, one JSON object per
line, level `info` by default (`[logging] level`). Nothing rypsok says
ever goes to standard output except protocol messages.

## At startup

| Symptom | Cause | Fix |
|---|---|---|
| `check` exits 1 and names a key | The value is outside its range or of the wrong form; the message says which | Change the named key; the [configuration page](./reference/configuration.md) lists every range |
| `search.max_results: expected an integer from 1 to 20, found text (line 3, column 15)` | A value of the wrong kind: text for a number, a decimal for an integer, a negative or too large number. The message gives the key, what it holds, what was found (a text is never repeated) and the place | Write the value as the page says (`true` and `false` without quotes, numbers without quotes) |
| `TOML syntax error: ... (line 1, column 6)` | The file is not valid TOML at that place | Fix the place named; the same line and column are in the file as you see it |
| `engines.w: defined more than once (line 5, column 10)` | A table or a key appears twice | Remove or merge one |
| `search.engine_timeout_ms: must be between 100 and 300 (search.deadline_ms 500 minus search.reserve_ms 200)` | The range of this key depends on other keys; the message names them with their values | Change this key, or the one it names |
| `egress.deny[0]: a host pattern must not end with a dot` | A rule written as `blocked.invalid.` | Write it without the dot: `blocked.invalid` |
| `engines.<id>.api_key: give the secret by api_key_env or api_key_file` | The key was written into the file | Move it to an environment variable or a file and name it |
| `engines.<id>.type: no such driver` | A driver name that does not exist | One of `wikipedia`, `brave`, `duckduckgo` |
| `... scrapes a human-facing interface; it runs only with [scraping] enabled = true ...` | A scraping engine without the switch | Read [Tutorial 5](./tutorials/05-scraping.md), then decide |
| `service.http.bind: a non-loopback bind needs [service.http.auth] callers` | A bind off loopback without tokens | Add callers, or bind on loopback |
| `service.http.bind: a bind on an unspecified address (0.0.0.0, ::) needs service.http.allowed_hosts` | No `Host` name a client would send | List the names the proxy forwards |
| An engine line ends with `inactive: environment variable ... is unset or empty` | The key is not in the environment of the process that runs rypsok | Export it where rypsok starts (a service has its own environment) |
| Exit code 2 | A usage error on the command line | `rypsok --help` |

## During searches

| Symptom | Cause | Fix |
|---|---|---|
| `unavailable` with reason `no_engine` | No enabled, active engine serves the category asked | Check `categories` of the engines and the engines' state in `check` |
| Every search is `partial` with one engine `auth_failure` | The provider refuses the key | Check the key and the subscription. The key is read at startup; a corrected key takes effect at the next restart, and the engine keeps being asked meanwhile, so no cooldown delays it |
| `partial` with `rate_limited` | The provider's or the engine's rate is used up; the notice may carry when to retry | Wait; lower `requests` by asking for fewer engines, or raise the plan |
| `partial` with `timeout` for one engine, often | The provider is slow for the deadline configured | Raise `[search] engine_timeout_ms` within `deadline_ms`, or accept |
| `engine_status` shows an engine `open` | It failed several times in a row and rests; after the cooldown one call probes it | Nothing to do; fix the cause of the failures if there is one (key, network, provider) |
| `denied` with reason `secret_in_query` | The query carries what looks like a credential, at the position given | Remove it; or set `[privacy] secret_detection` to `mask` or `warn` if your use needs it |
| `empty` for a query that should match | Engines search literally; a category mismatch asks the wrong engines | Fewer words; the right category |
| Results arrive slowly | The deadline is the ceiling, not the usual case; one slow engine is waited for up to `engine_timeout_ms` | Lower the timeouts or the number of engines per request |

## Reading pages

| Symptom | Cause | Fix |
|---|---|---|
| `denied` with reason `egress` | The URL names a loopback, private, link-local or metadata address literally, or a port that is not listed, or a host name that never names a public site (`localhost`, a single label, `*.local`) | Nothing for a private address: it is refused by design. For another port, add it to `[fetch] ports`; for a host you trust, an `[egress] allow` rule |
| `unavailable` with reason `network` | The site could not be reached, for whatever reason: no such name, a refused or dropped connection, a name that resolves only to addresses the policy refuses | Retry later or choose another page. The answer is the same whatever the reason, so that it tells nothing about names inside your network; the operator's log says whether the policy refused it (`refused`, with the denial's name and no host) |
| `denied` with reason `robots_txt` | The site's robots.txt disallows the page for `rypsok` or `*`, or the page a redirect led to | Respect it; `[fetch] robots_txt = false` turns the check off, at your risk |
| `denied` with reason `strict_mode` | Strict mode is on and the URL was not returned by a search in this process | Fetch what a search returned, or list the host in `[egress] allow` |
| `denied` with reason `scheme` | Either the URL is not `http` or `https` at all (`ftp://…`), or it is a plain `http` URL, the operator did not allow plain HTTP, and the retry over `https` failed; the `message` says which | Use an `https` URL. rypsok retries `http` over `https` first; set `[egress] allow_http = true` only if you must |
| `redirected` | The page moved to another host | Call again with `detail.location`, or set `[fetch] follow_cross_host_redirects = true` |
| `unavailable` with `http_status` | The site answered 4xx or 5xx; the status is in `detail.status`, the body is never returned | For 408, 425, 429 and 5xx: later (the message ends "retry later"; wait `retry_after_ms` when present). For another 4xx (404, 403, 410): the site refused the request or has no such page, and the same URL will not help; use another page |
| `unsupported_content` | Not HTML, XHTML, plain text or Markdown; `detail.media_type` says what it was | Another page |
| `too_large` | More bytes than `[fetch] max_bytes` | Raise the limit within its range, or another page |
| `content` is short or misses the part you wanted | The extractor chose another block as the main content, or the page builds its content with scripts, which rypsok does not run | `max_chars` higher does not help; another page or the site's text version |

## Over HTTP

| Status | Cause | Fix |
|---|---|---|
| 400 | A protocol header (`MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name`) is missing (answered by the SDK; counted, not logged per request) | Send the headers ([Tutorial 4](./tutorials/04-network-mode.md)) |
| 401 | The bearer token is missing or wrong (the gate) | The token of a configured caller, at least 32 bytes |
| 403 | The `Host` header is not in `allowed_hosts`, or the request carried an `Origin` that is not allowed (answered by the SDK; logged as `sdk_forbidden`) | List the host in `allowed_hosts`. Browsers never call rypsok; if yours must, list the origin in `allowed_origins` |
| 404 | A path other than `/mcp` (the gate) | `/mcp` |
| 406 | `Accept` does not list both `application/json` and `text/event-stream`; the protocol requires a client to list both (answered by the SDK; counted, not logged per request) | Send `Accept: application/json, text/event-stream` |
| 408 | A request was still running after `request_timeout_s` (the gate) | A slow client or proxy: raise `request_timeout_s`, or fix the link |
| 413 | The body is above `max_request_bytes` (the gate, or the SDK for a streamed body) | A tool call is small; check what the client sends |
| 431 | The request line and headers together are above 16 KiB (LIM-H7; answered by hyper before the gate, so neither logged nor counted; over HTTP/2 the connection is closed) | Send fewer or shorter headers; a token is at most a few hundred bytes |
| 503 with `Retry-After: 30` | The handshake-based revisions hold `max_sessions` live sessions already (the gate); the 30 s is a hint, sessions expire after 300 s without traffic | Retry later, use the 2026-07-28 revision (it keeps no session), or raise `max_sessions` |
| JSON-RPC error `-32022` on `initialize` | `legacy_revisions = false` and the client uses the handshake-based revisions | Use the current revision, or set `legacy_revisions = true` |

## Logs

Each line has an `event`. The ones you will look for: `call` (one per
tool call with its outcome and latency), `engine_failed` (engine and
class), `refused` (what was refused and why), `http_refused`,
`circuit_opened` and `circuit_closed`, `counters` (every minute). The
gate logs `http_refused` with a `reason` for `too_many_connections`,
`body_too_large`, `unauthorized`, `request_timeout`, `session_limit` and
`sdk_forbidden` (a foreign `Host` or `Origin`), with the address family and
never the address or a header value. A missing protocol header (400) and a
wrong `Accept` (406) are not logged per request; every refusal of the gate
and of the SDK behind it is counted, per reason, in the `counters` event
(`refusals`, under `http`). A connection that sent nothing, or only the first
bytes of a request or of the HTTP/2 preface, and was closed after the idle
time is counted there as `idle_connection` and not logged.
The SDK writes its own warning for a refused
`Host` or `Origin`, and one error line for each refused handshake. The
query text never appears in a log unless `[logging] log_queries = true`
at level `debug`, which you turn on only to diagnose and turn off again.
