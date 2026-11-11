# Limits and Defaults

**Status:** Baseline 1.0 (2026-09-30). Values were provisional until the M1 and M3 measurements, as confirmed by the owner on 2026-09-30. The measurements were taken and left the values they tested unchanged: at M1 the cold start (729 ms, 2026-10-20) kept LIM-S4 and LIM-S6, and at M3 the fetch memory report (release 0.3.0, 2026-10-27) proposed no change to LIM-F10 and derived the ceiling from LIM-C3, LIM-C5 and LIM-F3. M8 measures again (RFC 020, handoff 020e).
**Governs:** every number that the [requirements](./requirements.md) and the
[external design](./external-design.md) refer to by a `LIM-` identifier.

A bare `D-nn` in this book is a decision of the startup review that fixed the
first requirements. That review is not a page of this book and no page lists its
decisions; the requirements quote what they need. The decisions of the audit
(RFC 020) are written `RFC 020, D-n`.
([Requirements](./requirements.md) says the same in its introduction.)

Every value here is a starting point. It is confirmed or corrected by
measurement in milestones M1 and M3 (see `ROADMAP.md`). The requirements do
not repeat the values; they cite the identifier, so that a value changes in
one place only.

"Range" is what an operator may configure, through the key the
[configuration reference](../reference/configuration.md) names for the row.
Where a range is "operator", the operator may choose any value; where it is
"fixed", no configuration changes it (RFC 020, D-9: a row that is not backed
by a key is "fixed").

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
| LIM-S4 | Overall deadline of a search | 2500 ms | 500–10000 | Study proposes 1500 ms (R-10). Measured: cold start to the first answer 729 ms at M1 and 715 ms at M8; rypsok's own share about 6 ms (RFC 020, 020e) |
| LIM-S5 | Timeout of one engine request | 2000 ms | ≤ LIM-S4 − LIM-S6 | C-14 |
| LIM-S6 | Reserve for post-processing | 200 ms | 50–1000 | Architecture overview §4.6. Measured (RFC 020, 020e): post-processing took 0.64 ms median and 5.11 ms at most, with mock engine answers; larger real answers were not measured. Kept at 200 ms (RFC 020, PRF-5) |
| LIM-S7 | Maximum duration of a `web_search` call | LIM-S4 + 300 ms | derived | X-04 |
| LIM-S8 | Engines queried per request | 8 | 1–16 | Bounded fan-out |
| LIM-S9 | Length of a snippet | 300 characters | 50–1000 | Context budget |
| LIM-S10 | Length of a title | 150 characters | 50–300 | Context budget |
| LIM-S11 | Window for the typical latency of an engine | last 20 completed requests | fixed | Median over the window |
| LIM-S12 | Connect timeout of an engine request (`search.connect_timeout_ms`) | 1000 ms | 200–5000 | — |
| LIM-S13 | Maximum duration of an `engine_status` call | 200 ms | derived | X-04 |

## Fetch

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-F1 | `max_chars` | 3000 | 500 to LIM-F2 | Draft external design §4.2 and §15; mimlys study; X-06 |
| LIM-F2 | Ceiling of `max_chars` | 20000 | 500–100000 | M-02 |
| LIM-F3 | Response bytes after decompression | 5 MiB | 64 KiB–10 MiB | Study; the ceiling is 10 MiB since RFC 020, D-20: four 20 MiB pages need 265 to 282 MiB (audit 020e), more than the 256 MiB of the documented unit |
| LIM-F4 | Overall deadline of a fetch | 10000 ms | 1000–30000 | — |
| LIM-F5 | Maximum duration of a `web_fetch` call | LIM-F4 + 1000 ms | derived | X-04, R-04 |
| LIM-F6 | Connect timeout | 3000 ms | 500–10000 | — |
| LIM-F7 | Redirects followed | 5 | 0–10 | Draft external design §15 |
| LIM-F8 | Ports | 80, 443 | operator | M-07 |
| LIM-F9 | Length of `url` | 2048 characters | fixed | — |
| LIM-F10 | HTML nodes parsed | 100000 | fixed | R-04 |
| LIM-F11 | HTML nesting depth; enforced while parsing, a deeper page is read in part | 256 | fixed | R-04; RFC 020, D-3 |
| LIM-F12 | robots.txt kept in memory | 24 hours | fixed | RFC 9309 §2.4 |
| LIM-F13 | Size of a robots.txt read | 512 KiB | fixed | Parser bound |
| LIM-F14 | URLs remembered for strict mode | 10000 | operator | REQ-SEC-021; bounded memory (RFC 011) |
| LIM-F15 | robots.txt entries kept in memory | 1000 origins | operator | REQ-CMP-005; bounded memory (RFC 011) |

