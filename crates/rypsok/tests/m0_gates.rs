//! Tests of the M0 gates: the dependency rule, supply-chain hardening and
//! the CI workflow (RFC 001, RFC 002). They read the repository, so they run
//! from the workspace checkout.

use std::path::{Path, PathBuf};
use std::process::Command;

mod workflow_yaml;

use workflow_yaml::Yaml;

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

/// A workflow file, parsed.
fn workflow(name: &str) -> Yaml {
    workflow_yaml::parse(&read(&format!(".github/workflows/{name}")))
}

/// The four workflow files by name.
fn workflow_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(repository_root().join(".github/workflows"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".yml"))
        .collect();
    names.sort();
    names
}

/// The jobs of a workflow, by id.
fn jobs(workflow: &Yaml) -> &[(String, Yaml)] {
    workflow.get("jobs").expect("a workflow has jobs").entries()
}

/// Every line of every `run` of a workflow, trimmed.
fn run_lines(workflow: &Yaml) -> Vec<String> {
    workflow
        .find_all("run")
        .iter()
        .filter_map(|run| run.scalar())
        .flat_map(|text| text.lines().map(|line| line.trim().to_owned()))
        .collect()
}

/// `1.2` or `1.2.3`: a version, not a moving name.
fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// REQ-TEST-012: the CI workflow runs every gate of RFC 002 on every change.
/// The gates are looked for in the `run` lines of the parsed workflow, not in
/// its text, so a gate that is only mentioned in a comment does not count.
#[test]
fn req_test_012_ci_workflow_lists_every_gate() {
    let ci = workflow("ci.yml");
    let runs = run_lines(&ci);
    for gate in [
        "cargo fmt --all --check",
        "cargo clippy --workspace --all-targets --locked -- -D warnings",
        "cargo test --workspace --locked",
        "tools/check-core-dependencies",
        "tools/check-requirements-coverage",
        "tools/check-limits",
        "cargo deny check advisories bans licenses sources",
        "cargo doc --workspace --no-deps --locked",
        "mdbook build docs",
    ] {
        assert!(
            runs.iter().any(|line| line == gate),
            "ci.yml has no step that runs the gate `{gate}`"
        );
    }
    let on = ci.get("on").expect("ci.yml has triggers");
    assert!(
        on.get("pull_request").is_some(),
        "ci.yml runs on pull requests"
    );
    assert!(on.get("push").is_some(), "ci.yml runs on pushes");
    let matrix = ci.at(&["jobs", "test", "strategy", "matrix"]).unwrap();
    let os: Vec<&str> = matrix
        .get("os")
        .unwrap()
        .items()
        .iter()
        .filter_map(Yaml::scalar)
        .collect();
    for wanted in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(os.contains(&wanted), "ci.yml runs on {wanted}");
    }
    let msrv = matrix
        .get("include")
        .unwrap()
        .items()
        .iter()
        .any(|entry| entry.get("toolchain").and_then(Yaml::scalar) == Some("1.88"));
    assert!(msrv, "ci.yml tests the MSRV");
    let lints = read("Cargo.toml");
    assert!(
        lints.contains("print_stdout = \"deny\""),
        "the lint that forbids printing to stdout is on"
    );
}

