# Verification ledger

The requirements marked `T` (verified by a test) whose tests do not yet
verify everything the requirement says. It is kept in the open on purpose
(RFC 020, D-18): the debt is known, listed, and closed before 1.0.0.

How to read it. The 2026-11-02 audit read the test of every `T` requirement
up to milestone M7 and gave it a verdict: *verifies* (the test fails if the
requirement is broken in the way its text describes), *partial* (it verifies
part, and the missing assertion is named), *names-only* (the assertions do not
depend on the behaviour). This page lists the *partial* and *names-only* rows
that are still open, with the missing assertion as the audit wrote it. A row
leaves the page when a test with the missing assertion lands, in the change
that adds the test.

What has been closed since the audit: the rows that handoff 020q closed
(`REQ-FETCH-007`, `REQ-FETCH-010`, `REQ-ENG-002`, `REQ-PAGE-005`,
`REQ-SEC-001`, `REQ-SEC-005`, `REQ-SEC-007`, `REQ-SEC-009`, `REQ-SEC-011`,
`REQ-SEC-016`, `REQ-PRIV-003`, `REQ-PRIV-004`, `REQ-ERR-004`, `REQ-RES-008`),
`REQ-OBS-005` (handoff 020r) and `REQ-MCP-008` (handoff 020m). The other
handoffs of the audit changed some of the paths listed below; the rows were
not re-read after them, so a row may be closed already and still be listed.

