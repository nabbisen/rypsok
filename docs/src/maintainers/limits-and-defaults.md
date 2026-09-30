# Limits and Defaults

**Status:** Baseline 1.0 (2026-09-30). Values are provisional until the M1 and M3 measurements, as confirmed by the owner on 2026-09-30.
**Governs:** every number that the [requirements](./requirements.md) and the
[external design](./external-design.md) refer to by a `LIM-` identifier.

Every value here is a starting point. It is confirmed or corrected by
measurement in milestones M1 and M3 (see `ROADMAP.md`). The requirements do
not repeat the values; they cite the identifier, so that a value changes in
one place only.

"Range" is what an operator may configure. Where a range is "operator", the
operator may choose any value; where it is "fixed", no configuration changes
it.

## Units

| ID | Quantity | Unit |
|---|---|---|
| LIM-U1 | Text budgets | Unicode scalar values, cut only at a character boundary |
| LIM-U2 | Byte budgets | Bytes after decompression |
| LIM-U3 | Time | Milliseconds, measured on a monotonic clock |
| LIM-U4 | Timestamps in output | RFC 3339, UTC |

## Search

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-S1 | Length of `query` | 400 characters | 1–600 | Brave accepts 600 characters and 75 words; an engine with a lower limit is skipped for a longer query |
| LIM-S2 | `max_results` | 5 | 1–20 | Study example; Brave returns at most 20 |
| LIM-S3 | Results requested from each engine | 10 | 5–20 | Overlap for ranking |
| LIM-S4 | Overall deadline of a search | 2500 ms | 500–10000 | Study proposes 1500 ms; to be measured (R-10) |
| LIM-S5 | Timeout of one engine request | 2000 ms | ≤ LIM-S4 − LIM-S6 | C-14 |
| LIM-S6 | Reserve for post-processing | 200 ms | 50–1000 | Architecture overview §4.6 |
| LIM-S7 | Maximum duration of a `web_search` call | LIM-S4 + 300 ms | derived | X-04 |
| LIM-S8 | Engines queried per request | 8 | 1–16 | Bounded fan-out |
| LIM-S9 | Length of a snippet | 300 characters | 50–1000 | Context budget |
| LIM-S10 | Length of a title | 150 characters | 50–300 | Context budget |
| LIM-S11 | Window for the typical latency of an engine | last 20 completed requests | 5–100 | Median over the window |
| LIM-S12 | Connect timeout of an engine request | 1000 ms | 200–5000 | — |
| LIM-S13 | Maximum duration of an `engine_status` call | 200 ms | derived | X-04 |

## Fetch

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-F1 | `max_chars` | 3000 | 500 to LIM-F2 | Draft external design §4.2 and §15; mimlys study; X-06 |
| LIM-F2 | Ceiling of `max_chars` | 20000 | operator | M-02 |
| LIM-F3 | Response bytes after decompression | 5 MiB | 64 KiB–20 MiB | Study |
| LIM-F4 | Overall deadline of a fetch | 10000 ms | 1000–30000 | — |
| LIM-F5 | Maximum duration of a `web_fetch` call | LIM-F4 + 1000 ms | derived | X-04, R-04 |
| LIM-F6 | Connect timeout | 3000 ms | 500–10000 | — |
| LIM-F7 | Redirects followed on the same host | 5 | 0–10 | Draft external design §15 |
| LIM-F8 | Ports | 80, 443 | operator | M-07 |
| LIM-F9 | Length of `url` | 2048 characters | 256–8192 | — |
| LIM-F10 | HTML nodes parsed | 100000 | operator | R-04 |
| LIM-F11 | HTML nesting depth | 256 | operator | R-04 |
| LIM-F12 | robots.txt kept in memory | 24 hours | ≤ 24 hours | RFC 9309 §2.4 |
| LIM-F13 | Size of a robots.txt read | 512 KiB | operator | Parser bound |

## Concurrency and rates

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-C1 | Outbound requests in flight, whole process | 32 | operator | REQ-SEC-012 |
| LIM-C2 | Requests in flight per engine | 2 | operator, lower where the provider demands it | Provider terms |
| LIM-C3 | Fetches in flight, whole process | 4 | operator | REQ-SEC-012 |
| LIM-C4 | Fetches in flight per host | 1 | fixed | Politeness |
| LIM-C5 | Parsing tasks in flight | 2 | operator | R-04 |
| LIM-C6 | `web_search` calls accepted | 60 per minute | operator | MCP: servers must rate limit tool invocations |
| LIM-C7 | `web_fetch` calls accepted | 30 per minute | operator | Same |
| LIM-C8 | `engine_status` calls accepted | 30 per minute | operator | Same |
| LIM-C9 | Requests to one engine | per engine, from its admission record | operator, never above the provider's limit | REQ-CMP-002 |

## Circuit breaker

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-B1 | Consecutive failures that open the circuit | 3 | 1–10 | — |
| LIM-B2 | First cooldown | 60 s | 10–600 s | — |
| LIM-B3 | Cooldown growth | doubling, up to 900 s | operator | — |
| LIM-B4 | `Retry-After` from a provider | honored, up to LIM-B3's maximum | fixed | M-15 |
| LIM-B5 | Probe after cooldown | one real request | fixed | D-33 |

## Interface size and shape

| ID | Parameter | Budget | Range | Basis |
|---|---|---:|---|---|
| LIM-I1 | Names, descriptions and input schemas of all tools, plus the server instructions, together | 2000 characters | fixed | R-15, X-06 |
| LIM-I2 | Parameters of one tool | 5 at most | fixed | Clarity rule 8 |
| LIM-I3 | Nesting of a tool's input schema | none: flat object of primitives and enumerations | fixed | X-06 |
| LIM-I4 | Length of a notice or error message | 200 characters | fixed | Context budget |

## Operation

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-O1 | Time to exit after end of file on stdin or `SIGTERM` | 3000 ms | 500–10000 | REQ-OPS-002 |
| LIM-O2 | Time until a cancelled call has aborted its outbound requests | 500 ms | 100–2000 | REQ-MCP-007 |

## Privacy and logging

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-P1 | Secret detection | on, refuse | off, warn, mask, refuse | D-10 |
| LIM-P2 | Log level | info | error, warn, info, debug | D-19 |
| LIM-P3 | Query text in logs | never | only in debug mode, which warns at start | D-19 |

## Cache (not before M5)

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-K1 | Cache | off | on, off | D-28 |
| LIM-K2 | Lifetime of an entry | 300 s | 10–3600 s | Draft external design §15 |
| LIM-K3 | Entries | 1000 | operator | Bounded memory |
