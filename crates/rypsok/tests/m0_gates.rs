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
    const MAY_WRITE: [(&str, &str, &[&str]); 3] = [
        ("docs.yml", "deploy", &["pages", "id-token"]),
        ("release.yml", "attach", &["contents"]),
        // The one job that may sign (RFC 020, D-16): it runs no build code.
        ("release.yml", "checksums", &["id-token", "attestations"]),
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

/// RFC 020 (DEP-7; D-23): the external checks (the live driver checks and
/// the advisory check) run on dispatch or when another workflow calls them,
/// never on a timer; a failed run fails (GitHub tells whoever started it) and
/// a Brave check that cannot run says so.
#[test]
fn req_test_003_external_checks_run_on_demand_fail_loudly_and_say_when_they_skip() {
    let external = workflow("external-checks.yml");
    let triggers: Vec<&str> = external
        .get("on")
        .expect("triggers")
        .entries()
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        triggers,
        ["workflow_dispatch", "workflow_call"],
        "external-checks.yml runs on dispatch and on call, nothing else"
    );
    assert!(
        external.find_all("continue-on-error").is_empty(),
        "external-checks.yml: a failed run must fail"
    );
    let ids: Vec<&str> = jobs(&external).iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["live-checks", "advisories"], "two independent jobs");
    for (id, body) in jobs(&external) {
        assert!(body.get("needs").is_none(), "job `{id}` waits for no other");
        assert!(
            body.get("timeout-minutes").is_some(),
            "job `{id}` has a time limit"
        );
    }
    let steps = external.at(&["jobs", "live-checks", "steps"]).unwrap();
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
    // The advisory check: cargo-deny pinned as in ci.yml, advisories only,
    // on a fresh database.
    let advisories = run_lines(&external);
    assert!(
        advisories
            .iter()
            .any(|line| line == "cargo deny check advisories"),
        "the advisory job runs `cargo deny check advisories`"
    );
    let ci = workflow("ci.yml");
    let ci_tools: Vec<&str> = ci
        .find_all("tool")
        .iter()
        .filter_map(|t| t.scalar())
        .collect();
    let deny = ci_tools
        .iter()
        .find(|tool| tool.starts_with("cargo-deny@"))
        .expect("ci.yml pins cargo-deny");
    assert!(
        external
            .find_all("tool")
            .iter()
            .any(|tool| tool.scalar() == Some(deny)),
        "external-checks.yml pins cargo-deny as ci.yml does ({deny})"
    );
}

/// REQ-TEST-019 and REQ-SEC-023 (RFC 020, D-23): no workflow has a timer.
/// Checks run per change (the handoff's own mutation testing and fuzzing),
/// before a release (the release checks) and on demand, never on a schedule.
#[test]
fn req_test_019_no_workflow_runs_on_a_timer() {
    for name in workflow_names() {
        let file = workflow(&name);
        assert!(
            file.get("on")
                .is_some_and(|on| on.get("schedule").is_none()),
            "{name} has an `on.schedule`"
        );
        assert!(
            file.find_all("schedule").is_empty() && file.find_all("cron").is_empty(),
            "{name} mentions a schedule or a cron"
        );
    }
    assert!(
        !repository_root()
            .join(".github/workflows/scheduled.yml")
            .exists(),
        "scheduled.yml is deleted"
    );
}

/// The release checks workflow, and what the tests below read from it.
fn release_checks() -> Yaml {
    workflow("release-checks.yml")
}

/// The matrix of a job.
fn matrix_of<'a>(workflow: &'a Yaml, job: &str) -> &'a Yaml {
    workflow
        .at(&["jobs", job, "strategy", "matrix"])
        .unwrap_or_else(|| panic!("job `{job}` has a matrix"))
}

/// The scalars of a list under a key.
fn list_of<'a>(node: &'a Yaml, key: &str) -> Vec<&'a str> {
    node.get(key)
        .unwrap_or_else(|| panic!("no `{key}`"))
        .items()
        .iter()
        .filter_map(Yaml::scalar)
        .collect()
}

/// The names of the `[[bin]]` targets of `fuzz/Cargo.toml`.
fn fuzz_targets() -> Vec<String> {
    let manifest = read("fuzz/Cargo.toml");
    let mut names = Vec::new();
    let mut in_bin = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_bin = line == "[[bin]]";
        } else if in_bin && let Some(rest) = line.strip_prefix("name = ") {
            names.push(rest.trim_matches('"').to_owned());
        }
    }
    names.sort();
    names
}

