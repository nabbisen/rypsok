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
        "tools/check-limits",
        "cargo deny check advisories bans licenses sources",
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

/// REQ-RUST-002: CI builds and tests on Linux, macOS and Windows; the
/// release job (crates.io and binaries) is where aarch64 is added.
#[test]
fn req_rust_002_ci_matrix_covers_three_platforms() {
    let ci = read(".github/workflows/ci.yml");
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(ci.contains(os), "{os}");
    }
    assert!(ci.contains("cargo test --workspace"));
}

/// REQ-MAIN-005: every stage is testable without the network: recorded
/// fixtures exist for every driver, the hostile corpus exists, and the
/// mock server is a development dependency of both crates.
#[test]
fn req_main_005_fixtures_and_loopback_servers_exist() {
    for engine in ["wikipedia", "brave"] {
        for case in [
            "search-ok.json",
            "search-empty.json",
            "error-429.json",
            "error-malformed.json",
        ] {
            assert!(
                repository_root()
                    .join("crates/rypsok-core/fixtures")
                    .join(engine)
                    .join(case)
                    .is_file(),
                "{engine}/{case}"
            );
        }
    }
    assert!(
        repository_root()
            .join("crates/rypsok-core/fixtures/hostile")
            .read_dir()
            .unwrap()
            .count()
            > 5
    );
    assert!(read("crates/rypsok-core/Cargo.toml").contains("wiremock"));
    assert!(read("crates/rypsok/Cargo.toml").contains("wiremock"));
}

/// REQ-SEC-020 (RFC 018): every action a workflow uses is pinned to a
/// full commit hash, with the version it stands for as a comment, so
/// that a moved tag cannot run in this project's CI.
#[test]
fn req_sec_020_ci_actions_are_pinned() {
    let dir = repository_root().join(".github/workflows");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "yml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (number, line) in text.lines().enumerate() {
            let Some(rest) = line
                .trim_start()
                .strip_prefix("- uses: ")
                .or_else(|| line.trim_start().strip_prefix("uses: "))
            else {
                continue;
            };
            seen += 1;
            let (reference, comment) = rest.split_once('#').unwrap_or((rest, ""));
            let hash = reference.trim().rsplit('@').next().unwrap_or("");
            assert!(
                hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()),
                "{}:{}: `{}` is not pinned to a commit hash",
                path.display(),
                number + 1,
                rest.trim()
            );
            assert!(
                !comment.trim().is_empty(),
                "{}:{}: the pinned action needs its version as a comment",
                path.display(),
                number + 1
            );
        }
    }
    assert!(seen >= 15, "{seen} action references found");
}

/// REQ-DOC-001 (RFC 019 D7): every page of the book is in its summary, so
/// that no page is built without a way to reach it and no link in another
/// page leads to a file the book does not publish.
#[test]
fn req_doc_001_every_page_is_in_the_summary() {
    let src = repository_root().join("docs").join("src");
    let summary = read("docs/src/SUMMARY.md");
    let mut linked = std::collections::BTreeSet::new();
    let mut rest = summary.as_str();
    while let Some(at) = rest.find("](") {
        rest = &rest[at + 2..];
        let Some(end) = rest.find(')') else { break };
        let target = rest[..end].trim_start_matches("./");
        if target.ends_with(".md") {
            linked.insert(target.to_owned());
        }
        rest = &rest[end..];
    }
    assert!(
        linked.len() >= 20,
        "{} links found in SUMMARY.md",
        linked.len()
    );

    let mut pages = std::collections::BTreeSet::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                let relative = path
                    .strip_prefix(&src)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if relative != "SUMMARY.md" {
                    pages.insert(relative);
                }
            }
        }
    }
    let missing: Vec<_> = pages.difference(&linked).collect();
    assert!(missing.is_empty(), "pages not in SUMMARY.md: {missing:?}");
    let dangling: Vec<_> = linked.difference(&pages).collect();
    assert!(
        dangling.is_empty(),
        "SUMMARY.md links to no page: {dangling:?}"
    );
}
