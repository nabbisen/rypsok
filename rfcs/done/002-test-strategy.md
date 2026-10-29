# RFC 002 — Test strategy and CI gates

**Status.** Implemented (0.1.0); accepted 2026-09-30.
**Tracks.** Milestone M0 of `ROADMAP.md`; requirements REQ-TEST-001,
REQ-TEST-012, REQ-TEST-013 (from M1), REQ-SEC-020, REQ-ARCH-005,
REQ-MAIN-005, REQ-RUST-002; the project rule "design specifications serve
as the source for test design".
**Touches.** `.github/workflows/`, `tools/`, `deny.toml`, the `tests.rs`
convention of every module, the fixture directories.
**Handoff.** [`../handoffs/002-test-strategy/README.md`](../handoffs/002-test-strategy/README.md)

## Summary

Tests validate the specification, not the code: every requirement marked
`T` names at least one test, and a script fails CI when one is missing for
the current milestone. Five test layers, each with a place and a naming
rule. CI gates that run on every change, and one scheduled job that is not a
gate.

## Design

### Test layers

| Layer | Where | What it proves | Network |
|---|---|---|---|
| Unit | `src/<module>/tests.rs` of the crate | One stage in isolation: cleaning, comparison keys, rank fusion, validation, configuration | None |
| Driver fixtures | `crates/rypsok-core/fixtures/<engine>/` with `tests.rs` of the driver | Parsing and error classification against recorded responses (REQ-TEST-002) | None |
| Integration | `crates/rypsok/tests/` | A pipeline end to end against a local mock server; egress rules against a local listener admitted only through an allow rule injected in code (REQ-TEST-005) | Loopback only |
| Protocol | `crates/rypsok/tests/protocol/` | The binary is spawned; JSON-RPC over stdio; tool list, schemas, budget, annotations, cancellation, stdout purity, shutdown (REQ-TEST-009) | None |
| Property and fuzz | `proptest` cases in `tests.rs`; `fuzz/` targets for cargo-fuzz | URL handling, cleaning, extraction never panic and keep their invariants (REQ-TEST-013) | None |

A sixth job, **live checks** (REQ-TEST-003), runs on a schedule against
real providers within their terms. It reports; it never gates.

### Naming rule

Every test function that verifies a requirement is annotated with a doc
comment naming it, and the requirement identifier appears in the function
name:

```rust
/// REQ-SEARCH-003: a `max_results` outside the range is refused.
#[test]
fn req_search_003_rejects_out_of_range_max_results() { … }
```

`tools/check-requirements-coverage` reads `docs/src/maintainers/requirements.md`,
collects every requirement whose method is `T` and whose milestone is at or
below the milestone named in `tools/CURRENT_MILESTONE`, and fails when one
has no test that names it. Requirements of later milestones are reported,
not enforced. The script runs in CI (REQ-TEST-001).

### Fixtures

Recorded provider responses live under `fixtures/<engine>/` as files named
by case: `search-ok.json`, `search-empty.json`, `error-429.json`,
`error-blocked.html`, `error-malformed.json`. Fixtures contain no
credentials and no personal data; a fixture with either is rejected in
review. Each driver ships with the cases of REQ-TEST-002 at least.

### Hostile corpora

Content-boundary tests (REQ-TEST-007) and resource tests (REQ-TEST-006)
draw from `crates/rypsok-core/fixtures/hostile/`: closing markers, Markdown
in titles, hidden elements, zero-width and tag characters, bidirectional
controls, instruction-like text, deep nesting, oversized bodies, compressed
bombs (generated at test time, never checked in). The corpus grows with
every incident and every fuzz finding.

### CI gates

`.github/workflows/ci.yml`, on every push and pull request, on Linux,
macOS and Windows (REQ-RUST-002), with the stable toolchain and, on Linux,
also the MSRV 1.88:

| Gate | Command | Requirement |
|---|---|---|
| Format | `cargo fmt --all --check` | REQ-TEST-012 |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | REQ-TEST-012, REQ-MCP-002 (print lints) |
| Tests | `cargo test --workspace` | REQ-TEST-012 |
| Dependency rule | `tools/check-core-dependencies` | REQ-ARCH-005 |
| Requirement coverage | `tools/check-requirements-coverage` | REQ-TEST-001 |
| Advisories and licenses | `cargo deny check advisories licenses sources` | REQ-SEC-020 |
| Documentation | `cargo doc --workspace --no-deps` with `-D warnings`; `mdbook build docs` | REQ-ARCH-007 |
| Definitions budget (from M1) | Protocol test asserting LIM-I1 to LIM-I3 | REQ-MCP-008 |

A scheduled workflow runs `cargo fuzz` targets for a fixed time on Linux
and the live checks; both report only.

### What is not tested automatically

Requirements marked `I`, `A` and `D` are verified by review, measurement
report and demonstration. The review request package of each milestone
lists them with the evidence (project rules: review packages reference the
specifications).

## Alternatives considered

- **Tests that mirror code structure only.** Rejected: the project rule
  makes the specification the source of test design, and untested
  requirements would go unnoticed.
- **Gating fuzzing.** Rejected: fuzzing is time-bounded and
  non-deterministic; findings enter the hostile corpus as deterministic
  tests, which do gate.
- **Live provider tests in CI.** Rejected: they violate provider limits under
  parallel CI and fail for reasons outside the code (REQ-TEST-003 makes them
  a scheduled report instead).

## Open questions

None that block M0. The mock server crate is chosen in the aggregation RFC.

## Amendment to RFC 002 (2026-10-20, M1 review)

- Requirements of milestone `All` with method `T` are enforced by the
  coverage gate from M1 on, because M0 has no pipeline to test determinism,
  panic isolation or network-free stages against. `All` means "every
  milestone that has the subject", never "M0 with placeholders".
- The deny step is `taiki-e/install-action@cargo-deny` followed by the
  literal `cargo deny` command of the gate table.
- Gate tests that read repository files live in
  `crates/rypsok/tests/m0_gates.rs`; a unit test of the core
  (`req_cfg_001_configuration_page_matches_the_schema`) checks the
  configuration page against the serialized schema, which is the second
  half of RFC 004's deliverable 7.

## Amendment to RFC 002 (2026-10-29, RFC 019 D9)

- The gate table gains two rows, both run by the `rules` job of
  `ci.yml`: **Limits table** `tools/check-limits` (REQ-CFG-003: the
  constants of `limits.rs` equal the rows of the limits table), and the
  dependency check now runs all four checks, `cargo deny check advisories
  bans licenses sources` (REQ-SEC-020; the `bans` check was written in
  `deny.toml` and never run by CI). `req_test_012_ci_workflow_lists_every_gate`
  requires both.
