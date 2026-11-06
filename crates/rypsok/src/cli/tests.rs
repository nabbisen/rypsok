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

/// REQ-OPS-001 (RFC 020, DAT-1): `check` exits 1 for a configuration that
/// `serve --transport http` would refuse for its callers' tokens (an unset
/// variable, a token below the minimum) and 0 for a good one.
#[test]
fn req_ops_001_check_exits_1_for_a_token_serve_refuses() {
    let dir = std::env::temp_dir().join(format!("rypsok-cli-tokens-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str, content: &str| -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        path
    };
    let short = write("short.token", "12345");
    let good = write("good.token", "0123456789abcdefghijABCDEFGHIJ0123456789");
    let config = |source: &str| {
        let text = format!(
            "config_version = 1\n[[service.http.auth.callers]]\nid = \"alpha\"\n{source}\n"
        );
        write("config.toml", &text)
    };
    let check = |path: &PathBuf| run(["rypsok", "check", "--config", path.to_str().unwrap()]);
    let toml_path = |p: &PathBuf| p.to_str().unwrap().replace('\\', "/");
    assert_eq!(
        check(&config("token_env = \"RYPSOK_TEST_CLI_NO_SUCH_VARIABLE\"")),
        ExitCode::from(EXIT_CONFIGURATION)
    );
    assert_eq!(
        check(&config(&format!("token_file = \"{}\"", toml_path(&short)))),
        ExitCode::from(EXIT_CONFIGURATION)
    );
    assert_eq!(
        check(&config(&format!("token_file = \"{}\"", toml_path(&good)))),
        ExitCode::from(EXIT_SUCCESS)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
