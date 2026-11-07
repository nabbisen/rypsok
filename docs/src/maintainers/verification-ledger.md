# Verification ledger

The requirements marked `T` (verified by a test) whose tests do not yet
verify everything the requirement says. It is kept in the open on purpose
(RFC 020, D-18): the debt is known, listed, and closed before 1.0.0.

How to read it. The 2026-11-02 audit read the test of every `T` requirement
up to milestone M7 and gave it a verdict: *verifies* (the test fails if the
requirement is broken in the way its text describes), *partial* (it verifies
part, and the missing assertion is named), *names-only* (the assertions do not
depend on the behaviour). This page lists the *partial* and *names-only* rows
that are still open, with the missing assertion as the audit wrote it (five are narrowed to what
is still missing). A row
leaves the page when a test with the missing assertion lands, in the change
that adds the test.

What has been closed since the audit. The rows were read again against the
tree on 2026-11-07 (handoff 020q and its review); twelve left the page that
day, with these tests:

| Requirement | Verified by |
|---|---|
| `REQ-FETCH-005` | `req_mcp_011_tools_list_matches_the_fixture` pins the `web_fetch` description, which says the page may have changed |
| `REQ-ENG-003` | `req_ops_003_inactive_engines_are_disabled_not_fatal`: a disabled engine is never selected, so never queried |
| `REQ-RANK-003` | `req_rank_003_returned_url_table`: queries with no tracking parameter come back unchanged (`?a=1&b`) |
| `REQ-CTX-006` | `req_err_004_every_error_constructor_has_a_bounded_message_without_external_text` (messages); notices carry codes and counts, not text |
| `REQ-PAGE-006` | `req_page_006_a_document_at_the_depth_bound_parses_in_full`, `req_page_006_a_million_nested_divs_stop_at_the_bound_plus_one_chunk` |
| `REQ-PAGE-008` | `req_sec_014_a_panic_in_extraction_is_contained_and_logged` (the work runs off the thread of the call and ends at the call's deadline), `req_page_008_parsing_permit_bounds_parsing` |
| `REQ-SEC-006` | `req_sec_006_every_special_purpose_block_is_refused` (read from the IANA registry, not from the table under test) |
| `REQ-SEC-014` | `req_sec_014_a_panic_in_extraction_is_contained_and_logged` |
| `REQ-SEC-022` | `req_sec_022_an_http2_stream_beyond_max_streams_is_refused` |
| `REQ-CACHE-003` | `req_cache_003_an_entry_is_valid_only_for_its_caller_engine_and_parameters` (the requirement was reworded) |
| `REQ-MAIN-005` | verification method changed to inspection (`I`) |
| `REQ-CFG-005` | `req_cfg_005_a_wrong_kind_of_value_names_its_key`, `req_cfg_005_the_invalid_configurations_of_the_audit` |

Handoff 020q also closed `REQ-FETCH-007`, `REQ-FETCH-010`, `REQ-ENG-002`,
`REQ-PAGE-005`, `REQ-SEC-001`, `REQ-SEC-005`, `REQ-SEC-007`, `REQ-SEC-009`,
`REQ-SEC-011`, `REQ-SEC-016`, `REQ-PRIV-003`, `REQ-PRIV-004`, `REQ-ERR-004` and
`REQ-RES-008`; handoff 020r `REQ-OBS-005`; handoff 020m `REQ-MCP-008`. Five
rows were narrowed to what is still missing.

| Requirement | Verdict | What is missing |
|---|---|---|
| `REQ-ARCH-004` Deterministic pipeline | partial | fusion order independence asserted for one other arrival order; byte-for-byte output, the fixed accumulation order and explicit tie-breakers on equal scores are not asserted (normalization idempotence is not determinism of the whole pipeline) |
| `REQ-MCP-003` Streamable HTTP transport | partial | stateless JSON discover and call over HTTP, no session header; not asserted: that the 2024-11-05 HTTP+SSE endpoint is absent (a GET on its path) |
| `REQ-MCP-007` Cancellation | partial | stdio: no response after cancelled, next call works; HTTP: dropped connection counted cancelled; fetch cancelled < 1 s; the 500 ms LIM-O2 bound for aborting in-flight outbound requests is asserted only at engine level (< 400 ms), not through the protocol |
| `REQ-MCP-011` Structured results | partial | output schemas equal the committed fixtures and stdout text form equals structured content; no test validates a real response against its declared schema |
| `REQ-SEARCH-004` Language parameter | partial | default language and configured-only validated; the "engines that cannot honor are skipped with a notice" half is not asserted under this id (see REQ-ENG-007) |
| `REQ-SEARCH-005` Time range parameter | partial | value accepted and passed to the validated request; that it reaches the engine request and that an engine without support is skipped with a notice are not asserted under this id |
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
| `REQ-CTX-010` Budget unit | partial | 150 CJK characters fit a 150 bound and 151 do not (scalar values); the documentation sentence about token cost is not tested |
| `REQ-PAGE-001` Pipeline order | partial | privacy before policy, strict mode before policy, robots before request asserted; the order of the later stages (byte limit, media type, charset, parse, extraction, cleaning, truncation) is not asserted |
| `REQ-SEC-004` No effect of content on rypsok | partial | one scenario: instruction text in a result changes neither max_results nor the number of engine calls; not asserted: selecting an engine or altering configuration by content |
| `REQ-SEC-017` Engine profile | partial | base_url https unless loopback, redirect not followed with the credential staying on the engine origin, https_only; environment proxies ignored is asserted under REQ-TEST-015, not this id; "only the configured host" is structural |
| `REQ-SEC-020` Supply chain and hardening | partial | tests read the repository's own files: licenses named in deny.toml, `cargo deny` named in ci.yml, `unsafe_code = "forbid"` in the manifests, every action pinned with a comment (real parse); secret redaction asserted; the advisory audit itself is only run by CI |
| `REQ-CMP-002` Provider limits honored | partial | in-flight cap and Retry-After block with its cap asserted with real-time windows of 10 to 100 ms; the request rate LIM-C9 is asserted under REQ-ENG-010 |
| `REQ-CMP-003` Attribution | partial | required attribution reaches the result; "the documentation shall name the provider" is not tested |
| `REQ-CMP-005` robots.txt | partial | the mechanisms are tested with short times and small bounds: the expiry, the redirect limit, the prefix cut and the status table; not asserted: that the fetch service builds the cache with the defaults of LIM-F12 (24 h) and LIM-F13 (500 KiB), which `tools/check-limits` compares as numbers only |
| `REQ-RES-002` Rate-limit detection | partial | Retry-After parsing and bounds asserted here; 429 mapped to rate_limited is asserted under REQ-ENG-006 |
| `REQ-ERR-002` Error kinds | partial | one error example round-trips; "exactly six kinds" is REQ-SEC-016's test, the LIM-I4 bound is REQ-ERR-004's; a `detail` with fixed keys is not asserted for every constructor |
| `REQ-OBS-002` Latency | partial | median over the sliding window and null when empty (strong); "latency of every call" is logged as latency_ms in the call event but no test asserts it |
| `REQ-OBS-006` No content logging | partial | a provider error body is not in the response or in stderr at the default level; "at any level" contradicts invariant I5 (200 characters at debug), which the requirement does not mention |
| `REQ-LIFE-002` Disposal | partial | after a cancelled call the engine's Arc count is 1; releasing bodies, queries, URLs and intermediate results is not examined |
| `REQ-TEST-004` Deadline and partial-failure tests | partial | the named test is one end-to-end partial search; LIM-S4, LIM-S5, early completion and cancellation are tested under REQ-AGG-* ids |
| `REQ-TEST-005` Egress tests | partial | literal notations judged canonically and mixed DNS answers; every refused range (read from the IANA special-purpose registry), changing DNS answers, refused redirects, credentials and ports are asserted under REQ-SEC-006 to -009, not under this id |
| `REQ-TEST-007` Content boundary tests | partial | hostile corpus: no removed code point survives, the envelope keeps three fields, JSON round-trips; the ASCII form of hosts is under REQ-RANK-003, and provider text through error, status and log paths under REQ-ENG-009, REQ-STATUS-002, REQ-OBS-006 |
| `REQ-TEST-009` Protocol tests | partial | the stateless and handshake-based revisions return the same tool list; the other protocol tests (annotations, budget, cancellation, stdout purity, EOF) run on one revision each, so "against every supported revision" is not what the suite does |
| `REQ-TEST-010` Cache tests | partial | a circuit probe bypasses the cache and a cancelled call stores nothing; eligibility, isolation and expiration are asserted under REQ-CACHE-003..005 and REQ-CMP-007 |
| `REQ-TEST-013` Fuzz and property tests | partial | proptests for the cleaner invariants (idempotent, bound, no removed code point, JSON round trip), normalization and keys; no property test for extraction; fourteen fuzz targets cover the parsers, run by the release checks (RFC 020, D-23) and by the handoff that changes a parser, report only |
| `REQ-TEST-015` Engine profile tests | partial | environment proxy variables are ignored by an engine in a spawned binary; the credential never leaving its origin and HTTPS only are asserted under REQ-SEC-017 |
| `REQ-TEST-016` Limit tests | partial | a limiter at 60 per minute admits a burst of 6 (5 ms deadlines); the invocation limits of REQ-MCP-010 are asserted under that id |
| `REQ-TEST-017` Circuit tests | partial | opening after consecutive counted failures, reset by a success, transition fields; cooldown growth, probing, closing and Retry-After are under REQ-RES-004/005 |
| `REQ-TEST-018` Scraping driver tests | partial | three page states told apart, results in order with wrappers unwrapped, the node bound without panic; refusal without the switch, header absence, statuses and the redirect rule are under REQ-ENG-013..015 |
| `REQ-TEST-012` CI gate | partial | the gates are looked for in the `run` lines of the parsed workflow; the "1.88" and the OS names are still looked for as text (`req_rust_002_ci_matrix_covers_three_platforms`) |
| `REQ-OPS-001` Command line | partial | check exit 1 on invalid/missing and 0 on valid, unknown command exit 2, the token and warning cases of the audit; `--version` is asserted by no test |
