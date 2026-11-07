//! A panic writes one JSON line and not its message (RFC 020, F-1 of the 020q
//! review; REQ-OBS-005, REQ-PRIV-004). The hook is process-wide, so the test
//! runs its body in a child process of itself and reads the child's standard
//! error.

use rypsok_core::telemetry::install_panic_hook;

const SECRET: &str = "the-secret-value-31c9d7";

#[test]
fn req_obs_005_a_panic_is_one_json_line_without_its_message() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "panic_child", "--ignored", "--nocapture"])
        .output()
        .unwrap();
    assert!(output.status.success(), "the child ends normally");
    let stderr = String::from_utf8(output.stderr).unwrap();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "one line, not a block: {stderr:?}");
    assert!(
        !stderr.contains(SECRET),
        "the message is not logged: {stderr}"
    );
    let line: serde_json::Value = serde_json::from_str(lines[0]).expect("a JSON object");
    assert_eq!(line["level"], "ERROR");
    assert_eq!(line["event"], "panic");
    assert!(line["timestamp"].as_str().unwrap().ends_with('Z'), "{line}");
    let location = line["location"].as_str().unwrap();
    assert!(
        location.ends_with("panic_hook.rs:36") || location.contains("panic_hook.rs:"),
        "{location}"
    );
    assert_eq!(line["thread"], "panic_child", "the test thread's name");
}

/// The child: installs the hook, panics inside `catch_unwind` (the panic
/// unwinds and the process goes on), and ends normally.
#[test]
#[ignore = "run by the parent test"]
fn panic_child() {
    install_panic_hook();
    let caught = std::panic::catch_unwind(|| panic!("{SECRET}"));
    assert!(caught.is_err(), "the panic unwound");
}
