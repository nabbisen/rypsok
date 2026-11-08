# Handoff 020x — Say why a client cannot be built; an estimate for the binary that prints it

**Created 2026-11-08.** A fix handoff of the audit (RFC 020, D3), wave 4.
Findings F-3 (Low) and F-4 (Medium) of the 020t review. **RFC 020, D-20**
already asks that `rypsok check` tell an operator the truth about their own
settings. F-4 shows that the estimate is true for one of the two Linux
builds only.

**RFC.** [`../../done/020-project-audit.md`](../../done/020-project-audit.md).
**Rules.** As 020w, including the per-change rule (`egress.rs` is not in its
list, but run mutation on it anyway: 020s found six mutants alive there).
Evidence goes to `.git-exclude/evidence/020x/`.
**Depends on.** 020t.

## Change scope

| # | Change | Test (fails before) |
|---|---|---|
| 1 | **F-3, the start-up message.** When a client cannot be built, the message says **once** that the HTTP client could not be built, and then **the cause**, taken from the error's source chain. Read the chain of the `reqwest` builder error. When the cause is that no root certificate could be loaded, the message names it and where roots come from: `no root certificates could be loaded (the system store, or SSL_CERT_FILE / SSL_CERT_DIR when set; see the deployment page, "Which certificates the fetches trust")`. Any other cause is shown as the source's own text, never just `builder error`. Find where the clause is doubled (`ClientError`'s `#[error]` and the caller in `lib.rs:140` both add it) and keep one. Name every message you change in the review request | A test that starts the binary with `SSL_CERT_FILE` pointing at an empty file (Linux only, `#![cfg(target_os = "linux")]`). It asserts exit 1; the clause "could not be built" exactly once; and "root certificates" and `SSL_CERT_FILE` in standard error. A unit test that a source chain is rendered, not `builder error` |
| 2 | **F-4, the estimate follows the allocator.** Measure the glibc build as 020e and 020t measured the musl build: `tools/measure-memory --defaults` at two page sizes, so that the parse cost is a line (the default 5 MiB and 20 MiB, LIM-F3's top), at least three runs each, and the idle base. Add the glibc constants beside the musl ones: LIM-M rows "(glibc)" in the limits table, with the measurement as basis. `Limits::worst_case_memory` selects them with `cfg!(target_env = "musl")` on Linux. On every other platform, `check` prints the estimate with the note `measured on Linux; not measured for this platform`, and does not warn against the unit's 256 MiB, because the unit is a Linux unit. `check` prints which build the estimate is for (`musl`, `glibc` or `not measured`). Update the `deployment.md` sentence of 020t to say that `check` now gives each Linux build its own figure | Unit tests: on a gnu Linux test build the glibc constants are used (`cfg`-gated test), and on a musl one the musl constants are used (gated as well, so whichever one CI builds runs). The `check` output names the build; this is already pinned by the `check` output tests, so extend them. `tools/check-limits` covers the new rows |
| 3 | Changelog `[Unreleased]`, under Fixed: "`rypsok check` gives the memory estimate of the build it runs as (musl or glibc on Linux) and says when a platform is not measured"; "A client that cannot be built says why, once" | — |

## Explicit non-change scope

- No change of any limit's value.
- No new allocator.
- No `MALLOC_ARENA_MAX` or other tuning of glibc. Measure what a user gets.

## Required evidence and review request

1. The gates.
2. The glibc measurement runs (raw output) and the fit.
3. The two messages before and after.
4. Mutation of `egress.rs` and `limits.rs`.
5. The review request at `.git-exclude/review-requests/020x-truthful-startup-and-estimate/README.md`.