/// REQ-TEST-019 (RFC 020, D-23): the release checks are one manual workflow
/// of independent jobs (only the comparison of the two builds waits), none
/// of which may fail silently or run unbounded, with the least permissions.
#[test]
fn req_test_019_the_release_checks_are_manual_independent_and_bounded() {
    let file = release_checks();
    let triggers: Vec<&str> = file
        .get("on")
        .expect("triggers")
        .entries()
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(triggers, ["workflow_dispatch"], "dispatched by hand only");
    assert_eq!(
        file.at(&["concurrency", "group"]).and_then(Yaml::scalar),
        Some("release-checks")
    );
    assert_eq!(
        file.at(&["concurrency", "cancel-in-progress"])
            .and_then(Yaml::scalar),
        Some("false"),
        "a second dispatch does not cancel the run under way"
    );
    assert!(
        file.find_all("continue-on-error").is_empty(),
        "no job is allowed to fail"
    );
    let ids: Vec<&str> = jobs(&file).iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "reproducible",
            "reproducible-compare",
            "fuzz",
            "mutants",
            "external"
        ]
    );
    for (id, body) in jobs(&file) {
        if id == "reproducible-compare" {
            assert_eq!(
                body.get("needs").and_then(Yaml::scalar),
                Some("reproducible"),
                "the comparison waits for the builds"
            );
        } else {
            assert!(body.get("needs").is_none(), "job `{id}` waits for no other");
        }
        if id == "external" {
            // A job that calls a workflow cannot carry a time limit of its
            // own; the called jobs have theirs (checked above).
            assert_eq!(
                body.get("uses").and_then(Yaml::scalar),
                Some("./.github/workflows/external-checks.yml")
            );
            let secrets: Vec<(&str, Option<&str>)> = body
                .get("secrets")
                .expect("the Brave key is passed")
                .entries()
                .iter()
                .map(|(name, value)| (name.as_str(), value.scalar()))
                .collect();
            assert_eq!(
                secrets,
                [("BRAVE_API_KEY", Some("${{ secrets.BRAVE_API_KEY }}"))],
                "exactly the Brave key reaches the live check, by name"
            );
        } else {
            assert!(
                body.get("timeout-minutes").is_some(),
                "job `{id}` has a time limit"
            );
        }
    }
}

/// REQ-TEST-019 and REQ-SEC-020 (review of 020w, C1): a called workflow gets
/// the one secret it needs, by name. `secrets: inherit` would hand it every
/// secret of the repository, today and whenever one is added.
#[test]
fn req_test_019_a_called_workflow_gets_only_the_secret_it_needs() {
    for name in workflow_names() {
        assert!(
            !read(&format!(".github/workflows/{name}")).contains("secrets: inherit"),
            "{name} passes every secret"
        );
    }
    let external = workflow("external-checks.yml");
    let declared = external
        .at(&["on", "workflow_call", "secrets"])
        .expect("external-checks.yml declares the secrets it takes");
    let names: Vec<&str> = declared.entries().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["BRAVE_API_KEY"]);
    assert_eq!(
        declared
            .at(&["BRAVE_API_KEY", "required"])
            .and_then(Yaml::scalar),
        Some("false"),
        "a release can be checked without the key (the check says it skipped)"
    );
}

/// REQ-TEST-019 (review of 020w, C4): every fuzz target the release checks
/// run starts from a committed corpus: 300 s from nothing on a structured
/// format is spent finding the format. The seeds are `seed-*` files.
#[test]
fn req_test_019_every_fuzz_target_has_committed_seeds() {
    let file = release_checks();
    for target in list_of(matrix_of(&file, "fuzz"), "target") {
        let dir = repository_root().join("fuzz/corpus").join(target);
        let seeds = std::fs::read_dir(&dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.file_name().to_string_lossy().starts_with("seed-"))
                    .count()
            })
            .unwrap_or(0);
        assert!(seeds >= 1, "fuzz/corpus/{target} has no seed-* file");
    }
}

/// REQ-TEST-019: the fuzz job runs every target of `fuzz/Cargo.toml`, from a
/// static list, one job each, 300 s, from its corpus; a target added without
/// the workflow fails here.
#[test]
fn req_test_019_the_release_checks_fuzz_every_target_for_300_s() {
    let file = release_checks();
    let matrix = matrix_of(&file, "fuzz");
    let mut listed: Vec<String> = list_of(matrix, "target")
        .into_iter()
        .map(str::to_owned)
        .collect();
    listed.sort();
    assert_eq!(
        listed,
        fuzz_targets(),
        "the matrix lists exactly the [[bin]] targets of fuzz/Cargo.toml"
    );
    let strategy = file.at(&["jobs", "fuzz", "strategy"]).unwrap();
    assert_eq!(
        strategy.get("max-parallel").and_then(Yaml::scalar),
        Some("4")
    );
    assert_eq!(
        strategy.get("fail-fast").and_then(Yaml::scalar),
        Some("false")
    );
    assert_eq!(
        file.at(&["jobs", "fuzz", "timeout-minutes"])
            .and_then(Yaml::scalar),
        Some("20")
    );
    let lines = run_lines(&file);
    assert!(
        lines.iter().any(|l| l.contains("-max_total_time=300")),
        "each target runs 300 s"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("cargo +nightly fuzz build") && l.contains("${{ matrix.target }}")),
        "each job builds only its target"
    );
    let uploads = file
        .at(&["jobs", "fuzz", "steps"])
        .unwrap()
        .items()
        .iter()
        .any(|step| {
            step.get("if").and_then(Yaml::scalar) == Some("failure()")
                && step
                    .at(&["with", "path"])
                    .and_then(Yaml::scalar)
                    .is_some_and(|path| path.contains("fuzz/artifacts"))
        });
    assert!(uploads, "the artifacts of a failed fuzz job are uploaded");
}

