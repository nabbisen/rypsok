//! What the binary says when it cannot start (RFC 020, 020x, F-3). Linux
//! only: the trust store and its environment variables are read there from
//! `SSL_CERT_FILE` and `SSL_CERT_DIR`.
#![cfg(target_os = "linux")]

use std::process::{Command, Stdio};

/// REQ-ERR-004 / REQ-OPS-001 (RFC 020, F-3): with a trust store that holds no
/// certificate, `serve` stops with status 1 and one message that says once
/// that the client could not be built and names the cause and where roots
/// come from.
#[test]
fn req_ops_001_a_missing_trust_store_is_named_once_at_start() {
    let dir = std::env::temp_dir().join(format!("rypsok-startup-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let empty = dir.join("empty.pem");
    std::fs::write(&empty, "").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rypsok"))
        .arg("serve")
        .env("SSL_CERT_FILE", &empty)
        .env_remove("SSL_CERT_DIR")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    let line = stderr
        .lines()
        .find(|l| l.starts_with("rypsok: configuration:"))
        .unwrap_or_else(|| panic!("no configuration error in:\n{stderr}"));
    assert_eq!(
        line.matches("could not be built").count(),
        1,
        "the clause once: {line}"
    );
    assert!(line.contains("root certificates"), "{line}");
    assert!(line.contains("SSL_CERT_FILE"), "{line}");
    assert!(!line.contains("builder error"), "{line}");
}
