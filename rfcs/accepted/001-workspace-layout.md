# RFC 001 — Workspace layout and crate boundaries

**Status.** Accepted (2026-09-30) — implementer may start.
**Tracks.** Milestone M0 of `ROADMAP.md`; requirements REQ-ARCH-001,
REQ-ARCH-005, REQ-ARCH-007, REQ-ARCH-008, REQ-RUST-001, REQ-RUST-004,
REQ-DEP-005, REQ-MAIN-006.
**Touches.** The repository root, `Cargo.toml`, `crates/`, `docs/`,
`LICENSE`, `NOTICE`, `README.md`, `CHANGELOG.md`, `deny.toml`,
`.github/workflows/`.
**Handoff.** [`../handoffs/001-workspace-layout/README.md`](../handoffs/001-workspace-layout/README.md)

## Summary

rypsok becomes a Cargo workspace of two crates: `rypsok-core`, a library
that holds everything except protocol adapters and the command line, and
`rypsok`, the binary. The core has no dependency on any MCP SDK, and CI
proves it on every change. This RFC also fixes the repository scaffolding
that the project rules require and the crate metadata that publishing needs.

## Motivation

The owner's constraint K-02 ("keep the core independent of the MCP
transport so other clients can reuse the same core") is only real if the
compiler enforces it. The protocol has changed five times in two years and
its Rust SDK went from 1.0 to 3.0 in five months (startup review R-16). A
crate boundary makes protocol churn a change of one crate. mimlys, the first
consumer, may embed the core as a library later (decision D-35), which the
same boundary makes possible.

## Design

### Crates

```text
rypsok/                      workspace root
├── Cargo.toml               [workspace], [workspace.package], [workspace.dependencies]
├── crates/
│   ├── rypsok-core/         library: models, pipelines, drivers, policies, config, telemetry
│   └── rypsok/              binary: command line, MCP adapters (stdio now, HTTP in M4)
├── docs/                    mdBook (already present)
├── rfcs/                    RFCs and handoffs (already present)
├── LICENSE, NOTICE, README.md, CHANGELOG.md, ROADMAP.md
├── deny.toml                cargo-deny policy
└── .github/workflows/ci.yml
```

| Crate | Kind | May depend on | May not depend on |
|---|---|---|---|
| `rypsok-core` | library | Tokio, an HTTP client, serde, TOML, tracing, URL and HTML crates chosen by later RFCs | Any MCP SDK, any HTTP server framework, any crate whose purpose is human-facing rendering (REQ-RUST-004) |
| `rypsok` | binary | `rypsok-core`, the MCP SDK pinned to an exact version (REQ-ARCH-008), an HTTP server framework from M4 | — |

The dependency rule (REQ-ARCH-005) is checked in CI by
`cargo tree -p rypsok-core -e normal` containing no MCP SDK crate. The check
is a script under `tools/` and a step of the workflow; it fails the build.

### Module layout of `rypsok-core`

Module names are the components of external design §5. Each is a directory
with a `mod`-less layout (Rust 2018 module style, project rules): `foo.rs`
beside `foo/`, tests in `foo/tests.rs`, never inline (REQ-TEST-001,
project rules).

```text
crates/rypsok-core/src/
├── lib.rs           public interface: the application services (REQ-ARCH-007)
├── model/           request, result, outcome, notice, error types (external design §3, §4)
├── config/          TOML schema, validation, secrets by reference (external design §13.2)
├── engine/          the engine abstraction, registry, admission properties
│   └── drivers/     one module per driver; wikipedia and brave in M1
├── search/          the search pipeline
├── fetch/           the fetch pipeline (M3; empty module with a doc comment until then)
├── egress/          the egress component and its two profiles
├── boundary/        the content boundary
├── privacy/         the secret check
├── limits/          the limits table as typed constants, generated or mirrored from docs/src/maintainers/limits-and-defaults.md
└── telemetry/       structured logging, counters, health state
```

The public interface of `rypsok-core` is the three application services
and their request and response types. Everything else is `pub(crate)`. The
interface is documented with `#![warn(missing_docs)]` and carries the
notice "unstable until 1.0.0" in the crate documentation (REQ-ARCH-007).

### Module layout of `rypsok`

```text
crates/rypsok/src/
├── main.rs          entry: parses the command line, loads configuration, starts a transport
├── cli/             `serve`, `check`, `--version`; exit codes (REQ-OPS-001)
└── mcp/             the adapter: tool definitions, schema validation, mapping of responses and errors to MCP results
    └── stdio.rs     the stdio transport; http.rs arrives in M4
```

### Workspace metadata

```toml
[workspace]
members = ["crates/rypsok-core", "crates/rypsok"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.88"
license = "Apache-2.0"
repository = "https://github.com/nabbisen/rypsok"
authors = ["nabbisen"]

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
print_stdout = "deny"
print_stderr = "deny"
dbg_macro = "deny"
```

`print_stderr` is denied because all diagnostics go through `tracing`
(REQ-OBS-005); the tracing subscriber writes to stderr. `unsafe_code` is
forbidden by REQ-SEC-020. Both crates inherit the lints with
`[lints] workspace = true`.

The version of both crates is the workspace version. Releases are published
with `cargo publish --workspace` (project rule) under the names `rypsok` and
`rypsok-core`, both free on crates.io on 2026-09-29. Tags carry no `v`
prefix.

### Pinned SDK

`rypsok` depends on the official MCP SDK `rmcp` with an exact version
requirement (`=3.5.0` at the time of writing) and only the features it
needs (`server`, `transport-io`; `transport-streamable-http-server` in M4).
The pin changes only through a pull request whose description names the
protocol revisions added or dropped (REQ-ARCH-008, REQ-MCP-006).

### Scaffolding

| File | Content | Rule |
|---|---|---|
| `LICENSE` | Apache License 2.0 | Project rules |
| `NOTICE` | "rypsok — Copyright 2026 nabbisen" and the Apache notice | Project rules |
| `README.md` | Hero line, overview, why/when, quick start (to be filled at M1), design notes, links to `docs/` | Project rules; at most 200 lines |
| `CHANGELOG.md` | "Keep a Changelog" format; an `Unreleased` section | Project rules |
| `deny.toml` | Advisories: deny; licenses: allow Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, MPL-2.0; everything else denied; sources: crates.io only | REQ-SEC-020 |
| `.gitignore` | `/target`, editor files | — |
| `rust-toolchain.toml` | Not used; CI tests stable and the MSRV | REQ-RUST-001 |

The existing `src/main.rs` and root `Cargo.toml` are replaced by the
workspace.

## Alternatives considered

- **One crate with feature flags.** Rejected: a feature flag does not stop
  a dependency from reaching the core; a crate boundary does.
- **One crate per driver.** Rejected for now: two drivers do not justify the
  overhead. Drivers are modules behind Cargo features of `rypsok-core`
  (`engine-wikipedia`, `engine-brave`, both default) so that an operator
  who builds from source can drop one. A per-driver crate can be split out
  later without changing the public interface.
- **A separate `rypsok-types` crate.** Rejected: the types are the core's
  interface; a third crate adds churn without a consumer that needs it.

## Open questions

None that block M0.