/// RFC 020 (DEP-5, DEP-6): every workflow declares `permissions`, and at the
/// workflow level they only read; a job writes only what this table allows.
#[test]
fn req_sec_020_every_workflow_declares_least_permissions() {
    // (workflow, job, the permissions the job may write)
    const MAY_WRITE: [(&str, &str, &[&str]); 2] = [
        ("docs.yml", "deploy", &["pages", "id-token"]),
        ("release.yml", "attach", &["contents"]),
    ];
    for name in workflow_names() {
        let file = workflow(&name);
        let top = file
            .get("permissions")
            .unwrap_or_else(|| panic!("{name} declares no top-level `permissions`"));
        assert_eq!(
            top.get("contents").and_then(Yaml::scalar),
            Some("read"),
            "{name}: the workflow reads the contents and nothing else by default"
        );
        for (permission, level) in top.entries() {
            assert_eq!(
                level.scalar(),
                Some("read"),
                "{name}: `{permission}` is granted to the whole workflow"
            );
        }
        for (job, body) in jobs(&file) {
            let allowed: &[&str] = MAY_WRITE
                .iter()
                .find(|(w, j, _)| *w == name && j == job)
                .map_or(&[], |(_, _, writes)| writes);
            let granted: Vec<&str> = body.get("permissions").map_or_else(Vec::new, |p| {
                p.entries()
                    .iter()
                    .filter(|(_, level)| level.scalar() == Some("write"))
                    .map(|(permission, _)| permission.as_str())
                    .collect()
            });
            for permission in &granted {
                assert!(
                    allowed.contains(permission),
                    "{name}: job `{job}` may not write `{permission}`"
                );
            }
            for permission in allowed {
                assert!(
                    granted.contains(permission),
                    "{name}: job `{job}` should write `{permission}` and holds no such grant"
                );
            }
        }
    }
}

/// RFC 020 (DEP-7): a failed scheduled run fails (GitHub notifies the owner)
/// and a Brave check that cannot run says so.
#[test]
fn req_test_003_scheduled_checks_fail_loudly_and_say_when_they_skip() {
    let scheduled = workflow("scheduled.yml");
    assert!(
        scheduled.find_all("continue-on-error").is_empty(),
        "scheduled.yml: a failed run must fail"
    );
    let ids: Vec<&str> = jobs(&scheduled).iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["fuzz", "live-checks"], "two independent jobs");
    for (id, body) in jobs(&scheduled) {
        assert!(body.get("needs").is_none(), "job `{id}` waits for no other");
    }
    let steps = scheduled.at(&["jobs", "live-checks", "steps"]).unwrap();
    let position = |needle: &str| {
        steps.items().iter().position(|step| {
            step.get("run")
                .and_then(Yaml::scalar)
                .is_some_and(|run| run.contains(needle))
        })
    };
    let warns = position("::warning").expect("a step warns about a missing key");
    let live = position("--ignored").expect("the live checks run");
    assert!(warns < live, "the warning comes before the checks");
    let step = &steps.items()[warns];
    let run = step.get("run").and_then(Yaml::scalar).unwrap();
    assert!(
        run.contains("-z \"$BRAVE_API_KEY\""),
        "it tests for an empty key"
    );
    assert!(
        run.contains("Brave check skipped: no key") && run.contains("GITHUB_STEP_SUMMARY"),
        "it writes the skip to the step summary"
    );
}

/// RFC 020 (DEP-8, TST-14): each fuzz target runs 240 s and the job's limit
/// follows the number of targets (four minutes each and fifteen for the
/// rest). The job runs every target `cargo fuzz list` names, so a target
/// added to `fuzz/Cargo.toml` is run without a change to the workflow.
#[test]
fn req_sec_023_every_fuzz_target_runs_240_s_within_a_matching_limit() {
    let manifest = read("fuzz/Cargo.toml");
    let declared = manifest.matches("[[bin]]").count();
    let files = std::fs::read_dir(repository_root().join("fuzz/fuzz_targets"))
        .unwrap()
        .count();
    assert_eq!(
        declared, files,
        "every target file is a [[bin]] of fuzz/Cargo.toml"
    );
    let scheduled = workflow("scheduled.yml");
    let fuzz = scheduled.at(&["jobs", "fuzz"]).unwrap();
    let limit: usize = fuzz
        .get("timeout-minutes")
        .and_then(Yaml::scalar)
        .expect("the fuzz job has a timeout")
        .parse()
        .unwrap();
    assert_eq!(
        limit,
        declared * 4 + 15,
        "timeout-minutes = targets x 4 + 15"
    );
    assert!(
        run_lines(&scheduled)
            .iter()
            .any(|line| line.contains("-max_total_time=240")),
        "each target runs 240 s"
    );
}

