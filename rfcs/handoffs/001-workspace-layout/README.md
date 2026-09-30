# Handoff — RFC 001: Workspace layout and crate boundaries

**RFC.** [`../../accepted/001-workspace-layout.md`](../../accepted/001-workspace-layout.md)
(the design; this handoff never overrides it).
**Specifications.** `docs/src/maintainers/requirements.md` (REQ-ARCH-001,
-005, -007, -008; REQ-RUST-001, -004; REQ-DEP-005; REQ-MAIN-006),
`docs/src/maintainers/external-design.md` §5, `ROADMAP.md` M0.
**Rules.** `.git-exclude/rules/project-instructions-general-common.md`,
`.git-exclude/rules/project-instructions-rust.md` (test files beside
modules, never inline; Rust 2024; 2018 module style).
**Milestone.** M0. No feature code: this handoff produces an empty,
compiling, publishable workspace with its scaffolding.

## 1. What to build

| # | Deliverable | Acceptance |
|---|---|---|
| 1 | Workspace `Cargo.toml` with the metadata and lints of RFC 001 | `cargo metadata` shows two members; both inherit `[workspace.package]` and `[lints]` |
| 2 | `crates/rypsok-core` with the module tree of RFC 001, each module a documented empty shell (`//!` comment naming its component and its requirements) and a `tests.rs` beside it | `cargo build`, `cargo doc -D warnings` pass; `#![forbid(unsafe_code)]` effective |
| 3 | `crates/rypsok` with `main.rs`, `cli/`, `mcp/stdio.rs` shells; `--version` works; `serve` and `check` print "not implemented" to stderr and exit 2 until M1 | `rypsok --version` prints the workspace version; exit codes as REQ-OPS-001 |
| 4 | `rmcp` pinned exactly in `crates/rypsok/Cargo.toml` with features `server`, `transport-io` | `cargo tree -p rypsok` shows one `rmcp` version |
| 5 | `tools/check-core-dependencies` (shell or Rust script) | Fails when `rmcp` appears in `cargo tree -p rypsok-core -e normal`; proven by a deliberate temporary breakage in the PR description |
| 6 | `LICENSE`, `NOTICE`, `README.md`, `CHANGELOG.md`, `.gitignore`, `deny.toml` as listed in RFC 001 | Present; README under 200 lines; `cargo deny check` passes |
| 7 | Removal of the old root `src/main.rs` | Gone |

The CI workflow itself belongs to RFC 002; build the workspace so that its
gates pass.

## 2. Pull request plan

Small, reviewable steps; each PR names this handoff.

| PR | Content | Depends on |
|---|---|---|
| 1 | Workspace root: `Cargo.toml`, `.gitignore`, `LICENSE`, `NOTICE`, empty crates that build | — |
| 2 | Module tree of `rypsok-core` with documentation shells and `tests.rs` files | 1 |
| 3 | `rypsok` binary: command line skeleton, exit codes, pinned `rmcp` | 1 |
| 4 | `tools/check-core-dependencies`, `deny.toml`, `README.md`, `CHANGELOG.md` | 1–3 |

## 3. Conventions that the reviewer will check

- Rust 2024 edition; MSRV 1.88; no `mod.rs`.
- `src/foo.rs` with `src/foo/tests.rs`; `#[cfg(test)] mod tests;` in
  `foo.rs`; no `#[test]` inside implementation files.
- English everywhere, including comments.
- No `println!`/`eprintln!`; the lints deny them. Tracing arrives with M1.
- Commit messages: a one-line digest, an optional body of at most 400
  characters, no `Co-authored-by` trailer.
- Nothing from `.git-exclude/` is referenced by code or committed.

## 4. Review request package

When done, hand back one entry-point file under
`.git-exclude/reviews/001-workspace-layout/README.md` that contains:

1. Links to this handoff and to RFC 001.
2. The list of PRs with their commit ranges.
3. For every acceptance row of §1: how it was verified, with command output.
4. Anything that deviated from the RFC and why. A deviation is a question
   for the architect, not a decision.
