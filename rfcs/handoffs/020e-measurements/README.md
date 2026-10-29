# Handoff 020e — Measurements

**Created 2026-10-29.** An audit handoff: measurements and findings, no source change.

**Task title.** Measure what the requirements say shall be measured, with a
method another person can repeat, and compare each number with the limit it
supports.
**RFC.** [`../../accepted/020-project-audit.md`](../../accepted/020-project-audit.md),
D1 and D4.
**Applicable requirements.** REQ-PERF-001 to REQ-PERF-005; the LIM rows
whose basis says "to be measured"; `limits-and-defaults.md` header (values
provisional). **Seeds.** `arch-001-prepare` B7 and S1 (RSS about 50 KiB per
legacy session, now bounded by LIM-H6).
**Rules.** As 020b. **Depends on.** Nothing.
**Purpose.** Replace "provisional" and "to be measured" with numbers taken on
the delivered release build, or report that a number cannot be taken here.

## Change scope (what to measure)

For every measurement: hardware (CPU model, cores, RAM), OS and kernel,
rustc version, the commit, the command, the config file, the number of runs,
and the individual results (not only the mean). Release build
(`cargo build --release`). State what was running on the machine.

| # | Measurement | Method |
|---|---|---|
| 1 | **Memory ceiling (REQ-PERF-003).** | `tools/measure-memory` with 1, 4 and 8 concurrent fetches, 5 runs each. Then derive the bound the requirement defines (LIM-C3 × LIM-F3 for bodies, LIM-C5 × parse cost for trees, plus a fixed base), compute it from the limits table and from the measured parse cost, and compare with the measured peak. State by how much the measurement is below the derived bound and whether the derivation holds for the largest allowed page |
| 2 | **Memory with the sessions bound.** | A loopback listener with `max_sessions` at its default (256) and at the maximum (65536): create sessions up to the bound with scripted `initialize` POSTs and report RSS before, at the bound, and after the sessions expire (state the idle expiry). The deployment unit sets `MemoryMax=256M`: does 65536 sessions fit? (The first review measured about 50 KiB per session.) |
| 3 | **Overhead beyond the slowest engine (REQ-PERF-004).** | Mock engines on loopback with fixed delays (50, 200, 800 ms) through the real pipeline: the time from the slowest engine's answer to the tool's response, over 30 runs, with 1, 3 and 8 engines and 5, 10 and 20 results. Report median, p95 and maximum; compare with LIM-S6 (200 ms reserve) |
| 4 | **Cold start (REQ-PERF-005).** | Time from process start to the first answer against a **mock** engine (so the number is rypsok's, not the network's), over 20 starts; and the same with `tools/measure-cold-start` against the real engines **once** (it needs the network; it contacts Wikipedia and Brave only if configured: use Wikipedia only, no key). Compare with LIM-S4 |
| 5 | **Fetch throughput and latency.** | A local server with pages of 10 KB, 500 KB and 5 MiB: `web_fetch` latency (median, p95) with the default limits, sequentially and at the per-process concurrency bound. Report CPU time per fetch (`/usr/bin/time -v`) |
| 6 | **The limits that say "to be measured" or "provisional".** | Make the list from `limits-and-defaults.md` (the basis column); for each, say whether the measurements above or the earlier packages (`.git-exclude/reviewed/m1`, `reviewed/012-fetch-pipeline`) support the value; for those nothing measures, say what would |
| 7 | **The test suite and the build.** | Wall time of `cargo test --workspace` (cold and warm), of `cargo build --release`, binary size per target the release builds (the dry-run archives of the release workflow are the source), and the size of the compile-time dependency tree |

## Explicit non-change scope

No source change; `tools/` scripts are not edited (copy to the scratchpad to
parametrize). No third-party host is contacted except the one cold-start run
named in 4.

## Prohibited shortcuts

A single run reported as a number. A mean without the spread. A comparison
with a limit not named.

## Required evidence and review request

`.git-exclude/review-requests/020e-measurements/README.md`: one section per
measurement (method, environment, raw results in a table, summary, the
limit compared, the verdict `within`/`exceeds`/`cannot judge`), the findings
table (`PRF-n`), and the four closing lists. Raw outputs in the scratchpad,
named.