/// REQ-SEC-005 (RFC 020, TST-7): every request `rypsok-core` makes is sent on
/// a client that `egress.rs` built. In non-test code, no HTTP client is built
/// anywhere else (`Client::builder`, `ClientBuilder`, `Client::new`,
/// `reqwest::get`, the blocking client), and `.send()` appears in the five
/// places that use `engine_client` or `agent_client`, once each; a new send
/// site is a change to this list, which is a change a reviewer reads.
#[test]
fn req_sec_005_every_request_is_sent_on_a_client_egress_built() {
    let root = repository_root();
    let src = root.join("crates/rypsok-core/src");
    let mut files = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    assert!(files.len() > 40, "{} source files found", files.len());

    // File -> (the egress constructor the file uses or is handed a client
    // from, the number of `.send()` it holds).
    let allowed: [(&str, &str, usize); 5] = [
        ("engine/drivers/wikipedia.rs", "egress::engine_client(", 1),
        ("engine/drivers/brave.rs", "egress::engine_client(", 1),
        ("engine/drivers/duckduckgo.rs", "egress::engine_client(", 1),
        ("fetch.rs", "agent_client", 1),
        // The robots.txt read is handed the agent client of the fetch
        // service, which `fetch.rs` builds with `agent_client`.
        ("fetch/robots.rs", "client: &reqwest::Client", 1),
    ];
    let builders = [
        "Client::builder",
        "ClientBuilder",
        "Client::new",
        "reqwest::get(",
        "reqwest::blocking",
    ];
    let mut problems = Vec::new();
    let mut sends = 0;
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name == "tests.rs" || name.starts_with("tests_") {
            continue;
        }
        let relative = path
            .strip_prefix(&src)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&path).unwrap();
        let code = without_test_module(&text);
        if relative != "egress.rs" {
            for builder in builders {
                if code.contains(builder) {
                    problems.push(format!("{relative}: builds a client with `{builder}`"));
                }
            }
        }
        let count = code.matches(".send()").count();
        sends += count;
        match allowed.iter().find(|(file, _, _)| *file == relative) {
            Some((_, constructor, expected)) => {
                if count != *expected {
                    problems.push(format!(
                        "{relative}: {count} `.send()`, expected {expected}"
                    ));
                }
                if !code.contains(constructor) {
                    problems.push(format!("{relative}: does not use `{constructor}`"));
                }
            }
            None if count > 0 => {
                problems.push(format!("{relative}: {count} `.send()` in an unlisted file"))
            }
            None => {}
        }
    }
    assert_eq!(sends, 5, "the number of send sites");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// RFC 020 (DEP-3, DEP-5, DEP-13): no tool is installed at a moving version
/// and the release compiler is a named one.
#[test]
fn req_sec_020_tools_and_the_release_compiler_are_pinned() {
    for name in workflow_names() {
        let file = workflow(&name);
        for with in file.find_all("with") {
            for (key, value) in with.entries() {
                let Some(text) = value.scalar() else { continue };
                match key.as_str() {
                    "tool" => {
                        for tool in text.split(',').map(str::trim) {
                            let (_, version) = tool
                                .split_once('@')
                                .unwrap_or_else(|| panic!("{name}: tool `{tool}` has no @version"));
                            assert!(is_version(version), "{name}: tool `{tool}` is not pinned");
                        }
                    }
                    "mdbook-version" => {
                        assert!(
                            is_version(text),
                            "{name}: mdbook-version `{text}` is not pinned"
                        );
                    }
                    _ => assert_ne!(text, "latest", "{name}: `{key}: latest`"),
                }
            }
        }
    }
    let release = workflow("release.yml");
    let toolchains = release.find_all("toolchain");
    assert!(!toolchains.is_empty(), "the release names a toolchain");
    for toolchain in toolchains {
        let text = toolchain.scalar().unwrap();
        assert!(
            is_version(text),
            "release.yml builds with `{text}`, not a named compiler"
        );
    }
}