/// REQ-TEST-019: the mutation job names the six security modules, runs the
/// pinned tool with two jobs and the committed exclusion list (read by
/// default from `.cargo/mutants.toml`), checks the disk first, and is red
/// only for a missed mutant.
#[test]
fn req_test_019_the_release_checks_mutate_the_six_security_modules() {
    let file = release_checks();
    let matrix = matrix_of(&file, "mutants");
    let names: Vec<&str> = matrix
        .get("include")
        .unwrap()
        .items()
        .iter()
        .filter_map(|entry| entry.get("name").and_then(Yaml::scalar))
        .collect();
    assert_eq!(
        names,
        [
            "egress-policy",
            "privacy",
            "boundary",
            "robots",
            "http",
            "sessions"
        ]
    );
    let files: String = matrix
        .get("include")
        .unwrap()
        .items()
        .iter()
        .filter_map(|entry| entry.get("files").and_then(Yaml::scalar))
        .collect::<Vec<_>>()
        .join(" ");
    for needed in [
        "crates/rypsok-core/src/egress/policy.rs",
        "crates/rypsok-core/src/privacy.rs",
        "crates/rypsok-core/src/boundary.rs",
        "crates/rypsok-core/src/fetch/robots.rs",
        "crates/rypsok/src/mcp/http.rs",
        "crates/rypsok/src/mcp/http/detect.rs",
        "crates/rypsok/src/mcp/http/sessions.rs",
    ] {
        assert!(
            files.contains(needed),
            "the matrix does not mutate {needed}"
        );
        assert!(repository_root().join(needed).exists(), "{needed}");
    }
    let strategy = file.at(&["jobs", "mutants", "strategy"]).unwrap();
    assert_eq!(
        strategy.get("max-parallel").and_then(Yaml::scalar),
        Some("3")
    );
    assert_eq!(
        strategy.get("fail-fast").and_then(Yaml::scalar),
        Some("false")
    );
    assert_eq!(
        file.at(&["jobs", "mutants", "timeout-minutes"])
            .and_then(Yaml::scalar),
        Some("90")
    );
    let lines = run_lines(&file).join("\n");
    assert!(lines.contains("df -h"), "the disk is checked first");
    // A full disk makes a mutant's build or test fail, which the tool counts
    // as caught: the run must say the counts cannot be trusted (review of
    // 020w, C2). The script checks the logs, and shows the disk again.
    let mutate = file
        .at(&["jobs", "mutants", "steps"])
        .unwrap()
        .items()
        .iter()
        .find(|step| step.get("name").and_then(Yaml::scalar) == Some("Mutate"))
        .and_then(|step| step.get("run").and_then(Yaml::scalar))
        .expect("a step named Mutate");
    assert!(
        mutate.contains("No space left on device"),
        "the script fails a run in which the disk filled"
    );
    assert!(
        mutate.contains("df -h"),
        "the disk is shown after the run too"
    );
    assert!(
        mutate.find("No space left on device") < mutate.find("case \"$code\""),
        "the disk check comes before the exit-code case"
    );
    assert!(
        lines.contains("cargo mutants --jobs 2"),
        "two jobs at a time"
    );
    assert!(
        !lines.contains("--config"),
        "the exclusion list is read from .cargo/mutants.toml, the tool's default"
    );
    assert!(
        read(".cargo/mutants.toml").contains("exclude_re"),
        "the committed exclusion list exists"
    );
    // Red only for a missed mutant: exit 2 fails, 3 (timeouts) does not.
    assert!(
        lines.contains("2)"),
        "an exit code of 2 (a missed mutant) fails"
    );
    let pinned = run_lines(&file)
        .iter()
        .any(|line| line == "cargo install cargo-mutants --version 27.1.0 --locked");
    assert!(
        pinned,
        "cargo-mutants is installed at the version the list was made with, locked"
    );
}

/// REQ-TEST-019: a runner pool is shared, so at most twelve release-check
/// jobs run at once (CI's jobs are not queued behind them): the biggest
/// matrices' `max-parallel` and the jobs of the called workflow add up to
/// twelve or fewer, and every matrix sets one.
#[test]
fn req_test_019_the_release_checks_leave_room_for_ci() {
    let file = release_checks();
    let mut total = 0;
    for job in ["reproducible", "fuzz", "mutants"] {
        let parallel: usize = file
            .at(&["jobs", job, "strategy", "max-parallel"])
            .and_then(Yaml::scalar)
            .unwrap_or_else(|| panic!("job `{job}` sets max-parallel"))
            .parse()
            .unwrap();
        total += parallel;
    }
    // The jobs of external-checks.yml run beside them.
    total += jobs(&workflow("external-checks.yml")).len();
    assert!(total <= 12, "{total} release-check jobs at once");
    // The reason is said at the top of the file.
    let text = read(".github/workflows/release-checks.yml");
    let head = text.split("\non:").next().unwrap();
    assert!(
        head.contains("12") && head.contains("shared"),
        "the file says why the jobs are capped"
    );
    for (id, body) in jobs(&file) {
        if let Some(strategy) = body.get("strategy") {
            assert!(
                strategy.get("max-parallel").is_some(),
                "job `{id}` sets max-parallel"
            );
        }
    }
}

/// REQ-TEST-019: both builds of every release target are built by the same
/// action as the release itself, uploaded under their own names and compared
/// by the SHA-256 of the unpacked `rypsok` binary, not of the archive.
#[test]
fn req_test_019_the_reproducibility_check_builds_twice_and_compares_the_binaries() {
    let file = release_checks();
    let matrix = matrix_of(&file, "reproducible");
    assert_eq!(list_of(matrix, "build"), ["a", "b"]);
    let mut targets = list_of(matrix, "target");
    targets.sort_unstable();
    let release = workflow("release.yml");
    let mut released: Vec<&str> = release
        .at(&["jobs", "build", "strategy", "matrix", "include"])
        .unwrap()
        .items()
        .iter()
        .filter_map(|entry| entry.get("target").and_then(Yaml::scalar))
        .collect();
    released.sort_unstable();
    assert_eq!(
        targets, released,
        "the release targets are the checked ones"
    );
    let uses_action = file
        .at(&["jobs", "reproducible", "steps"])
        .unwrap()
        .items()
        .iter()
        .any(|step| {
            step.get("uses").and_then(Yaml::scalar) == Some("./.github/actions/build-release")
        });
    assert!(uses_action, "the builds use the release's own action");
    let uploads = file
        .at(&["jobs", "reproducible", "steps"])
        .unwrap()
        .items()
        .iter()
        .any(|step| {
            step.at(&["with", "name"]).and_then(Yaml::scalar)
                == Some("repro-${{ matrix.target }}-${{ matrix.build }}")
        });
    assert!(uploads, "each build is uploaded as repro-<target>-<build>");
    let compare = run_lines(&file).join("\n");
    assert!(compare.contains("sha256sum"), "the binaries are hashed");
    assert!(
        compare.contains("GITHUB_STEP_SUMMARY"),
        "a table is written"
    );
}

