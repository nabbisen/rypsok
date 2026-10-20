//! A spawned `rypsok serve` process spoken to over stdio.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const META: &str = "_meta";

/// Writes a configuration file for one test and returns its path.
pub fn config_file(text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rypsok-protocol-{}-{}",
        std::process::id(),
        unique()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("rypsok.toml");
    std::fs::write(&path, text).unwrap();
    path
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

pub struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    stderr: Receiver<String>,
    next_id: u64,
    stdout_log: Vec<String>,
}

impl Session {
    /// Starts `rypsok serve` with the given configuration file, or with a
    /// configuration that disables every engine so that no network is
    /// touched when `config` is `None`.
    pub fn start(config: Option<&Path>) -> Self {
        let default = config_file(
            "config_version = 1\n[engines.wikipedia]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n[engines.brave]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key_env = \"RYPSOK_TEST_NO_SUCH_KEY\"\n",
        );
        let path = config.map(Path::to_path_buf).unwrap_or(default);
        Self::spawn(Some(&path), &[])
    }

    /// Starts `rypsok serve` with no configuration at all (REQ-OPS-003).
    pub fn start_without_config() -> Self {
        Self::spawn(None, &[])
    }

    /// Starts `rypsok serve` with extra environment variables.
    pub fn start_with_env(config: &Path, env: &[(&str, &str)]) -> Self {
        Self::spawn(Some(config), env)
    }

    /// The process identifier.
    #[cfg(unix)]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    fn spawn(config: Option<&Path>, env: &[(&str, &str)]) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rypsok"));
        command
            .arg("serve")
            .env_remove("RYPSOK_CONFIG")
            .env_remove("BRAVE_API_KEY");
        for (name, value) in env {
            command.env(name, value);
        }
        if let Some(path) = config {
            command.arg("--config").arg(path);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("rypsok spawns");
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let (etx, stderr_rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if etx.send(line).is_err() {
                    break;
                }
            }
        });
        let stdin = child.stdin.take();
        Session {
            child,
            stdin,
            lines,
            stderr: stderr_rx,
            next_id: 1,
            stdout_log: Vec::new(),
        }
    }

    fn meta() -> Value {
        json!({
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientInfo": { "name": "rypsok-protocol-test", "version": "0" },
            "io.modelcontextprotocol/clientCapabilities": {}
        })
    }

    fn send(&mut self, message: Value) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin.write_all(message.to_string().as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }

    fn read_response(&mut self, id: u64) -> Value {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(remaining) {
                Ok(line) => {
                    self.stdout_log.push(line.clone());
                    let value: Value = serde_json::from_str(&line)
                        .unwrap_or_else(|_| panic!("non-JSON on stdout: {line:?}"));
                    if value["id"] == json!(id) {
                        return value;
                    }
                }
                Err(RecvTimeoutError::Timeout) => panic!("no response to request {id} within 10 s"),
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("rypsok closed stdout before answering request {id}")
                }
            }
        }
    }

    /// `server/discover` with the 2026-07-28 metadata; returns the result.
    pub fn discover(&mut self) -> Value {
        self.request("server/discover", json!({}))
    }

    /// A request of the stateless revision, with the per-request metadata.
    pub fn request(&mut self, method: &str, mut params: Value) -> Value {
        params[META] = Self::meta();
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let response = self.read_response(id);
        assert!(
            response.get("error").is_none(),
            "unexpected protocol error: {response}"
        );
        response["result"].clone()
    }

    /// Sends a request and returns its id without waiting.
    pub fn send_request(&mut self, method: &str, mut params: Value) -> u64 {
        params[META] = Self::meta();
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    /// Whether no response for `id` arrives within `within`.
    pub fn no_response_for(&mut self, id: u64, within: Duration) -> bool {
        let deadline = Instant::now() + within;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(remaining) {
                Ok(line) => {
                    self.stdout_log.push(line.clone());
                    let value: Value = serde_json::from_str(&line).unwrap_or_default();
                    if value["id"] == json!(id) {
                        return false;
                    }
                }
                Err(_) => return true,
            }
        }
    }

    /// A request that is expected to fail at the protocol level; returns
    /// the error object.
    pub fn request_error(&mut self, method: &str, mut params: Value) -> Value {
        params[META] = Self::meta();
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let response = self.read_response(id);
        assert!(
            response.get("result").is_none(),
            "expected a protocol error: {response}"
        );
        response["error"].clone()
    }

    /// A request without per-request metadata (legacy handshake sessions).
    pub fn request_raw(&mut self, mut message: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        message["jsonrpc"] = json!("2.0");
        message["id"] = json!(id);
        self.send(message);
        let response = self.read_response(id);
        assert!(
            response.get("error").is_none(),
            "unexpected protocol error: {response}"
        );
        response["result"].clone()
    }

    /// A notification.
    pub fn notify_raw(&mut self, mut message: Value) {
        message["jsonrpc"] = json!("2.0");
        self.send(message);
    }

    /// Waits for the process without closing stdin (after a signal).
    #[cfg(unix)]
    pub fn wait_without_closing(self, within: Duration) -> (ExitStatus, Vec<String>, String) {
        self.wait_inner(within, false)
    }

    /// Closes stdin and waits for the process; returns its status, every
    /// stdout line seen, and the whole of stderr.
    pub fn close_and_wait(self, within: Duration) -> (ExitStatus, Vec<String>, String) {
        self.wait_inner(within, true)
    }

    fn wait_inner(mut self, within: Duration, close: bool) -> (ExitStatus, Vec<String>, String) {
        if close {
            drop(self.stdin.take());
        }
        let deadline = Instant::now() + within;
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "rypsok did not exit within {within:?} after end of file"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        while let Ok(line) = self.lines.try_recv() {
            self.stdout_log.push(line);
        }
        let mut stderr = String::new();
        while let Ok(line) = self.stderr.recv_timeout(Duration::from_millis(200)) {
            stderr.push_str(&line);
            stderr.push('\n');
        }
        (status, std::mem::take(&mut self.stdout_log), stderr)
    }

    /// Ends the session, asserting a clean exit.
    pub fn finish(self) {
        let (status, _, _) = self.close_and_wait(Duration::from_millis(3000));
        assert!(status.success());
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