/// RFC 020 (DEP-9): the cargo commands that build, test, lint or document
/// use the lock file, in every workflow.
#[test]
fn req_test_012_cargo_commands_of_the_workflows_are_locked() {
    let mut seen = 0;
    for name in workflow_names() {
        for line in run_lines(&workflow(&name)) {
            let mut words = line.split_whitespace();
            if words.next() != Some("cargo") {
                continue;
            }
            let subcommand = words.find(|word| !word.starts_with('+'));
            if matches!(subcommand, Some("build" | "test" | "clippy" | "doc")) {
                seen += 1;
                assert!(
                    line.contains("--locked"),
                    "{name}: `{line}` is not --locked"
                );
            }
        }
    }
    assert!(seen >= 5, "{seen} cargo commands found");
}

/// RFC 020 (DEP-4): the Windows release build links reproducibly.
#[test]
fn req_sec_020_the_windows_release_build_links_with_brepro() {
    let release = workflow("release.yml");
    let build = release.at(&["jobs", "build"]).unwrap();
    let windows = build
        .at(&["strategy", "matrix", "include"])
        .unwrap()
        .items()
        .iter()
        .find(|entry| entry.get("os").and_then(Yaml::scalar) == Some("windows-latest"))
        .expect("a Windows entry");
    assert!(
        windows
            .get("rustflags")
            .and_then(Yaml::scalar)
            .is_some_and(|flags| flags.contains("link-arg=/Brepro")),
        "the Windows entry sets /Brepro"
    );
    assert_eq!(
        build.at(&["env", "RUSTFLAGS"]).and_then(Yaml::scalar),
        Some("${{ matrix.rustflags }}"),
        "the build takes its flags from the matrix entry"
    );
}

/// RFC 020 (handoff 020o): the reader of the workflow files reads what they
/// use, and refuses what it does not know.
#[test]
fn req_sec_020_the_workflow_reader_reads_block_yaml_and_refuses_the_rest() {
    let text = "\
# comment
on:
  push:
    tags: [\"[0-9]*\", other]
jobs:
  a:
    steps:
      - uses: x/y@abc # v1
        with:
          tool: t@1.2.3
      - name: Two
        run: |
          echo one # kept
          echo two
    list:
    - plain
    - key: value
";
    let yaml = workflow_yaml::parse(text);
    let tags: Vec<&str> = yaml
        .at(&["on", "push", "tags"])
        .unwrap()
        .items()
        .iter()
        .filter_map(Yaml::scalar)
        .collect();
    assert_eq!(tags, ["[0-9]*", "other"]);
    let steps = yaml.at(&["jobs", "a", "steps"]).unwrap().items();
    assert_eq!(steps[0].get("uses").and_then(Yaml::scalar), Some("x/y@abc"));
    assert_eq!(
        steps[0].at(&["with", "tool"]).and_then(Yaml::scalar),
        Some("t@1.2.3")
    );
    assert_eq!(
        steps[1].get("run").and_then(Yaml::scalar),
        Some("echo one # kept\necho two")
    );
    let list = yaml.at(&["jobs", "a", "list"]).unwrap().items();
    assert_eq!(list[0].scalar(), Some("plain"));
    assert_eq!(list[1].get("key").and_then(Yaml::scalar), Some("value"));
    for name in workflow_names() {
        assert!(
            !jobs(&workflow(&name)).is_empty(),
            "{name} has jobs once parsed"
        );
    }
    let refused = std::panic::catch_unwind(|| workflow_yaml::parse("a: &anchor 1\n"));
    assert!(refused.is_err(), "an anchor is refused, not read");
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

/// The `pub struct` and `pub enum` items of a Rust source file, with whether
/// the block of doc comments and attributes above each carries
/// `#[non_exhaustive]`. Only declarations that start a line are read (a
/// `pub(crate)` item is not public).
fn public_types(source: &str) -> Vec<(String, bool)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed
            .strip_prefix("pub struct ")
            .or_else(|| trimmed.strip_prefix("pub enum "))
        else {
            continue;
        };
        if line.len() != trimmed.len() {
            continue; // indented: inside a module or an impl, not a top-level item
        }
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let mut exhaustive_marker = false;
        for above in lines[..index].iter().rev() {
            if above.trim().is_empty() {
                break;
            }
            if above.trim() == "#[non_exhaustive]" {
                exhaustive_marker = true;
            }
        }
        found.push((name, exhaustive_marker));
    }
    found
}