/// REQ-TEST-019 (RFC 020, D-23): the release compiler is named in one place,
/// the build action; the release workflow and the release checks both build
/// through it, and no workflow names that compiler itself.
#[test]
fn req_test_019_the_release_compiler_is_named_in_one_place() {
    let action = read(".github/actions/build-release/action.yml");
    let parsed = workflow_yaml::parse(&action);
    let toolchains = parsed.find_all("toolchain");
    assert_eq!(toolchains.len(), 1, "the action names the compiler once");
    let compiler = toolchains[0].scalar().unwrap();
    assert!(is_version(compiler), "`{compiler}` is a named compiler");
    for name in workflow_names() {
        assert!(
            !read(&format!(".github/workflows/{name}")).contains(compiler),
            "{name} names the release compiler {compiler}"
        );
    }
    let release = workflow("release.yml");
    let build_steps = release.at(&["jobs", "build", "steps"]).unwrap();
    assert!(
        build_steps
            .items()
            .iter()
            .any(|step| step.get("uses").and_then(Yaml::scalar)
                == Some("./.github/actions/build-release")),
        "the release builds through the action"
    );
    // The inputs the action takes.
    assert!(parsed.at(&["inputs", "target"]).is_some());
    assert!(parsed.at(&["inputs", "rustflags"]).is_some());
    // The procedure names the action's line.
    assert!(
        read("docs/src/maintainers/releasing.md")
            .contains(".github/actions/build-release/action.yml"),
        "releasing.md names the action's `toolchain:` line"
    );
}

/// REQ-OBS-005 (RFC 020, F-1 of the 020q review): the panic hook is the first
/// thing `main` does, before the command line is read, so that no panic
/// reaches standard error as plain text. The install is looked for in the
/// source (the hook is process-wide and no test can undo it).
#[test]
fn req_obs_005_main_installs_the_panic_hook_first() {
    let main = read("crates/rypsok/src/main.rs");
    let body = main
        .split("fn main()")
        .nth(1)
        .expect("main")
        .split_once('{')
        .expect("a body")
        .1;
    let first = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("//"))
        .expect("a statement");
    assert_eq!(
        first, "rypsok_core::telemetry::install_panic_hook();",
        "the first statement of main"
    );
}

/// REQ-SEC-007 (RFC 020, TST-15; RFC 011): the agent client probes a pooled
/// connection every 60 s and drops an idle one after 30 s, so that a
/// connection made to a judged address is not kept for long once the peer is
/// gone. The settings are not readable from a built client; the source of
/// `agent_client` is.
#[test]
fn req_sec_007_the_agent_client_sets_tcp_keepalive_and_the_idle_timeout() {
    let source = read("crates/rypsok-core/src/egress.rs");
    let start = source.find("pub fn agent_client").expect("agent_client");
    let body = &source[start
        ..source[start..]
            .find("\n}\n")
            .map_or(source.len(), |n| start + n)];
    let compact: String = body.split_whitespace().collect();
    assert!(
        compact.contains(".tcp_keepalive(Some(Duration::from_secs(60)))"),
        "agent_client does not set tcp_keepalive to 60 s"
    );
    assert!(
        compact.contains(".pool_idle_timeout(Some(Duration::from_secs(30)))"),
        "agent_client does not set pool_idle_timeout to 30 s"
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
    let action = workflow_yaml::parse(&read(".github/actions/build-release/action.yml"));
    let toolchains = action.find_all("toolchain");
    assert!(
        !toolchains.is_empty(),
        "the release action names a toolchain"
    );
    for toolchain in toolchains {
        let text = toolchain.scalar().unwrap();
        assert!(
            is_version(text),
            "the release action builds with `{text}`, not a named compiler"
        );
    }
}

/// RFC 020 (TST-9): the crate builds without a warning with no engine
/// feature and with each one alone: CI checks (all targets, locked) the
/// combination of no features and each feature of `rypsok-core` by itself.
/// The list of features is read from the manifest, so a new feature needs a
/// step.
#[test]
fn req_test_012_ci_checks_every_feature_alone_and_none() {
    let manifest = read("crates/rypsok-core/Cargo.toml");
    let features: Vec<String> = manifest
        .split("[features]")
        .nth(1)
        .unwrap()
        .lines()
        .skip(1)
        .take_while(|line| !line.starts_with('['))
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split_once(" = "))
        .map(|(name, _)| name.trim().to_owned())
        .filter(|name| name != "default")
        .collect();
    assert!(features.len() >= 3, "{features:?}");
    let runs = run_lines(&workflow("ci.yml"));
    let none = "cargo check -p rypsok-core --no-default-features --all-targets --locked";
    assert!(
        runs.iter().any(|line| line == none),
        "no step runs `{none}`"
    );
    for feature in features {
        let line = format!(
            "cargo check -p rypsok-core --no-default-features --features {feature} --all-targets --locked"
        );
        assert!(runs.contains(&line), "no step runs `{line}`");
    }
}

