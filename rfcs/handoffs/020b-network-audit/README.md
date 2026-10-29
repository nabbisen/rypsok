# Handoff 020b — Audit of the networking

**Created 2026-10-29.** An audit handoff: findings only, no source change.

**Task title.** Audit every outbound and inbound network path and every
bound on a network-facing resource, and report findings in the format of
RFC 020 D1.
**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md),
D1 (method, severity, evidence, safety rules) and D4.
**Applicable requirements.** REQ-SEC-006 to -012 (egress), REQ-CMP-005,
REQ-FETCH-*, REQ-PERF-001, REQ-SEC-013 to -018, REQ-SEC-024, LIM-H*, LIM-F*,
and the LIM rows of every area below. **Seeds.** `arch-001-prepare` S2, S4, S5, S9 (network part),
S10; `M8-DEFERRED.md` rows "Other `send()` paths…", "One canonical
`Host::Name` constructor…", "Latency of a refused versus an unreachable
name", "Filter the SDK's per-refusal log lines", "SDK `Host`/`Origin` WARN".
**Rules.** As 020a; and RFC 020 D1's safety rules without exception.
**Depends on.** Nothing. **Purpose.** Show, with evidence, where the system
can be made to wait, allocate, connect or answer beyond what its limits
table and its threat register say.

## Change scope (what to audit)

For each area: read the code, read what the RFC and the register say, and
**exercise it** with a probe where a probe is possible. Report one row per
finding and, for each area, a line in the "checked, found as documented"
list.

| # | Area | Questions the report answers |
|---|---|---|
| 1 | **Client construction.** Every place that builds an HTTP client or sends a request: engine drivers, scraping drivers, `fetch`, the robots.txt reader, any other `send()` | Is there exactly one way to build and to classify errors? List every site (`grep` for `.send(`, `Client::builder`, `reqwest`/`hyper` use). Per site: timeouts (connect, total, read), redirect policy, proxy use (environment proxies must be ignored), decompression, body bound, TLS roots, user agent. A table: site × property |
| 2 | **Name resolution and connect.** The resolver and strict mode | Does every connection go through the policy at connect time (no resolve-then-connect gap)? Is a name canonicalized in exactly one way before any judgment (`Host::from_name`) — find every construction of `Host::Name` and show none bypasses it? What happens on a name with many addresses, a slow resolver, a resolver that returns a mix of allowed and refused addresses, a CNAME to a refused address? Probe with a local test resolver or hosts entries if the code allows, otherwise by the existing tests and by reading |
| 3 | **The built-in refusal table (S2).** IPv4 and IPv6 ranges | For each range of `REFUSED_RANGES`: is the comment true (the first review found a comment that said `2001::/23` covers documentation `2001:db8::/32`, which it does not)? For each IPv6 form the first review listed as admitted (IPv4-compatible `::7f00:1`, site-local `fec0::1`, documentation `2001:db8::1`), **what does Linux do on `connect()` to it from this machine** (use a closed loopback port or a local listener; report the result, do not infer)? Compare with IANA's special-purpose registries (cite the version you used) and list every special-purpose block not refused |
| 4 | **Redirects (S4).** Following and judging | A table: hop type (same host, other host, scheme change, port change) × checks applied (policy, robots.txt, scheme, count). For a same-host redirect to a path that robots.txt disallows: show what happens (probe with a local server), and what REQ-CMP-005 requires. State both; do not decide |
| 5 | **Bounds on inbound resources.** The listener | For each of: accept errors (S5: the accept loop continues at once on an error — reproduce descriptor exhaustion with `ulimit -n` low in a scratch run and report CPU use and log volume), connection count, header size, body size, idle and slow clients, request rate, sessions (LIM-H6), the SSE stream, shutdown with open streams. The bearer scheme comparison (S5: `Bearer `/`bearer ` only, RFC 7235 says case-insensitive): list what is accepted. Per bound: the LIM id, the code site, a test that fails without it (name it) or "none" |
| 6 | **Bounds on outbound resources.** Fetch and engines | Per bound (bytes before and after decompression, nodes, depth, redirects, concurrent fetches, per-host concurrency, cache size, robots.txt cache size, rate buckets, circuit state): the LIM id, the code site, whether it holds when the peer misbehaves, a test (name or "none"). Probe: endless body, endless chunked stream, a gzip bomb, 10 000 redirects, a peer that stops reading, a peer that accepts and never answers |
| 7 | **Panics and locks (S9, network part).** `expect("… lock")` and `unwrap` on the request path | List every site on a network request's path. For each: can a panic occur inside the critical section (what runs while the lock is held)? What happens to later calls if it did (show with a test in the scratchpad that poisons one: a clone of the code is enough)? State the cost of switching to a non-poisoning approach; do not do it |
| 8 | **Observability of refusals.** Logs | The operator log must show a refusal without a caller-chosen value. List every `tracing` event on the network paths and the fields it carries; which fields can carry text chosen by a remote caller or a remote server (the SDK's `Host`/`Origin` WARN is known; find others)? What is the log volume of a 1 000-request flood per refusal type? |
| 9 | **DuckDuckGo (S10).** A scraping driver's failure mode | `div.no-results` is "not verified at admission". With the live check (opt-in: **you may run the DuckDuckGo live check once**, `RYPSOK_LIVE=1` as the test's doc says; read it first) show what a no-results query returns and whether the driver counts it as a failure that opens the circuit. Report only |

## Explicit non-change scope

No source file, test, workflow or document is changed. Probes, scripts,
logs and configs live in the scratchpad. No commit, no push.

## Prohibited shortcuts

"Looks fine" without the list of sites you read. A bound reported as
holding without a probe or a test that shows it. Running a probe against a
host that is not yours.

## Required evidence and review request

`.git-exclude/review-requests/020b-network-audit/README.md` in the format
of RFC 020 D1: the findings table (`NET-n`), the table of area 1 (sites ×
properties), the table of area 4 (hops × checks), the table of area 5 and 6
(bounds), then the four closing lists. Where a probe was run: the command,
the config, the output (trimmed to what shows the fact), and the scratchpad
path of the script.
