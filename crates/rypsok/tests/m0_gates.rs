//! Tests of the M0 gates: the dependency rule, supply-chain hardening and
//! the CI workflow (RFC 001, RFC 002). They read the repository, so they run
//! from the workspace checkout.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/rypsok sits two levels below the repository root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// REQ-ARCH-005: the core crate has no protocol dependency. Checked
/// directly through `cargo tree` on every platform.
#[test]
fn req_arch_005_core_has_no_protocol_dependency() {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "tree",
            "-p",
            "rypsok-core",
            "-e",
            "normal",
            "--prefix",
            "none",
            "--all-features",
        ])
        .current_dir(repository_root())
        .output()
        .expect("cargo tree runs");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout);
    let offenders: Vec<&str> = tree
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|name| name.starts_with("rmcp") || name.contains("mcp"))
        .collect();
    assert!(
        offenders.is_empty(),
        "protocol crates in rypsok-core: {offenders:?}"
    );
}

/// REQ-ARCH-005: the dependency-rule script exists, is the CI gate, and
/// passes. The script itself is run where a POSIX shell exists.
#[test]
fn req_arch_005_dependency_rule_script_passes() {
    let script = repository_root()
        .join("tools")
        .join("check-core-dependencies");
    assert!(script.is_file(), "tools/check-core-dependencies is missing");
    assert!(read(".github/workflows/ci.yml").contains("tools/check-core-dependencies"));
    if cfg!(unix) {
        let status = Command::new("bash")
            .arg(&script)
            .current_dir(repository_root())
            .status()
            .expect("bash runs");
        assert!(status.success(), "tools/check-core-dependencies failed");
    }
}

/// REQ-SEC-020: the supply-chain policy exists with the allowed licenses, CI
/// runs it, and project code forbids `unsafe`.
#[test]
fn req_sec_020_supply_chain_policy_and_unsafe_forbidden() {
    let deny = read("deny.toml");
    for license in ["Apache-2.0", "MIT", "BSD-3-Clause"] {
        assert!(
            deny.contains(&format!("\"{license}\"")),
            "deny.toml allows {license}"
        );
    }
    assert!(deny.contains("unknown-registry = \"deny\""));
    assert!(
        read(".github/workflows/ci.yml").contains("cargo deny"),
        "CI runs cargo deny"
    );
    let workspace = read("Cargo.toml");
    assert!(
        workspace.contains("unsafe_code = \"forbid\""),
        "the workspace forbids unsafe code"
    );
    for crate_manifest in ["crates/rypsok-core/Cargo.toml", "crates/rypsok/Cargo.toml"] {
        assert!(
            read(crate_manifest).contains("[lints]\nworkspace = true"),
            "{crate_manifest} inherits the lints"
        );
    }
}

/// REQ-TEST-012: the CI workflow runs every gate of RFC 002 on every change.
#[test]
fn req_test_012_ci_workflow_lists_every_gate() {
    let ci = read(".github/workflows/ci.yml");
    for gate in [
        "cargo fmt --all --check",
        "cargo clippy --workspace --all-targets -- -D warnings",
        "cargo test --workspace",
        "tools/check-core-dependencies",
        "tools/check-requirements-coverage",
        "cargo deny check advisories licenses sources",
        "cargo doc --workspace --no-deps",
        "mdbook build docs",
    ] {
        assert!(ci.contains(gate), "ci.yml lacks the gate `{gate}`");
    }
    assert!(ci.contains("pull_request"), "ci.yml runs on pull requests");
    assert!(ci.contains("push"), "ci.yml runs on pushes");
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(ci.contains(os), "ci.yml runs on {os}");
    }
    assert!(ci.contains("1.88"), "ci.yml tests the MSRV");
    let lints = read("Cargo.toml");
    assert!(
        lints.contains("print_stdout = \"deny\""),
        "the lint that forbids printing to stdout is on"
    );
}