## Concurrency and rates

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-C1 | Outbound requests in flight, whole process | 32 | 1–256 | REQ-SEC-012 |
| LIM-C2 | Requests in flight per engine | 2 | 1–8, lower where the provider demands it | Provider terms |
| LIM-C3 | Fetches in flight, whole process | 4 | 1–64 | REQ-SEC-012 |
| LIM-C4 | Fetches in flight per host | 1 | fixed | Politeness |
| LIM-C5 | Parsing tasks in flight | 2 | 1–16 | R-04 |
| LIM-C6 | `web_search` calls accepted | 60 per minute | 1–6000 per minute | MCP: servers must rate limit tool invocations |
| LIM-C7 | `web_fetch` calls accepted | 30 per minute | 1–6000 per minute | Same |
| LIM-C8 | `engine_status` calls accepted | 30 per minute | 1–6000 per minute | Same |
| LIM-C9 | Requests to one engine | per engine, from its admission record | fixed (the provider's limit) | REQ-CMP-002 |

## HTTP listener (M4)

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-H1 | HTTP connections accepted at once | 64 | 1–4096 | REQ-SEC-018; bounded memory (RFC 013) |
| LIM-H2 | Size of an HTTP request body | 65536 bytes | 4096–1048576 | A tool call is small; THR-11 (RFC 013) |
| LIM-H3 | Idle HTTP connection timeout, including a connection that has sent nothing or only the first bytes of the HTTP/2 preface (HTTP/1: close after; HTTP/2: keep-alive ping after, close when unanswered) | 60 s | 5–600 s | Resource hygiene (RFC 013) |
| LIM-H4 | Streams one HTTP/2 connection may open | 16 | 1–256 | THR-26 (RFC 018) |
| LIM-H5 | Time one HTTP request may take, whatever its state | 30 s | 3–600 s, at least `fetch.deadline_ms` + 2 s | THR-26 (RFC 018) |
| LIM-H6 | Live sessions of the handshake-based revisions over HTTP; each costs about 60 KiB | 256 | 1–2048 | THR-27 (RFC 019); the ceiling is 2048 since RFC 020, D-21: 65536 sessions need 3.7 GiB (audit 020e), 2048 about 120 MiB |
| LIM-H7 | Size of the header block of an HTTP request (HTTP/1: request line and headers; HTTP/2: the header list) | 16384 bytes | fixed | NET-12 (audit 020b); the same bound on both protocols |

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
| LIM-O1 | Time to exit after end of file on stdin or `SIGTERM` (`service.shutdown_ms`) | 3000 ms | 500–10000 | REQ-OPS-002 |
| LIM-O2 | Time until a cancelled call has aborted its outbound requests | 500 ms | fixed | REQ-MCP-007 |
| LIM-O3 | Largest line of the stdio transport, newline included; a longer line is dropped without an answer and the server reads on | 16 MiB | fixed | 022a; the protocol library's bound (rmcp 3.6.0), not ours; 0.9.0 had none |

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
| LIM-K3 | Entries | 1000 | 1–100000 | Bounded memory (RFC 014) |
| LIM-K4 | Size of one entry (raw titles, snippets, URLs) | 32768 bytes | 1024–1048576 | Memory at most LIM-K3 × LIM-K4 (RFC 014) |

## Scraping (M5)

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-G1 | Scraping drivers | off | on, off | D-06, DR-006 (RFC 015) |

## Measured costs (the memory estimate of `rypsok check`)

These are not limits an operator sets. They are the costs that the audit
measured (handoff 020e, the delivered musl release build of 0.8.0, and
handoff 020x, which fitted both sets by one method), rounded up, and `rypsok check` combines them with the effective settings into the
worst-case memory it prints and compares with `MemoryMax` of the documented
unit (RFC 020, D-20, D-21):

`13 MiB + fetches_in_flight × max_bytes + parsing_in_flight × (5.2 × max_bytes + 11 MiB) + max_sessions × 60 KiB` (the sessions only while `legacy_revisions` is on),
plus `cache.max_entries × cache.max_entry_bytes` (LIM-K3 × LIM-K4) when the
cache is on. It is an estimate, not a guarantee.

The allocator decides how much a process holds after a parse, so the costs
exist twice (RFC 020, 020x): LIM-M1 to LIM-M3 for a Linux musl build (the
release binaries and the image), LIM-M6 to LIM-M8 for a Linux glibc build
(what `cargo install` makes there), which replace the three in the formula
above (`14 MiB`, `10.6 × max_bytes + 21 MiB`). Both sets are fitted by the same
method, and without the sessions the estimate is above every measured peak. `rypsok check` uses the set of
the build it runs as and says which. On a platform that was not measured it
prints the larger set with the note "measured on Linux; not measured for this
platform" and does not compare it with the unit.

| ID | Parameter | Default | Range | Basis |
|---|---|---:|---|---|
| LIM-M1 | The process at rest (musl) | 13 MiB | fixed | Idle 10.0 to 12.05 MiB in 20 runs (020x; audit 020e: 10 to 12, median 10.9), and a margin |
| LIM-M2 | The part of one parse that does not grow with the page (musl) | 11 MiB | fixed | 020x: see below (audit 020e, on one fetch: the line crossed at 12.4 MiB) |
| LIM-M3 | The part of one parse that grows with the page, in tenths of a byte per byte of page (musl) | 52 | fixed | 020x: see below (audit 020e, on one fetch: 4.34 per byte) |
| LIM-M4 | One live session of the handshake-based revisions | 60 KiB | fixed | Audit 020e: 58.5 KiB measured (56.7 to 60.6) |
| LIM-M5 | `MemoryMax` of the documented systemd unit, the figure the estimate is compared with | 256 MiB | fixed | `deployment.md` |
| LIM-M6 | The process at rest (glibc) | 14 MiB | fixed | 020x: idle 12.8 to 13.4 MiB in 20 runs, and a margin |
| LIM-M7 | The part of one parse that does not grow with the page (glibc) | 21 MiB | fixed | 020x: see below |
| LIM-M8 | The part of one parse that grows with the page, in tenths of a byte per byte of page (glibc) | 106 | fixed | 020x: see below |

Basis of LIM-M2, LIM-M3 and LIM-M6 to LIM-M8 (RFC 020, 020x, F-4 and C1). `tools/measure-memory
--defaults --page-mib=N` on the host glibc release build, with the defaults
(four fetches from four hosts, two parses at once), five runs at a 5 MiB page
and five at a 10 MiB page (the ceiling of `fetch.max_bytes`; LIM-F3's old top
of 20 MiB is no longer configurable). Peaks: 153.4 to 178.0 MiB at 5 MiB and
268.6 to 301.9 MiB at 10 MiB. The maximum of each size, less the idle base
(14 MiB) and the four bodies, is the cost of two parses: 144.1 and 248.1 MiB,
that is 72.0 and 124.0 MiB per parse, a line of 10.49 per byte of page
crossing at 19.6 MiB, rounded up to 10.6 and 21 MiB so that the estimate is
above every run (182 and 307 MiB against 178.0 and 301.9). The line is fitted
on the delivered concurrency of two parses; a single fetch costs less on glibc
(52 to 57 MiB at 5 MiB, 79 to 106 at 10 MiB) because the allocator keeps what
earlier parses freed, and the estimate is for the concurrent case. For other
numbers of parses it extrapolates, as the musl set does. The musl set is
fitted the same way on the musl binary: peaks 99.4 to 104.6 MiB (5 MiB) and
172.9 to 175.3 MiB (10 MiB); less the idle base (13 MiB) and the four bodies,
the maxima are 71.6 and 122.5 MiB for two parses (35.8 and 61.3 per parse), a
line of 5.13 per byte crossing at 10.2 MiB, rounded up to 5.2 and 11 MiB. The
estimate without sessions is 107 and 179 MiB, above the maxima 104.6 and 175.3.
A unit test pins the rule for both builds with these peaks.

