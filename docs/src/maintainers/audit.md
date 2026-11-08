# Audit report (milestone M8, release 0.9.0)

This is the report of the project audit of RFC 020: what was examined, what
was found, what was done about each finding, and what remains. It is written
so that a reader can check each fix against the test that pins it.

## What the audit was

- **Scope.** The whole project as built through 0.8.0. That covers:
  - the data models and the external contract;
  - the networking: clients, resolver, timeouts, pools, proxies, redirects
    and every bound;
  - test adequacy, requirement by requirement;
  - the measurements of REQ-PERF;
  - the dependency tree;
  - dead code and drift from the RFCs;
  - the CI and release pipeline.
- **Method** (RFC 020, D1):
  - **Reports.** The development team investigated in five audit reports:
    network, data and contract, tests and code, measurements, and
    dependencies and CI. Each finding came with its evidence and how it was
    obtained (run, read or searched), and each report ended with what was
    checked and what was not.
  - **Re-performance.** The architect reviewed each report and re-ran at
    least a fifth of its findings, including every High and Medium resting
    on a run. The architect also set the final severities and the fix list.
  - **Decisions.** The owner decided 24 questions (RFC 020, D-1 to D-24 below).
  - **Fixes.** Each fix was a handoff of its own (020g to 020x), with tests
    first, shown failing before the fix, and its own review.
- **Dates.** 2026-10-29 (RFC 020 accepted) to 2026-11-08.
- **Limits of the audit, stated plainly.** The readers were the project's
  own roles, not an outside auditor: the audit's independence is the
  reviewer's re-performance, not a second organization. Windows could not
  be compiled on the audit machine; Windows is covered by CI only. Live
  providers were contacted only through the opt-in live checks.

## Summary

86 findings came from the five reports, plus 7 that arose while fixing them.

| Area | High | Medium | Low | Note |
|---|---:|---:|---:|---:|
| Network (NET) | 1 | 6 | 9 | 2 |
| Data and contract (DAT) | 0 | 8 | 13 | 2 |
| Tests and code (TST) | 0 | 8 | 6 | 2 |
| Measurements (PRF) | 0 | 2 | 4 | 3 |
| Dependencies and CI (DEP) | 0 | 5 | 8 | 7 |
| Found during the fixes | 0 | 1 | 3 | 2 |