/// RFC 020 (DEP-9): the cargo commands that build, test, lint or document
/// use the lock file, in every workflow.
#[test]
fn req_test_012_cargo_commands_of_the_workflows_are_locked() {
    let mut seen = 0;
    // The workflows, and the local action that builds the release.
    let mut files: Vec<(String, Yaml)> = workflow_names()
        .into_iter()
        .map(|name| {
            let parsed = workflow(&name);
            (name, parsed)
        })
        .collect();
    files.push((
        "build-release/action.yml".to_owned(),
        workflow_yaml::parse(&read(".github/actions/build-release/action.yml")),
    ));
    for (name, parsed) in files {
        for line in run_lines(&parsed) {
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

/// The workflow `release.yml`, and the targets of a matrix's `include`.
fn include_targets(file: &Yaml, job: &str) -> Vec<(String, String)> {
    file.at(&["jobs", job, "strategy", "matrix", "include"])
        .unwrap_or_else(|| panic!("job `{job}` has include entries"))
        .items()
        .iter()
        .filter_map(|entry| {
            Some((
                entry.get("target")?.scalar()?.to_owned(),
                entry.get("os")?.scalar()?.to_owned(),
            ))
        })
        .collect()
}

/// RFC 020 (D-22): the release profile strips symbols and nothing else: no
/// LTO, no `codegen-units`, no `panic` (unwinding stays, REQ-SEC-014).
#[test]
fn req_sec_014_the_release_profile_strips_symbols_and_keeps_unwinding() {
    let manifest = read("Cargo.toml");
    let profile = manifest
        .split("[profile.release]")
        .nth(1)
        .expect("a [profile.release] table")
        .split("\n[")
        .next()
        .unwrap();
    assert!(
        profile.lines().any(|l| l.trim() == "strip = \"symbols\""),
        "strip = \"symbols\""
    );
    for forbidden in ["lto", "panic", "codegen-units"] {
        assert!(
            !profile
                .lines()
                .any(|l| l.trim_start().starts_with(forbidden)),
            "the release profile sets {forbidden}"
        );
    }
}

/// REQ-RUST-002 and REQ-DEP-005 (RFC 020, D-13): the release builds the same
/// four targets as the release checks compare, Linux aarch64 on the native
/// arm runner, and the action installs the musl tools for both musl targets.
#[test]
fn req_dep_005_the_release_and_its_checks_build_the_same_four_targets() {
    let release = include_targets(&workflow("release.yml"), "build");
    let checks_file = release_checks();
    let checks_os = include_targets(&checks_file, "reproducible");
    let mut released: Vec<&str> = release.iter().map(|(t, _)| t.as_str()).collect();
    released.sort_unstable();
    let mut checked: Vec<&str> = list_of(matrix_of(&checks_file, "reproducible"), "target");
    checked.sort_unstable();
    assert_eq!(released, checked, "one set of targets");
    assert_eq!(
        released,
        [
            "aarch64-apple-darwin",
            "aarch64-unknown-linux-musl",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-musl"
        ]
    );
    for (name, entries) in [
        ("release.yml", &release),
        ("release-checks.yml", &checks_os),
    ] {
        assert!(
            entries.contains(&(
                "aarch64-unknown-linux-musl".to_owned(),
                "ubuntu-24.04-arm".to_owned()
            )),
            "{name}: Linux aarch64 builds on the native arm runner"
        );
    }
    let action = read(".github/actions/build-release/action.yml");
    assert!(
        action.contains("endsWith(inputs.target, '-linux-musl')"),
        "the musl tools are installed for both musl targets"
    );
    // Four targets, two builds each, three at a time.
    assert_eq!(
        checks_file
            .at(&["jobs", "reproducible", "strategy", "max-parallel"])
            .and_then(Yaml::scalar),
        Some("3")
    );
}

/// REQ-DEP-002 and REQ-DEP-005 (RFC 020, D-12): the image is made from the
/// release binary: the `Dockerfile` builds nothing (no `RUN`, no `cargo`),
/// every base is pinned by digest, and the binary is copied in by
/// architecture; the `image` job waits for the builds, runs on every run, and
/// makes one image per architecture with the names the release page lists.
#[test]
fn req_dep_002_the_image_is_made_from_the_release_binary() {
    let dockerfile = read("Dockerfile");
    let instructions: Vec<&str> = dockerfile
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    assert!(
        !instructions.iter().any(|l| l.starts_with("RUN ")),
        "the Dockerfile builds nothing"
    );
    assert!(!dockerfile.contains("cargo"), "no cargo in the Dockerfile");
    let froms: Vec<&&str> = instructions
        .iter()
        .filter(|l| l.starts_with("FROM "))
        .collect();
    assert_eq!(froms.len(), 1, "one stage");
    for from in froms {
        assert!(
            from.contains("@sha256:"),
            "`{from}` is not pinned by digest"
        );
    }
    assert!(instructions.contains(&"ARG TARGETARCH"));
    assert!(
        instructions.contains(&"COPY dist/linux-${TARGETARCH}/rypsok /rypsok"),
        "the binary of the architecture is copied in"
    );
    let release = workflow("release.yml");
    let image = release.at(&["jobs", "image"]).expect("an image job");
    assert!(
        image.get("if").is_none(),
        "the image is made on every run, so that a dry run proves it builds"
    );
    let needs: Vec<&str> = match image.get("needs") {
        Some(n) => n.scalar().map_or_else(|| list_of_items(n), |one| vec![one]),
        None => Vec::new(),
    };
    assert!(needs.contains(&"build"), "the image waits for the builds");
    let text = read(".github/workflows/release.yml");
    for arch in ["amd64", "arm64"] {
        assert!(
            text.contains(&format!("-image-{arch}.tar.gz")),
            "no image file for {arch}"
        );
        assert!(text.contains(&format!("--platform linux/{arch}")));
    }
}

/// The scalars of a sequence node.
fn list_of_items(node: &Yaml) -> Vec<&str> {
    node.items().iter().filter_map(Yaml::scalar).collect()
}

/// REQ-DEP-005 (RFC 020, D-12): the order of the release jobs is build, image,
/// checksums, attach; the sums list the images too; the build jobs write the
/// hash of their archive, and the image job checks the archives it downloaded
/// against those hashes before it makes an image.
#[test]
fn req_dep_005_checksums_list_the_images_and_attach_uploads_them() {
    let release = workflow("release.yml");
    let needs = |job: &str| -> Vec<&str> {
        let node = release
            .at(&["jobs", job, "needs"])
            .unwrap_or_else(|| panic!("{job} needs"));
        node.scalar()
            .map_or_else(|| list_of_items(node), |one| vec![one])
    };
    assert_eq!(needs("image"), ["build"]);
    let mut checksums = needs("checksums");
    checksums.sort_unstable();
    assert_eq!(checksums, ["build", "image"]);
    let attach = needs("attach");
    assert!(attach.contains(&"checksums") && attach.contains(&"image"));
    let text = read(".github/workflows/release.yml");
    assert!(
        text.contains("sha256sum rypsok-* > SHA256SUMS"),
        "the sums cover the archives and the images (all start with rypsok-)"
    );
    // The attach job downloads the images as well as the archives.
    let downloads = release
        .at(&["jobs", "attach", "steps"])
        .unwrap()
        .items()
        .iter()
        .filter_map(|step| step.at(&["with", "pattern"]).and_then(Yaml::scalar));
    let patterns: Vec<&str> = downloads.collect();
    assert!(
        patterns.contains(&"archive-*") && patterns.contains(&"image-*"),
        "{patterns:?}"
    );
    // The hash of each archive travels from its build job to the image job.
    let build_runs = run_lines(&release.at(&["jobs", "build"]).cloned().unwrap()).join("\n");
    // Git Bash on Windows has `sha256sum` and no `shasum`; macOS the reverse
    // (020p review, C2): `shasum` only inside the fallback, never unconditionally.
    assert!(
        build_runs.contains("command -v sha256sum"),
        "the build jobs hash their archive with what the runner has"
    );
    for line in build_runs.lines().filter(|l| l.contains("shasum")) {
        assert!(
            line.contains("() {"),
            "`shasum` only as the fallback function: {line}"
        );
    }
    let image_runs = run_lines(&release.at(&["jobs", "image"]).cloned().unwrap()).join("\n");
    assert!(
        image_runs.contains("sha256sum --check"),
        "the image job checks the archives it downloaded"
    );
}

/// REQ-DEP-005 (RFC 020, D-16; 020p review, C1): the checksums job verifies
/// what it signs: it downloads the hashes of the build jobs into a directory
/// of their own (so they are neither in `SHA256SUMS` nor attested) and checks
/// the archives against them before it writes `SHA256SUMS`.
#[test]
fn req_dep_005_checksums_verifies_the_archives_before_it_writes_the_sums() {
    let release = workflow("release.yml");
    let steps = release.at(&["jobs", "checksums", "steps"]).unwrap().items();
    let mut hashes_to = None;
    for step in steps {
        if step.at(&["with", "pattern"]).and_then(Yaml::scalar) == Some("sha-*") {
            hashes_to = step.at(&["with", "path"]).and_then(Yaml::scalar);
        }
    }
    let hashes_to = hashes_to.expect("checksums downloads the sha-* artifacts");
    assert!(
        !hashes_to.starts_with("dist"),
        "the hashes are not in dist/: {hashes_to}"
    );
    let position = |needle: &str| {
        steps.iter().position(|step| {
            step.at(&["run"])
                .and_then(Yaml::scalar)
                .is_some_and(|run| run.contains(needle))
        })
    };
    let check = position("sha256sum --check").expect("a --check step in checksums");
    let write = position("> SHA256SUMS").expect("the step that writes SHA256SUMS");
    assert!(check < write, "the check comes before the sums are written");
}

/// REQ-SEC-020 (RFC 020, D-16): only the checksums job can sign, and it runs
/// no third-party build code: it downloads artifacts and hashes them, with no
/// checkout and no cargo; the attestation is made on a tag only and names the
/// files of `SHA256SUMS` (the archives and the images, `rypsok-*`).
#[test]
fn req_sec_020_only_the_checksums_job_signs_and_runs_no_build_code() {
    for name in workflow_names() {
        let file = workflow(&name);
        for (job, body) in jobs(&file) {
            let signs = body.get("permissions").is_some_and(|p| {
                p.entries().iter().any(|(permission, level)| {
                    matches!(permission.as_str(), "id-token" | "attestations")
                        && level.scalar() == Some("write")
                })
            });
            let may = matches!(
                (name.as_str(), job.as_str()),
                ("release.yml", "checksums") | ("docs.yml", "deploy")
            );
            assert!(!signs || may, "{name}: job `{job}` may not sign");
        }
    }
    let release = workflow("release.yml");
    let checksums = release.at(&["jobs", "checksums"]).unwrap();
    assert_eq!(
        checksums
            .at(&["permissions", "contents"])
            .and_then(Yaml::scalar),
        Some("read")
    );
    assert_eq!(
        checksums
            .at(&["permissions", "id-token"])
            .and_then(Yaml::scalar),
        Some("write")
    );
    assert_eq!(
        checksums
            .at(&["permissions", "attestations"])
            .and_then(Yaml::scalar),
        Some("write")
    );
    let steps = checksums.get("steps").unwrap().items();
    for step in steps {
        let uses = step.get("uses").and_then(Yaml::scalar).unwrap_or("");
        assert!(
            !uses.starts_with("actions/checkout"),
            "the signing job checks out nothing"
        );
        assert!(
            !uses.starts_with("./"),
            "the signing job runs no local action"
        );
    }
    assert!(
        !run_lines(checksums).iter().any(|l| l.contains("cargo")),
        "the signing job runs no cargo"
    );
    let attest = steps
        .iter()
        .find(|step| {
            step.get("uses")
                .and_then(Yaml::scalar)
                .is_some_and(|u| u.starts_with("actions/attest-build-provenance@"))
        })
        .expect("an attestation step");
    assert_eq!(
        attest.get("if").and_then(Yaml::scalar),
        Some("github.ref_type == 'tag'"),
        "attested on a tag only"
    );
    let subjects = attest
        .at(&["with", "subject-path"])
        .and_then(Yaml::scalar)
        .unwrap();
    assert!(
        subjects.contains("dist/rypsok-*"),
        "the subjects are the files of SHA256SUMS"
    );
}

/// REQ-DEP-005 (RFC 020, DEP-3): the attach job's summary says which compiler
/// built the release, read from the action's `toolchain:` line, so that the
/// owner can copy it into the release notes.
#[test]
fn req_dep_005_the_attach_summary_names_the_compiler() {
    let release = workflow("release.yml");
    let runs = run_lines(release.at(&["jobs", "attach"]).unwrap()).join("\n");
    assert!(
        runs.contains(".github/actions/build-release/action.yml") && runs.contains("toolchain:"),
        "the summary reads the action's toolchain line"
    );
    assert!(runs.contains("Built with rustc"));
}

/// RFC 020 (DEP-4): the Windows release build links reproducibly, in the
/// release and in its reproducibility check: the Windows entry carries the
/// flag and the build passes it to the action.
#[test]
fn req_sec_020_the_windows_release_build_links_with_brepro() {
    let release = workflow("release.yml");
    let checks = release_checks();
    for (name, file, job, entries) in [
        (
            "release.yml",
            &release,
            "build",
            release
                .at(&["jobs", "build", "strategy", "matrix", "include"])
                .unwrap(),
        ),
        (
            "release-checks.yml",
            &checks,
            "reproducible",
            checks
                .at(&["jobs", "reproducible", "strategy", "matrix", "include"])
                .unwrap(),
        ),
    ] {
        let windows = entries
            .items()
            .iter()
            .find(|entry| entry.get("os").and_then(Yaml::scalar) == Some("windows-latest"))
            .unwrap_or_else(|| panic!("{name}: a Windows entry"));
        assert!(
            windows
                .get("rustflags")
                .and_then(Yaml::scalar)
                .is_some_and(|flags| flags.contains("link-arg=/Brepro")),
            "{name}: the Windows entry sets /Brepro"
        );
        let passes = file
            .at(&["jobs", job, "steps"])
            .unwrap()
            .items()
            .iter()
            .any(|step| {
                step.get("uses").and_then(Yaml::scalar) == Some("./.github/actions/build-release")
                    && step.at(&["with", "rustflags"]).and_then(Yaml::scalar)
                        == Some("${{ matrix.rustflags }}")
            });
        assert!(
            passes,
            "{name}: the build takes its flags from the matrix entry"
        );
    }
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

/// REQ-RUST-002: CI builds and tests on Linux (x86_64 and aarch64), macOS and
/// Windows; the release builds the four targets (a test of its own).
#[test]
fn req_rust_002_ci_matrix_covers_three_platforms() {
    let ci = read(".github/workflows/ci.yml");
    for os in [
        "ubuntu-latest",
        "ubuntu-24.04-arm",
        "macos-latest",
        "windows-latest",
    ] {
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
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "yml"))
        .collect();
    // The local composite actions are workflow code too.
    let actions = repository_root().join(".github/actions");
    if actions.exists() {
        for entry in std::fs::read_dir(&actions).unwrap() {
            let action = entry.unwrap().path().join("action.yml");
            if action.exists() {
                files.push(action);
            }
        }
    }
    let mut seen = 0;
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        for (number, line) in text.lines().enumerate() {
            let Some(rest) = line
                .trim_start()
                .strip_prefix("- uses: ")
                .or_else(|| line.trim_start().strip_prefix("uses: "))
            else {
                continue;
            };
            // A local action or workflow of this repository has no commit to
            // pin: it is the code of the commit that runs it.
            if rest.trim_start().starts_with("./") {
                continue;
            }
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

/// REQ-DOC-002 (RFC 020, D-17): the book is published from the release tag
/// (or on a visible dispatch), never on a push to `main`, and a failed
/// publish is red; CI still builds the book on every push.
#[test]
fn req_doc_002_the_book_is_published_from_the_release_tag() {
    let docs = workflow("docs.yml");
    let triggers: Vec<&str> = docs
        .get("on")
        .expect("triggers")
        .entries()
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(triggers, ["push", "workflow_dispatch"]);
    let push = docs.at(&["on", "push"]).unwrap();
    assert!(push.get("branches").is_none(), "no push to a branch");
    assert_eq!(list_of_items(push.get("tags").expect("tags")), ["[0-9]*"]);
    assert!(
        docs.find_all("continue-on-error").is_empty(),
        "a failed publish is red"
    );
    let deploy = docs.at(&["jobs", "deploy", "permissions"]).unwrap();
    assert_eq!(deploy.get("pages").and_then(Yaml::scalar), Some("write"));
    assert_eq!(deploy.get("id-token").and_then(Yaml::scalar), Some("write"));
    let ci = workflow("ci.yml");
    assert!(
        run_lines(&ci).iter().any(|l| l == "mdbook build docs"),
        "CI builds the book on every push"
    );
}

/// REQ-DOC-002 (RFC 020, D-17): the first page says which release the book
/// describes, between two markers that the publishing workflow replaces, and
/// the workflow fails when the markers are gone.
#[test]
fn req_doc_002_the_first_page_says_which_release_it_describes() {
    let intro = read("docs/src/introduction.md");
    let open = intro.find("<!-- release -->").expect("opening marker");
    let close = intro.find("<!-- /release -->").expect("closing marker");
    assert!(open < close);
    assert!(
        intro[open..close]
            .contains("This book describes the development version of rypsok, not a release."),
        "the text between the markers is the development line"
    );
    assert!(
        intro[..open].lines().filter(|l| !l.is_empty()).count() == 1,
        "the line is directly under the title"
    );
    let docs = workflow("docs.yml");
    let steps = docs.at(&["jobs", "build", "steps"]).unwrap().items();
    let step_with = |needle: &str| {
        steps.iter().position(|step| {
            step.at(&["run"])
                .and_then(Yaml::scalar)
                .is_some_and(|run| run.contains(needle))
        })
    };
    let replace = step_with("<!-- release -->").expect("a step that replaces the line");
    let build = step_with("mdbook build docs").expect("the build step");
    assert!(replace < build, "the line is replaced before the build");
    let script = steps[replace].at(&["run"]).and_then(Yaml::scalar).unwrap();
    assert!(script.contains("<!-- /release -->") && script.contains("exit 1"));
    assert!(script.contains("This book describes rypsok "));
    assert!(script.contains("not a release."));
}

/// REQ-DOC-002 (RFC 020, D-17): the maintainer's page names the setting that
/// lets the tag deploy and says a documentation fix is a patch release.
#[test]
fn req_doc_002_releasing_names_the_pages_setting() {
    let releasing = read("docs/src/maintainers/releasing.md");
    for needle in ["github-pages", "[0-9]*", "patch release", "fails red"] {
        assert!(releasing.contains(needle), "releasing.md: {needle}");
    }
}

/// REQ-PERF-003 (RFC 020, PRF-4): the memory ceiling of the documented unit
/// and of the documented container run leaves no room for swap.
#[test]
fn req_perf_003_the_deployment_page_forbids_swap() {
    let deployment = read("docs/src/reference/deployment.md");
    assert!(deployment.contains("MemoryMax=256M"));
    assert!(deployment.contains("MemorySwapMax=0"));
    let line = deployment
        .lines()
        .find(|l| l.contains("--memory 256m"))
        .expect("the docker run line");
    assert!(line.contains("--memory-swap 256m"), "{line}");
}

/// RFC 020, D-17 and item 7 of 020t: a decision id a reader can resolve. Two
/// lists use `D-n`, the startup review's and the audit's (RFC 020). Outside
/// `requirements.md`, an audit decision is written `RFC 020, D-n` (the
/// `RFC 020` before it in the same sentence or parenthesis), and a bare id is
/// allowed only on a page that says what a bare id is.
#[test]
fn req_doc_003_a_decision_id_names_its_list() {
    let src = repository_root().join("docs").join("src");
    let mut stack = vec![src.clone()];
    let mut bare_pages = 0;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if !name.ends_with(".md") || name == "maintainers/requirements.md" {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let explained = text.contains("A bare `D-nn`");
            let bytes = text.as_bytes();
            let mut from = 0;
            while let Some(at) = text[from..].find("D-") {
                let start = from + at;
                from = start + 2;
                let digits = bytes[from..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();
                let word_start = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
                if digits == 0 || !word_start {
                    continue;
                }
                // The sentence or parenthesis so far: back to the last `.`,
                // `(` or line start.
                let before = &text[..start];
                let line_start = before.rfind('\n').map_or(0, |i| i + 1);
                let context = &before[line_start..];
                let cut = context.rfind(['.', '(']).map_or(0, |i| i + 1);
                let audit = context[cut..].contains("RFC 020")
                    || context.rsplit_once("RFC 020").is_some_and(|(_, tail)| {
                        tail.chars().all(|c| !matches!(c, '.' | '(' | ';'))
                    });
                if audit {
                    assert!(
                        !context[cut..].contains("RFC 020 D-"),
                        "{name}: write `RFC 020, D-n` with the comma: {context}D-"
                    );
                    continue;
                }
                assert!(
                    explained,
                    "{name}: a bare `D-n` without saying what it is: {context}D-"
                );
                bare_pages += 1;
            }
        }
    }
    assert!(bare_pages > 0 || src.exists());
}
