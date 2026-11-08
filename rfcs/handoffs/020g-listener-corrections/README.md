# Handoff 020g — Listener corrections (tests first)

**Created 2026-11-03.** A fix handoff of the audit (RFC 020, D3). Source:
audit reports 020b and its review `arch-003-audit/020b-review.md`.

**Task title.** Make the HTTP listener keep the bounds its documents state,
and give each bound the test it lacked.
**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md),
RFC 013 and RFC 018 (the bounds), RFC 019 D5/D6 (sessions, statuses).
**Applicable requirements.** REQ-SEC-018, REQ-SEC-022, REQ-TEST-014.
**Findings.** NET-1 (High), NET-2, NET-7, NET-8, NET-11, NET-12, NET-16.
**Rules.** English; one-line commit digests with an optional body of at most
400 characters, no `Co-authored-by`; **commit your own work; push only after
the architect has approved the review request**; no tag, no release.
**Depends on.** Nothing. (Owner decision D-4 reads LIM-H3 as "the wait for
the first byte and between requests"; the documents already say "idle
timeout of a connection", so this handoff makes that true. If the owner
decides otherwise the architect amends this file before you start.)
**Evidence rule.** Every probe script a finding rests on is copied to
`.git-exclude/evidence/020g/` before you hand in.

## Change scope

Order matters: **write every test of item 1 first, run it, and record that
the ones for items 2 to 4 fail on the unmodified tree** (paste the failure
lines into the review request); then fix.

| # | Required implementation | Required tests | Required documentation |
|---|---|---|---|
| 1 | **Tests for the bounds that had none (NET-7).** In `crates/rypsok/src/mcp/http/tests.rs`: LIM-H1 (with `max_connections = 2`, a third connection is closed without a response and `http.too_many_connections` is counted; a fourth works after one closes); LIM-H3 (three cases with the shortest legal idle time or a test-only constructor of the settings: a connection that sends **nothing**, one that sends a **partial header**, and a keep-alive connection idle between requests: each is closed after the idle time and frees its permit); LIM-H4 (an HTTP/2 client with prior knowledge, e.g. `hyper-util`'s client, opens `max_streams + 1` concurrent streams; the extra one is refused or queued, and the test says which, by what hyper does, not by what you guess) | The tests, named `req_sec_018_*`/`req_sec_022_*` (the requirement each verifies); they fail before items 2 and 3 where the audit says they do | — |
| 2 | **NET-1.** A connection that has sent no byte is closed after `idle_timeout` and its LIM-H1 permit is freed. Do it inside the connection's own task (never in the accept loop): wait for the stream to become readable with `tokio::time::timeout(idle_timeout, …)` before handing it to hyper; once a byte has arrived, hyper's `header_read_timeout` takes over as today. Count the close as `idle_connection` in `Counters::refused_http` (add the reason to the vocabulary and to the pages that list them); **no per-connection log line** (a flood of silent connections must not write a flood of log lines); the flush check of the counters already runs | The NET-1 reproduction as a test: fill `max_connections` with silent connections, wait the idle time, a legitimate request is answered | Limits table row LIM-H3 says "including a connection that has sent nothing"; `configuration.md`, `troubleshooting.md`: the new counter reason; the changelog under Fixed |
| 3 | **NET-2.** The accept loop backs off after an error: sleep 100 ms (racing the shutdown token), and log the error at most once per 10 s with the number of errors since the last line. Factor the policy into a small struct (`should_log(now)`, `delay()`) so it is unit-tested without descriptors | Unit tests of the struct; the `ulimit -n 24` probe of the audit re-run by you and its CPU and log figures in the review request (expected: far below 100 % and a handful of lines) | `deployment.md`: set `LimitNOFILE` to at least `max_connections` + 64 in the unit, next to the LIM-H1 line; the container line likewise |
| 4 | **NET-11.** The `Bearer` scheme is matched case-insensitively (RFC 9110 §11.1) followed by one or more spaces; a tab or other separator stays refused | A table test: `Bearer`, `bearer`, `BEARER`, `bEaReR` with one and with two spaces accepted; tab, no scheme, `Basic`, empty refused | — |
| 5 | **NET-12.** Find out whether hyper's HTTP/1 header buffer (`max_buf_size`) and the HTTP/2 header list size are configurable on the builder you use. If yes: set both to 16 KiB as a **fixed** new limit LIM-H7 (add the row to `limits-and-defaults.md` with range "fixed", the constant to `limits.rs`, and make `tools/check-limits` pass); a header block above it is answered 431 by hyper. If not: change nothing and say why in the review request | The 400,000-byte-header case of the audit (expected 431 after the change); a request with the largest legitimate headers (a 64-character token, long `Origin`) still succeeds | `stability.md` (statuses by owner: 431 is hyper's); `troubleshooting.md` |
| 6 | **Documents (NET-8, NET-16).** THR-27's residual row: the SDK's `Host`/`Origin` warning needs "a local process (loopback, no callers) or an authenticated caller (network mode: an unauthenticated request gets 401 first)". The LIM-F7 row says "Redirects followed" (the code applies it to every hop); the LIM-F11 row says "HTML nesting depth **at rendering**" | `tools/check-limits` and the doc tests | The rows themselves |
| 7 | **Changelog.** `[Unreleased]`: Fixed (idle connections, accept back-off, bearer case), Changed (the new counter reason; header bound 16 KiB, if item 5 applied: a request header block above 16 KiB is now refused) | — | `CHANGELOG.md` |

## Explicit non-change scope

No change to NET-9 (log volume of refused requests) or NET-10 (the SSE
stream's lifetime): the audit records them. No new configuration key. No
change to the session bound. No dependency added except a dev-dependency a
test needs (name it in the review request).

## Prohibited shortcuts

A timeout in the accept loop. A log line per idle close. A test that passes
on the unmodified tree for items 2 and 3. Changing a default.

## Required evidence and review request

The gates (fmt, clippy `-D warnings`, test, doc, requirements coverage,
limits, core-dependencies, `cargo deny`, `mdbook`); the failing-first output;
the NET-1 and NET-2 probes re-run with before/after figures; the review
request at `.git-exclude/review-requests/020g-listener-corrections/README.md`
with a "Questions" section.