Where a finding had two severities (NET-4, TST-14), it is counted at the
higher one. **The one High finding and every Medium one are fixed.** The
open items are listed under [What remains](#what-remains), each with whose
decision it is.

Contract changes that follow from the fixes are announced in the 0.9.0
section of the changelog, as REQ-MAIN-006 requires. They are why 0.9.0 is a
minor release.

## Findings and their fixes

Each row gives:
- the final severity;
- what was true before the fix;
- what was done: a fix in a handoff, or a decision `D-n` (of RFC 020) and
  then a fix;
- the requirement the fix is held to;
- one or more of the tests that pin the fix, by function name.

A test whose name starts with `req_` lives under `crates/`.

### Network

| Id | Sev. | Before | Done | Requirement | Pinned by |
|---|---|---|---|---|---|
| NET-1 | High | A TCP connection that sent no byte was never closed, so 64 of them blocked every later request | RFC 020, D-4; 020g, and a read-and-replay of the first bytes under one deadline | REQ-SEC-022 | `req_sec_022_a_silent_connection_is_closed_after_the_idle_time`, `req_sec_022_silent_connections_cannot_hold_every_permit`, `req_sec_022_one_ambiguous_byte_then_silence_is_closed_after_the_idle_time` |
| NET-2 | Medium | The accept loop retried at once on a descriptor-limit error (a full core, a flood of log lines) | 020g | REQ-SEC-018 | `req_sec_018_accept_errors_back_off_and_are_logged_once_per_interval` |
| NET-3 | Medium | A same-host redirect target was not checked against robots.txt | RFC 020, D-1; 020h | REQ-CMP-005 | `req_cmp_005_every_redirect_target_is_checked_against_robots_txt` |
| NET-4 | Medium | The refusal table admitted six non-global IPv6 blocks | RFC 020, D-2; 020h, with the table now checked against the IANA registry | REQ-SEC-006 | `req_sec_006_every_special_purpose_block_is_refused` |
| NET-5 | Medium | Parse time grew quadratically with nesting, so two deep pages made a third host wait | RFC 020, D-3; 020i | REQ-PAGE-006, REQ-SEC-012 | `req_sec_012_a_page_nested_100000_deep_parses_quickly`, `req_sec_012_deep_pages_on_two_hosts_do_not_starve_a_third` |
| NET-6 | Medium | A fetch ended by its deadline was given the search's message | 020k | REQ-ERR-004 | `req_sec_011_slow_server_hits_the_deadline` |
| NET-7 | Medium | No behavior test for the connection cap, the idle time or the stream bound | 020g | REQ-SEC-018, REQ-SEC-022 | `req_sec_018_the_connection_cap_refuses_the_third_and_frees_on_close`, `req_sec_022_an_http2_stream_beyond_max_streams_is_refused` |
| NET-8 | Low | The threat register said "unauthenticated" where only a local process qualifies | 020g (register reworded; the warning's size bounded by LIM-H7) | — | documents |
| NET-11 | Low | The `Bearer` scheme was matched case-sensitively | 020g | REQ-SEC-018 | `req_sec_018_the_bearer_scheme_is_case_insensitive` |
| NET-12 | Low | Header sizes had no limit of ours (a 400,000-byte header was accepted) | 020g: LIM-H7, 16 KiB | REQ-SEC-018 | `req_sec_018_an_oversized_header_block_is_431`, `req_sec_018_the_http2_header_list_is_bounded` |
| NET-13 | Low | 29 lock sites would panic on a poisoned lock | 020j | REQ-SEC-014 | `req_sec_014_lock_recovers_a_poisoned_mutex`, `req_sec_014_no_expect_on_a_standard_library_lock` |
| NET-16 | Low | Two rows of the limits table were worded wrongly | 020g | — | `tools/check-limits` |

### Data and contract

| Id | Sev. | Before | Done | Requirement | Pinned by |
|---|---|---|---|---|---|
| DAT-1 | Medium | `check` passed a configuration that `serve --transport http` refused | 020l | REQ-OPS-001 | `req_ops_001_check_and_serve_refuse_the_same_callers_alike` |
| DAT-2 | Medium | `check` printed four items and warned about nothing | RFC 020, D-6; 020l | REQ-OPS-001 | `req_ops_001_check_prints_every_section`, `req_ops_001_each_risky_setting_has_its_warning` |
| DAT-3 | Medium | A non-http scheme was told it had been "retried over https" | 020k | REQ-ERR-004 | `req_err_004_a_scheme_that_is_not_http_has_its_own_message` |
| DAT-4 | Medium | 404, 403 and 410 were `unavailable`, and the pages said to retry | RFC 020, D-7; 020k | REQ-FETCH-011 | `req_fetch_011_the_message_says_whether_retrying_helps` |
| DAT-5 | Medium | The stability page promised more `#[non_exhaustive]` types than the code had | 020m | REQ-ARCH-007 | `req_arch_007_stability_page_lists_match_the_source` |
| DAT-6 | Medium | No fixture pinned the input schemas and the tool descriptions | 020m | REQ-MCP-011 | `req_mcp_011_tools_list_matches_the_fixture` |
| DAT-7 | Medium | "Nothing on standard output" had no test in HTTP mode | 020m | REQ-MCP-002 | `req_mcp_002_http_serve_writes_nothing_to_stdout` |
| DAT-8 | Medium | Every returned URL had its query re-serialized | 020n | REQ-RANK-003 | `req_rank_003_returned_url_table` |
| DAT-9 | Low | The comparison key could merge different resources and showed `http` over `https` | RFC 020, D-8; 020n | REQ-RANK-002, REQ-RANK-004 | `req_rank_002_the_twelve_pairs_of_the_audit`, `req_rank_004_the_https_url_is_shown_when_the_best_is_http` |
| DAT-10 | Low | `SearchResult` could be deserialized past the boundary | 020m | REQ-SEC-001 | `compile_fail` doc tests in `boundary.rs` |
| DAT-11, 12, 14 | Low | Configuration messages missed the key, named Rust types, or named an unset key | 020l | REQ-CFG-005 | `req_cfg_005_the_invalid_configurations_of_the_audit`, `req_cfg_005_a_wrong_kind_of_value_names_its_key` |
| DAT-13 | Low | Ten "operator" limits had no key | RFC 020, D-9; 020i, 020s (keys for LIM-S12 and LIM-O1; the rest "fixed") | REQ-CFG-005 | `req_cfg_005_the_connect_timeout_and_the_shutdown_time_are_keys` |
| DAT-15 | Low | The secret check refused ordinary text and passed many credential shapes | 020n | REQ-PRIV-002 | `req_priv_002_the_corpus_of_ordinary_text_passes`, `req_priv_002_the_corpus_of_credentials_is_refused` |
| DAT-16 | Low | The boundary removed joiners that carry orthography | RFC 020, D-10; 020n | REQ-SEC-015 | `req_sec_015_joiners_that_carry_orthography_are_kept` |
| DAT-17, 19, 20 | Low | Three wrong error texts; a tool description and a notice documented two ways | 020k | REQ-ERR-004, REQ-MCP-008 | `req_search_003_an_unserved_category_lists_the_served_ones`, `req_mcp_008_the_web_fetch_description_is_the_designs` |
| DAT-18 | Low | A `disabled` engine did not say why | 020k (an optional `reason`) | REQ-STATUS-001 | `req_status_001_a_disabled_engine_says_why` |

### Tests and code

| Id | Sev. | Before | Done | Requirement | Pinned by |
|---|---|---|---|---|---|
| TST-1 | Medium | The coverage gate could not tell a test that names a requirement from one that verifies it | RFC 020, D-18, D-19 (as amended by RFC 020, D-23): the [verification ledger](./verification-ledger.md), mutation testing in the release checks | REQ-TEST-019 | `req_test_019_the_release_checks_mutate_the_six_security_modules` |
| TST-2 | Medium | No log line carried a request id or the caller | 020r | REQ-OBS-005 | `req_obs_005_every_event_of_a_call_has_its_request_id_and_caller` |
| TST-3, TST-4 | Medium | A range test took its expectations from the table it tested; the URL table missed the case that failed | 020h, 020n | REQ-SEC-006, REQ-RANK-003 | `req_sec_006_every_special_purpose_block_is_refused`, `req_rank_003_returned_url_table` |
| TST-5 | Medium | No test saw an `http` → `https` upgrade succeed | 020q | REQ-FETCH-010 | `req_fetch_010_an_http_url_is_fetched_over_https_when_the_site_speaks_it` |
| TST-6 | Medium | No test of what a fetch sends, or of proxy variables in the environment | 020q | REQ-FETCH-007, REQ-SEC-009 | `req_fetch_007_a_fetch_sends_get_and_only_the_headers_rypsok_sets`, `req_sec_009_a_proxy_in_the_environment_is_not_used` |
| TST-7 | Medium | Mutants survived on security paths | 020q, 020s: 54 missed mutants to 0, six equivalent ones listed with reasons in `.cargo/mutants.toml` | REQ-TEST-019 | the release checks' mutation jobs |
| TST-8 | Low | 35 tests depended on wall-clock time | 020s (paused time, exact windows) | REQ-AGG-003 | `req_agg_003_overall_deadline_holds` |
| TST-9 | Low | The crate did not compile without its default features | 020s; CI checks each feature alone | REQ-TEST-012 | `req_test_012_ci_checks_every_feature_alone_and_none` |
| TST-10 | Low | Inert public items and an unused limit | 020s (removals announced) | REQ-MCP-007 | `req_mcp_007_a_cancelled_call_drops_its_engine_task_within_lim_o2` |
| TST-11, 12 | Low | Error messages were bounded only in debug builds; some tests checked source text, not behavior | 020o, 020q, 020s | REQ-ERR-004 | `req_err_004_every_error_constructor_has_a_bounded_message_without_external_text` |
| TST-13 | Note | Four example answers could not be regenerated | 020s (the page, written back from the answers, and a regeneration tool) | — | `req_doc_002_the_page_behind_the_fetched_answers_is_kept_and_described` |
| TST-14 | Medium | Parsers of untrusted input had no fuzz target | 020u (five targets); seeds for all 14 (020w) | REQ-SEC-023 | `req_test_019_every_fuzz_target_has_committed_seeds` |
| TST-15 | Low | 16 claims of the RFCs had drifted from the code | 020s (RFC amendments; the agent client's keepalive set) | REQ-SEC-007 | `req_sec_007_the_agent_client_sets_tcp_keepalive_and_the_idle_timeout` |

### Measurements

| Id | Sev. | Before | Done | Requirement | Pinned by |
|---|---|---|---|---|---|
| PRF-1 | Low | Four default fetches peaked above the published memory derivation | 020l, 020x: the estimate is built from measured costs, one set per Linux build | REQ-PERF-003 | `req_perf_003_the_estimate_is_above_every_measured_peak_without_sessions` |
| PRF-2 | Medium | One 20 MiB page cost about 120 MiB; four of them were killed inside the documented unit | RFC 020, D-20; 020l: `fetch.max_bytes` ceiling 10 MiB, and `check` prints the derived worst case | REQ-PERF-003 | `req_perf_003_the_new_ceiling_fits_the_unit_and_the_old_one_did_not` |
| PRF-3 | Medium | About 58.5 KiB per session; the documented maximum did not fit the unit | RFC 020, D-21; 020l: ceiling 2048, cost in the estimate | REQ-PERF-003, REQ-SEC-024 | `req_sec_024_max_sessions_has_its_range` |
| PRF-4 | Low | With swap, the memory limit only slowed the host | 020t: the unit and the container line forbid swap | REQ-PERF-003 | `req_perf_003_the_deployment_page_forbids_swap` |
| PRF-5 | Low | The post-processing reserve LIM-S6 cited no measurement | Kept at 200 ms; its basis cites the measurement (this report's change) | REQ-PERF-004 | documents |
| PRF-6, 7, 9 | Note, Low | The burst behavior of the Wikipedia admission was unstated; the memory tool did not measure the delivered configuration; the LIM-S4 basis said "to be measured" | 020t | — | documents; `tools/measure-memory --defaults` |
| PRF-8 | Note | Release binaries were not stripped | RFC 020, D-22; 020p | REQ-SEC-014 | `req_sec_014_the_release_profile_strips_symbols_and_keeps_unwinding` |

### Dependencies, CI and release

| Id | Sev. | Before | Done | Requirement | Pinned by |
|---|---|---|---|---|---|
| DEP-1, 15 | Medium | The container image was not a release file, and was compiled apart from the release binaries | RFC 020, D-12; 020p: the image is the release binary on a pinned base, listed in `SHA256SUMS` | REQ-DEP-005 | `req_dep_002_the_image_is_made_from_the_release_binary` |
| DEP-2 | Medium | Linux aarch64 was not built | RFC 020, D-13; 020p | REQ-RUST-002 | `req_dep_005_the_release_and_its_checks_build_the_same_four_targets` |
| DEP-3 | Medium | Releases were built with a floating compiler | 020o, 020w, 020p: one named compiler, in one place | REQ-TEST-019 | `req_test_019_the_release_compiler_is_named_in_one_place` |
| DEP-4 | Note | The Windows binary differed between two builds of one commit | 020o (`/Brepro`); the release checks compare two builds of every target | REQ-TEST-019 | `req_test_019_the_reproducibility_check_builds_twice_and_compares_the_binaries` |
| DEP-5, 6 | Medium, Low | Workflows held more permissions than they used | 020o | REQ-SEC-020 | `req_sec_020_every_workflow_declares_least_permissions` |
| DEP-7, 8 | Medium, Low | Scheduled checks could fail without failing; fuzzing ran two minutes a week | 020o, then 020w: the release checks, with no timer (RFC 020, D-23) | REQ-TEST-019 | `req_test_019_no_workflow_runs_on_a_timer`, `req_test_019_the_release_checks_fuzz_every_target_for_300_s` |
| DEP-9, 10, 14 | Low | CI did not use `--locked`; an unused dependency; floating tools and base images | 020o | REQ-TEST-012, REQ-SEC-020 | `req_test_012_cargo_commands_of_the_workflows_are_locked`, `req_sec_020_tools_and_the_release_compiler_are_pinned` |
| DEP-17 | Note | No proof of where a release file was built | RFC 020, D-16; 020p: provenance attestations, made by a job that runs no build code | REQ-SEC-020 | `req_sec_020_only_the_checksums_job_signs_and_runs_no_build_code` |
| DEP-18 | Low | The book was published on every push and described unreleased code | RFC 020, D-17; 020t: published from the release tag, and the first page names the release | REQ-DOC-002 | `req_doc_002_the_book_is_published_from_the_release_tag` |

### Found while fixing

| Id | Sev. | Before | Done | Pinned by |
|---|---|---|---|---|
| F-1 | Low | A panic wrote plain text, with its message, to standard error | 020s: one JSON line without the message | `req_obs_005_a_panic_is_one_json_line_without_its_message` |
| F-2 | Low | A fuzz target asserted more than the driver promises (the product was correct) | 020w | `req_eng_001_oddly_written_links_are_made_canonical_by_the_pipeline` |
| F-3 | Low | A client that could not be built did not say why, and said so twice | 020x | `req_ops_001_a_missing_trust_store_is_named_once_at_start` |
| F-4 | Medium | `check`'s memory estimate was exceeded by a glibc build | 020x: one measured set per Linux build, and the build named | `req_perf_003_check_names_the_build_of_the_estimate` |
| N1 | Note | No page said which certificates outbound fetches trust | 020t | documents |
| — | — | A property test could generate an address as a host name | 020q (tests only) | `req_sec_006_an_address_deny_rule_is_a_network_rule_not_a_host_rule` |

## The decisions (RFC 020)

- **Networking and egress.**
  - **RFC 020, D-1:** every redirect target is checked against robots.txt.
  - **RFC 020, D-2:** the refusal table follows the IANA special-purpose registries.
  - **RFC 020, D-3:** parse time is linear in nesting depth.
  - **RFC 020, D-4:** LIM-H3 bounds a connection that never speaks.
- **Interface and contract.**
  - **RFC 020, D-6:** what `check` prints and warns about.
  - **RFC 020, D-7:** a fetch says whether retrying can help.
  - **RFC 020, D-8:** which URLs are the same resource.
  - **RFC 020, D-9:** a limit without a key is "fixed".
  - **RFC 020, D-10:** joiners that carry orthography are kept.
  - **RFC 020, D-11:** the frozen public surface is a 1.0.0 decision.
- **Release path.**
  - **RFC 020, D-12:** the image is built from the release binary.
  - **RFC 020, D-13:** Linux aarch64 is built.
  - **RFC 020, D-14, amended by D-23:** dependencies are updated once per release
    cycle.
  - **RFC 020, D-15:** a ruleset protects `main` and the tags; this is an owner
    setting.
  - **RFC 020, D-16:** build provenance.
  - **RFC 020, D-17:** the book is published from the release tag.
- **Tests.**
  - **RFC 020, D-18:** the verification debt on security paths is closed, and the
    rest is a public ledger.
  - **RFC 020, D-19, amended by D-23:** a committed exclusion list for mutation
    testing.
- **Limits and build.**
  - **RFC 020, D-20:** a 10 MiB page ceiling, and `check` prints the worst case.
  - **RFC 020, D-21:** the session ceiling is 2048.
  - **RFC 020, D-22:** stripped release binaries.
- **RFC 020, D-23: when checks run** ([When checks run](./checks.md)). CI on every
  push, the deep checks in each change and before every release, and no
  timer.
- **RFC 020, D-24:** a search result is shown only with a host that
  `web_fetch` would accept; one with an empty label is invalid.
- **RFC 020, D-5:** one deliberate DuckDuckGo no-results request; it is under
  "What remains".

## What remains

**Recorded, with no change** (each was judged as needing none, or not yet):

- **NET-9 (Low).** A refused request writes one log line, two for a foreign
  `Host`, with no rate limit of its own. The refusal counters are written
  once a minute.
- **NET-10 (Low).** An event stream of the legacy session revision is
  bounded by neither the request time (LIM-H5) nor the idle time (LIM-H3).
  Only an authenticated caller with a session can open one.
- **NET-14 (Low).** A resolver that never answers leaves one blocking thread
  per lookup until it gives up. The caller is released at the connect
  timeout, and the `web_fetch` rate limit bounds the count.
- **NET-15 (Note).** The engines' clients use the system resolver, not the
  judging one. An engine's address is the operator's setting and the
  operator's trust (REQ-SEC-017).
- **DAT-21 (Note).** The output schema allows any notice reason with any
  notice code. Per-code schemas would let a host validate this.
- **DAT-23 (Note).** A library user can hold a `Config` that is not yet
  validated. The service validates before it uses one.
- **DEP-11, 12, 19, 20 (Note).**
  - The declared minimum Rust version, 1.88, is true and tested.
  - Four crates appear in two versions each.
  - The Actions caches were above GitHub's 10 GB.
  - The repository's history holds no secret.
- **Residuals inside fixed rows:**
  - From accept to a complete header block can take up to twice LIM-H3
    (NET-1).
  - The SDK's warning for a foreign `Host` still carries header text, now
    at most 16 KiB (NET-8).
  - With fewer engine features the tests compile, but not all of them pass
    (TST-9).

**Open, and whose decision it is:**

| Item | What | Whose |
|---|---|---|
| The verification ledger | 41 requirements whose tests verify part of what they say ([ledger](./verification-ledger.md)) | closed before 1.0.0 (RFC 020, D-18) |
| DAT-22, RFC 020, D-11 | Which public items are frozen | the 1.0.0 preparation |
| NET-18, RFC 020, D-5 | Whether DuckDuckGo's no-results page carries the marker the driver reads; without it, three result-less searches open the engine's circuit | one deliberate request, in the release preparation (020v) |
| N2 | A returned URL whose host has an empty label (such as a leading dot) is shown, although it cannot be fetched | decided (RFC 020, D-24, 2026-11-08): such a result is invalid, dropped and counted; the fix is handoff 020y |
| DEP-13 | Several pinned actions are a major version behind | the dependency step at the start of the next cycle (RFC 020, D-14) |
| DEP-16, RFC 020, D-15 | `main` and the release tags have no protection rule | the owner (a repository setting) |
| First runs | The provenance attestations, the attach of the images, and the book from a tag run for the first time on the 0.9.0 tag | read in that run, as the release procedure says |

## Repeating the audit

Every fix above is pinned by a test or a gate in the tree. `cargo test
--workspace --locked` runs them, and the release checks ([When checks
run](./checks.md)) run the fuzzing, the mutation testing and the
reproducibility check. The audit reports' raw evidence was kept in the
project's private review folder. A finding is named here only once its fix
is in the tree.
