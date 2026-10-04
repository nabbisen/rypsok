//! Tests of the command line. Every test names the requirement it verifies.

use super::*;

/// Behavior of the M0 shell: `serve` and `check` are not implemented and
/// exit with the usage code (handoff 001, deliverable 3).
#[test]
fn m0_shell_serve_and_check_exit_with_usage_code() {
    assert_eq!(run(["rypsok", "serve"]), ExitCode::from(EXIT_USAGE));
    assert_eq!(run(["rypsok", "check"]), ExitCode::from(EXIT_USAGE));
}

/// An unknown subcommand is a usage error (REQ-OPS-001, exit code 2).
#[test]
fn req_ops_001_unknown_command_is_a_usage_error() {
    assert_eq!(run(["rypsok", "explode"]), ExitCode::from(EXIT_USAGE));
}