/// The entries of the first fenced block after the heading that contains
/// `heading` in a page: the name before a `:` or a space, per line.
fn page_list(page: &str, heading: &str) -> Vec<String> {
    let after = page
        .split_once(heading)
        .unwrap_or_else(|| panic!("the page has no heading `{heading}`"))
        .1;
    let block = after
        .split("```")
        .nth(1)
        .unwrap_or_else(|| panic!("no fenced list after `{heading}`"));
    block
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("text"))
        .map(|l| {
            l.split([':', ' '])
                .next()
                .expect("a line has a first part")
                .to_owned()
        })
        .collect()
}

/// REQ-ARCH-007 and RFC 016 (RFC 020, DAT-5): the stability page lists the
/// public types of the frozen surface of `rypsok-core` (the facade and the
/// `model` module) in two fenced lists, and the source agrees with them. The
/// types that carry `#[non_exhaustive]` are exactly the first list; every
/// other public type of the surface is on the second, "deliberately
/// exhaustive", with a reason. A type added to `model` fails this test until
/// it is on one of the lists.
#[test]
fn req_arch_007_stability_page_lists_match_the_source() {
    let mut sources = vec![read("crates/rypsok-core/src/lib.rs")];
    sources.push(read("crates/rypsok-core/src/model.rs"));
    let model_dir = repository_root().join("crates/rypsok-core/src/model");
    for entry in std::fs::read_dir(&model_dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.ends_with(".rs") && name != "tests.rs" {
            sources.push(read(&format!("crates/rypsok-core/src/model/{name}")));
        }
    }
    let mut all = std::collections::BTreeSet::new();
    let mut marked = std::collections::BTreeSet::new();
    for source in &sources {
        for (name, non_exhaustive) in public_types(source) {
            all.insert(name.clone());
            if non_exhaustive {
                marked.insert(name);
            }
        }
    }
    assert!(all.len() >= 30, "{} public types found", all.len());
    assert!(all.contains("Rypsok") && all.contains("SearchResponse"));

    let page = read("docs/src/reference/stability.md");
    let listed_marked: std::collections::BTreeSet<String> =
        page_list(&page, "### Types marked `#[non_exhaustive]`")
            .into_iter()
            .collect();
    let listed_exhaustive: std::collections::BTreeSet<String> =
        page_list(&page, "### Types that are deliberately exhaustive")
            .into_iter()
            .collect();
    assert_eq!(
        listed_marked, marked,
        "the page's list of `#[non_exhaustive]` types equals the source"
    );
    let unmarked: std::collections::BTreeSet<String> = all.difference(&marked).cloned().collect();
    assert_eq!(
        listed_exhaustive, unmarked,
        "every other public type of the surface is on the exhaustive list, and nothing else"
    );
    for named in [
        "Outcome",
        "ErrorKind",
        "CallError",
        "EngineState",
        "TimeRange",
    ] {
        assert!(
            listed_exhaustive.contains(named),
            "{named} is deliberately exhaustive (RFC 016 amendment)"
        );
    }
    // Each exhaustive entry says why.
    let exhaustive_block = page
        .split_once("### Types that are deliberately exhaustive")
        .unwrap()
        .1
        .split("```")
        .nth(1)
        .unwrap();
    for line in exhaustive_block
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("text"))
    {
        let (_, reason) = line
            .split_once(": ")
            .unwrap_or_else(|| panic!("`{line}` gives no reason after a colon"));
        assert!(reason.len() >= 10, "`{line}`: the reason is too short");
    }
}

