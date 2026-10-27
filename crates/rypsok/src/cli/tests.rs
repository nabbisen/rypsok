//! Tests of the command line. Every test names the requirement it verifies.

use super::*;

/// REQ-OPS-001: an unknown subcommand is a usage error (exit code 2).
#[test]
fn req_ops_001_unknown_command_is_a_usage_error() {
    assert_eq!(run(["rypsok", "explode"]), ExitCode::from(EXIT_USAGE));
}

/// REQ-OPS-001 and REQ-SEC-018: `serve --transport http` with a bind off
/// loopback and no callers is a configuration error (exit code 1).
#[test]
fn req_sec_018_http_serve_refuses_a_non_loopback_bind_without_callers() {
    let dir = std::env::temp_dir().join(format!("rypsok-cli-http-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("open.toml");
    std::fs::write(
        &bad,
        "config_version = 1\n[service.http]\nbind = \"0.0.0.0:8737\"\n",
    )
    .unwrap();
    assert_eq!(
        run([
            "rypsok",
            "serve",
            "--transport",
            "http",
            "--config",
            bad.to_str().unwrap()
        ]),
        ExitCode::from(EXIT_CONFIGURATION)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// REQ-OPS-001 and REQ-CFG-005: `check` exits 1 on an invalid file and 0
/// on a valid one.
#[test]
fn req_ops_001_check_exit_codes() {
    let dir = std::env::temp_dir().join(format!("rypsok-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.toml");
    std::fs::write(&bad, "config_version = 1\n[search]\nmax_results = 99\n").unwrap();
    assert_eq!(
        run(["rypsok", "check", "--config", bad.to_str().unwrap()]),
        ExitCode::from(EXIT_CONFIGURATION)
    );
    let good = dir.join("good.toml");
    std::fs::write(&good, "config_version = 1\n").unwrap();
    assert_eq!(
        run(["rypsok", "check", "--config", good.to_str().unwrap()]),
        ExitCode::from(EXIT_SUCCESS)
    );
    let missing = dir.join("missing.toml");
    assert_eq!(
        run(["rypsok", "check", "--config", missing.to_str().unwrap()]),
        ExitCode::from(EXIT_CONFIGURATION)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