| Requirement | Verdict | What is missing |
|---|---|---|
| `REQ-ARCH-004` Deterministic pipeline | partial | fusion order independence asserted for one other arrival order; byte-for-byte output, the fixed accumulation order and explicit tie-breakers on equal scores are not asserted (normalization idempotence is not determinism of the whole pipeline) |
| `REQ-MCP-003` Streamable HTTP transport | partial | stateless JSON discover and call over HTTP, no session header; not asserted: that the 2024-11-05 HTTP+SSE endpoint is absent (a GET on its path) |
| `REQ-MCP-007` Cancellation | partial | stdio: no response after cancelled, next call works; HTTP: dropped connection counted cancelled; fetch cancelled < 1 s; the 500 ms LIM-O2 bound for aborting in-flight outbound requests is asserted only at engine level (< 400 ms), not through the protocol |
| `REQ-MCP-011` Structured results | partial | output schemas equal the committed fixtures and stdout text form equals structured content; no test validates a real response against its declared schema |
| `REQ-SEARCH-004` Language parameter | partial | default language and configured-only validated; the "engines that cannot honor are skipped with a notice" half is not asserted under this id (see REQ-ENG-007) |
| `REQ-SEARCH-005` Time range parameter | partial | value accepted and passed to the validated request; that it reaches the engine request and that an engine without support is skipped with a notice are not asserted under this id |
| `REQ-FETCH-005` Continuation | partial | window and continuation without gap over multibyte text; start beyond end refused with the length; not asserted: the tool description says the page may have changed (no test pins the description, DAT-6) |
| `REQ-ENG-003` Enable and disable by configuration | partial | disabled engine leaves served categories empty and an unknown type is refused; not asserted: a disabled engine is never queried |
| `REQ-ENG-010` Provider obligations as engine properties | partial | properties and limiter rate asserted; not asserted: cacheability obeyed by the cache, attribution by the formatter, User-Agent format per engine |
| `REQ-ENG-011` Result count per engine | partial | Wikipedia sends srlimit=10 (mock matches the parameter); Brave and DuckDuckGo do not assert the count parameter |
| `REQ-ENG-014` No browser imitation by scraping drivers | partial | standard User-Agent, Accept: text/html only, no other headers asserted; "never worked around" (403, 429, 503, challenge) is asserted under the REQ-ENG-015 tests |
| `REQ-ENG-015` Bounded scraping | partial | no-results page empty, challenge and 403 blocked, 429/503 rate_limited, redirects not followed, byte and node bounds; not asserted: the depth bound (NET-5) and parsing under LIM-C5 |
| `REQ-AGG-001` Concurrent requests | partial | three 120 ms engines finish < 300 ms (a real-time window); the per-engine cap LIM-C2 is not asserted here |
| `REQ-AGG-003` Overall deadline | partial | the call returns < 600 ms in the harness; not asserted here: LIM-S7 as the ceiling of a call and the startup refusal of LIM-S5 + LIM-S6 > LIM-S4 (configuration test names another id) |
| `REQ-AGG-005` Bounded concurrency | partial | process-wide cap of 1 serializes two 100 ms engines (>= 200 ms); per-engine LIM-C2 not asserted under this id |
| `REQ-AGG-006` Cancellation of useless work | partial | cancelled call ends as cancelled within 300 ms and no new request starts (calls == 1); that the engine future is dropped (aborted, not detached) is not asserted |
| `REQ-AGG-007` Failure isolation | partial | malformed response and network failure in one engine keep the other's results; a panic in a driver is not exercised under this id |
| `REQ-RANK-001` Canonical result model | partial | returned URL, key, ranks and title asserted; the date field of the merged result is not asserted |
| `REQ-RANK-003` Returned URL | partial | table of returned URLs incl. tracking removal, IDNA host, too long; no case with a query that has no tracking parameter, which is where "nothing else changed" fails (DAT-8: ?a=1&b becomes ?a=1&b=) |
| `REQ-CTX-006` Size bounds | partial | snippets, titles, link targets bounded and cut at a word boundary; not asserted: notices and error messages bounded by LIM-I4 (the bound is a runtime assert in ToolError::new, see TST in area 5) |
| `REQ-CTX-010` Budget unit | partial | 150 CJK characters fit a 150 bound and 151 do not (scalar values); the documentation sentence about token cost is not tested |
| `REQ-PAGE-001` Pipeline order | partial | privacy before policy, strict mode before policy, robots before request asserted; the order of the later stages (byte limit, media type, charset, parse, extraction, cleaning, truncation) is not asserted |
| `REQ-PAGE-006` Malformed content | partial | node bound stops the parse and the notice appears, malformed markup parses, no crash; "LIM-F11 levels" stops rendering, not parsing: the parse is unbounded in depth (NET-5) |
| `REQ-PAGE-008` Computation off the I/O path | partial | a parse that cannot get the permit before the deadline ends busy and the permit comes back; not asserted: parsing runs off the event loop, and that a parse does not extend the call past LIM-F5 (NET-5 shows a call at the bound) |
| `REQ-SEC-004` No effect of content on rypsok | partial | one scenario: instruction text in a result changes neither max_results nor the number of engine calls; not asserted: selecting an engine or altering configuration by content |
| `REQ-SEC-006` Agent-directed destinations | partial | strong tests (every row of REFUSED_RANGES, mapped addresses, rule order, canonical names, two property tests); but the range test iterates the table itself and asserts its length (26), so it cannot see a missing range: 2001:db8::/32 (documentation, named by this requirement), fec0::/10 and ::/96 are admitted (NET-4) |
| `REQ-SEC-017` Engine profile | partial | base_url https unless loopback, redirect not followed with the credential staying on the engine origin, https_only; environment proxies ignored is asserted under REQ-TEST-015, not this id; "only the configured host" is structural |
| `REQ-SEC-014` Process stability | partial | a panicking driver is isolated and classified request_failure; not asserted: a panic in extraction is contained (event `extraction_panic` has no test), release builds unwind, recursion bounded by LIM-F11 (NET-5: the parse is not) |
| `REQ-SEC-020` Supply chain and hardening | partial | tests read the repository's own files: licenses named in deny.toml, `cargo deny` named in ci.yml, `unsafe_code = "forbid"` in the manifests, every action pinned with a comment (real parse); secret redaction asserted; the advisory audit itself is only run by CI |
| `REQ-SEC-022` Request time and stream bounds | partial | a body sent one byte at a time is answered 408 between 3 and 6 s with LIM-H5 = 3 s; the HTTP/2 stream bound LIM-H4 is only validated as a configuration range and its default (no test opens streams; NET-7) |
| `REQ-CMP-002` Provider limits honored | partial | in-flight cap and Retry-After block with its cap asserted with real-time windows of 10 to 100 ms; the request rate LIM-C9 is asserted under REQ-ENG-010 |
| `REQ-CMP-003` Attribution | partial | required attribution reaches the result; "the documentation shall name the provider" is not tested |
| `REQ-CMP-005` robots.txt | partial | rules for product token and *, longest match, cache per origin and capacity, status table incl. 1 h failure TTL, redirect following under the policy, honored/disabled, unreachable is unavailable; not asserted: the 24 h cap of LIM-F12 for a successful read and the LIM-F13 prefix cut of a long robots.txt; a same-host redirect to a disallowed path passes (NET-3) |
| `REQ-RES-002` Rate-limit detection | partial | Retry-After parsing and bounds asserted here; 429 mapped to rate_limited is asserted under REQ-ENG-006 |
| `REQ-CACHE-003` Eligibility | names-only | the named test checks that `clear` empties the cache (invariant I11); entries carrying engine set, configuration version and policy are not asserted (eligibility itself is REQ-CMP-007) |
| `REQ-ERR-002` Error kinds | partial | one error example round-trips; "exactly six kinds" is REQ-SEC-016's test, the LIM-I4 bound is REQ-ERR-004's; a `detail` with fixed keys is not asserted for every constructor |
| `REQ-OBS-002` Latency | partial | median over the sliding window and null when empty (strong); "latency of every call" is logged as latency_ms in the call event but no test asserts it |
| `REQ-OBS-006` No content logging | partial | a provider error body is not in the response or in stderr at the default level; "at any level" contradicts invariant I5 (200 characters at debug), which the requirement does not mention |
| `REQ-LIFE-002` Disposal | partial | after a cancelled call the engine's Arc count is 1; releasing bodies, queries, URLs and intermediate results is not examined |
| `REQ-TEST-004` Deadline and partial-failure tests | partial | the named test is one end-to-end partial search; LIM-S4, LIM-S5, early completion and cancellation are tested under REQ-AGG-* ids |
| `REQ-TEST-005` Egress tests | partial | literal notations judged canonically and mixed DNS answers; every refused range, changing DNS answers, refused redirects, credentials and ports are asserted under REQ-SEC-006 to -009 (and, for ranges, only against the table itself, NET-4) |
| `REQ-TEST-007` Content boundary tests | partial | hostile corpus: no removed code point survives, the envelope keeps three fields, JSON round-trips; the ASCII form of hosts is under REQ-RANK-003, and provider text through error, status and log paths under REQ-ENG-009, REQ-STATUS-002, REQ-OBS-006 |
| `REQ-TEST-009` Protocol tests | partial | the stateless and handshake-based revisions return the same tool list; the other protocol tests (annotations, budget, cancellation, stdout purity, EOF) run on one revision each, so "against every supported revision" is not what the suite does |
| `REQ-TEST-010` Cache tests | partial | a circuit probe bypasses the cache and a cancelled call stores nothing; eligibility, isolation and expiration are asserted under REQ-CACHE-003..005 and REQ-CMP-007 |
| `REQ-TEST-013` Fuzz and property tests | partial | proptests for the cleaner invariants (idempotent, bound, no removed code point, JSON round trip), normalization and keys; no property test for extraction; the fuzz targets that cover extraction run weekly, report only, 121 s each (020f DEP-7, DEP-8) |
| `REQ-TEST-015` Engine profile tests | partial | environment proxy variables are ignored by an engine in a spawned binary; the credential never leaving its origin and HTTPS only are asserted under REQ-SEC-017 |
| `REQ-TEST-016` Limit tests | partial | a limiter at 60 per minute admits a burst of 6 (5 ms deadlines); the invocation limits of REQ-MCP-010 are asserted under that id |
| `REQ-TEST-017` Circuit tests | partial | opening after consecutive counted failures, reset by a success, transition fields; cooldown growth, probing, closing and Retry-After are under REQ-RES-004/005 |
| `REQ-TEST-018` Scraping driver tests | partial | three page states told apart, results in order with wrappers unwrapped, the node bound without panic; refusal without the switch, header absence, statuses and the redirect rule are under REQ-ENG-013..015 |
| `REQ-TEST-012` CI gate | partial | the test searches ci.yml for the gate strings with `contains`: a gate in a comment or a job name passes; it does not read the YAML (steps); the "1.88" and OS names likewise |
| `REQ-MAIN-005` Testable without the network | names-only | the test asserts that fixture files exist, the hostile directory holds > 5 entries and Cargo.toml mentions wiremock; testability of every stage is shown by the behavioral tests, not by this one |
| `REQ-RUST-002` Platforms | partial | the test asserts that ci.yml contains "ubuntu-latest", "macos-latest", "windows-latest" and `cargo test --workspace`; aarch64 is absent from CI and from the test (020f DEP-2); the release builds three of six combinations |
| `REQ-CFG-005` Validation at startup | partial | ranges, rules, the deadline rule and ports are refused with key and index, all problems at once; a wrongly typed value (string for a number, a float) is refused without the key (DAT-11) and no test asserts that it names it |
| `REQ-OPS-001` Command line | partial | check exit 1 on invalid/missing and 0 on valid, unknown command exit 2; `--version` is asserted by no test; `serve` startup errors exit 1 under REQ-SEC-018 |