/// RFC 020 (DAT-5): the scanner of the test above reads what it must.
#[test]
fn req_arch_007_the_type_scanner_reads_attributes_and_ignores_the_rest() {
    let source = "\
/// Doc.
#[derive(
    Debug,
    Clone,
)]
#[non_exhaustive]
pub struct Marked {
    a: u8,
}

/// Doc.
#[derive(Debug)]
pub enum Open {
    A,
}

#[non_exhaustive]

pub struct SeparatedByABlankLine;

pub(crate) struct Hidden;

mod inner {
    pub struct Indented;
}
";
    assert_eq!(
        public_types(source),
        vec![
            ("Marked".to_owned(), true),
            ("Open".to_owned(), false),
            ("SeparatedByABlankLine".to_owned(), false),
        ]
    );
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

/// The code of a source file before its test module: everything up to the
/// first `#[cfg(test)]` that is followed by `mod tests`. A `#[cfg(test)]`
/// helper function earlier in the file is not the end of the code.
fn without_test_module(text: &str) -> &str {
    let mut from = 0;
    while let Some(at) = text[from..].find("#[cfg(test)]") {
        let start = from + at;
        let after = &text[start + "#[cfg(test)]".len()..];
        if after.trim_start().starts_with("mod tests") {
            return &text[..start];
        }
        from = start + 1;
    }
    text
}

/// REQ-SEC-014: the scan stops at the test module, not at the first
/// `#[cfg(test)]`: code after a test-only helper is still scanned.
#[test]
fn req_sec_014_scan_stops_at_the_test_module_only() {
    let text = "fn a() { m.lock().unwrap(); }\n#[cfg(test)]\nfn helper() {}\nfn b() { m.read().expect(\"x\"); }\n#[cfg(test)]\nmod tests;\nfn c() { m.write().unwrap(); }\n";
    let code = without_test_module(text);
    assert!(code.contains("fn a()"));
    assert!(code.contains("fn b()"), "code after a helper is scanned");
    assert!(!code.contains("fn c()"), "the test module is not scanned");
    let inline = "fn a() {}\n#[cfg(test)]\nmod tests {\n fn t() {}\n}\n";
    assert_eq!(without_test_module(inline), "fn a() {}\n");
    assert_eq!(without_test_module("fn a() {}\n"), "fn a() {}\n");
}

/// REQ-SEC-014 with RFC 020 D3, handoff 020j (NET-13): a `std::sync` lock is never taken
/// with `.expect(` or `.unwrap()` in non-test source, so that a panic while one is held
/// cannot make every later call panic. The code goes through the `sync`
/// helpers (`rypsok-core`) or recovers the guard itself. `cache.rs` is
/// exempt: it disables itself on a poisoned lock (RFC 014, REQ-CACHE-006).
/// Test code is exempt. The scan joins the lines of a call chain first, so a
/// `.lock()` and an `.expect(` on separate lines are found.
#[test]
fn req_sec_014_no_expect_on_a_standard_library_lock() {
    let root = repository_root();
    let mut files = Vec::new();
    for krate in ["rypsok-core", "rypsok"] {
        let mut stack = vec![root.join("crates").join(krate).join("src")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    files.push(path);
                }
            }
        }
    }
    assert!(files.len() > 50, "{} source files found", files.len());

    let exempt = root.join("crates/rypsok-core/src/cache.rs");
    let mut offenders = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path == exempt || name == "tests.rs" || name.starts_with("tests_") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let code = without_test_module(&text);
        let joined: String = code
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("//"))
            .collect::<Vec<_>>()
            .join("");
        for call in [".lock()", ".read()", ".write()"] {
            for panicking in [".expect(", ".unwrap()"] {
                let pattern = format!("{call}{panicking}");
                if joined.contains(&pattern) {
                    offenders.push(format!(
                        "{}: `{pattern}`",
                        path.strip_prefix(&root).unwrap().display()
                    ));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "locks taken with expect or unwrap (use the sync helpers):\n{}",
        offenders.join("\n")
    );
}
