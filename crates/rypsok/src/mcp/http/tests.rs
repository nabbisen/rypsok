//! Tests of the HTTP listener (REQ-TEST-014): an in-process listener on an
//! ephemeral loopback port, driven by an HTTP client. Every test names the
//! requirement it verifies.

use std::sync::Arc;
use std::time::Duration;

use rypsok_core::Rypsok;
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const META: &str = "_meta";

/// Writes a token to a file of this test run and returns its path.
fn token_file(name: &str, token: &str) -> String {
    let dir = std::env::temp_dir().join(format!("rypsok-http-tokens-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.token"));
    std::fs::write(&path, token).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    // Forward slashes: the path is written into a TOML basic string, where a
    // backslash would start an escape (Windows accepts both separators).
    path.to_str().unwrap().replace('\\', "/")
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "rypsok-http-test", "version": "0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

struct Server {
    url: String,
    service: Arc<Rypsok>,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl Server {
    async fn start(config_text: &str) -> Server {
        let config = Config::parse(config_text).unwrap();
        config.validate().unwrap();
        let service = Arc::new(Rypsok::new(config).unwrap());
        let settings = HttpSettings::from_config(service.config()).unwrap();
        let listener = bind(&settings).await.unwrap();
        let url = format!("http://{}{PATH}", listener.local_addr().unwrap());
        let shutdown = CancellationToken::new();
        let adapter = Adapter::new(Arc::clone(&service));
        let task = tokio::spawn(run(adapter, listener, settings, shutdown.clone()));
        Server {
            url,
            service,
            shutdown,
            task,
        }
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap()
    }

    /// A request of the 2026-07-28 revision: the per-request metadata in
    /// the body and the standard headers beside it (`MCP-Protocol-Version`,
    /// `Mcp-Method`, `Mcp-Name` for a tool call).
    fn request(&self, method: &str, params: Value) -> reqwest::RequestBuilder {
        let mut params = params;
        let name = params["name"].as_str().map(str::to_owned);
        params[META] = meta();
        let mut builder = self
            .client()
            .post(&self.url)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2026-07-28")
            .header("mcp-method", method);
        if let Some(name) = name {
            builder = builder.header("mcp-name", name);
        }
        builder.json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
    }

    async fn stop(self) {
        self.shutdown.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(5), self.task).await;
    }
}

const LOOPBACK: &str = "config_version = 1\n[service.http]\nbind = \"127.0.0.1:0\"\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n";

/// REQ-MCP-003 and REQ-MCP-011: `server/discover` and a tool call of the
/// 2026-07-28 revision are served statelessly over HTTP as JSON; a path
/// other than `/mcp` is 404.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_mcp_003_stateless_discover_and_call_as_json() {
    let s = Server::start(LOOPBACK).await;
    let response = s
        .request("server/discover", json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    assert!(
        response.headers().get("mcp-session-id").is_none(),
        "stateless"
    );
    let body: Value = response.json().await.unwrap();
    assert_eq!(
        body["result"]["instructions"],
        super::super::tools::INSTRUCTIONS
    );
    assert!(
        body["result"]["capabilities"]["tools"].is_object(),
        "{body}"
    );

    let response = s
        .request(
            "tools/call",
            json!({ "name": "engine_status", "arguments": {} }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["result"]["isError"], false, "{body}");
    assert_eq!(body["result"]["structuredContent"]["engines"][0]["id"], "w");

    let other = s
        .client()
        .post(s.url.replace("/mcp", "/other"))
        .send()
        .await
        .unwrap();
    assert_eq!(other.status(), 404);
    s.stop().await;
}

/// REQ-MCP-006: the legacy `initialize` handshake works over HTTP with a
/// session; with `legacy_revisions = false` it is refused with the SDK's
/// `UNSUPPORTED_PROTOCOL_VERSION` error, no session is created,
/// `server/discover` lists the 2026-07-28 revision alone and the stateless
/// revision keeps working.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_mcp_006_legacy_handshake_over_http_until_disabled() {
    let initialize = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }
    });
    let s = Server::start(LOOPBACK).await;
    let response = s
        .client()
        .post(&s.url)
        .header("accept", "application/json, text/event-stream")
        .json(&initialize)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let has_session = response.headers().get("mcp-session-id").is_some();
    let text = response.text().await.unwrap();
    assert_eq!(status, 200, "{text}");
    assert!(has_session, "a session for a legacy revision: {text}");
    s.stop().await;

    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nlegacy_revisions = false\n",
    ))
    .await;
    let response = s
        .client()
        .post(&s.url)
        .header("accept", "application/json, text/event-stream")
        .json(&initialize)
        .send()
        .await
        .unwrap();
    assert!(
        response.headers().get("mcp-session-id").is_none(),
        "no session without legacy revisions: {}",
        response.status()
    );
    let text = response.text().await.unwrap();
    assert!(
        text.contains("-32022"),
        "initialize refused as unsupported protocol version: {text}"
    );
    let response = s
        .request("server/discover", json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let discovered: Value = response.json().await.unwrap();
    let versions = discovered["result"]["supportedVersions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        versions,
        vec![json!("2026-07-28")],
        "one revision advertised: {discovered}"
    );
    s.stop().await;
}

/// REQ-SEC-018 and THR-18: a foreign `Host` is refused; any `Origin` is
/// refused by default; an allowed origin passes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_018_host_and_origin_are_validated() {
    let s = Server::start(LOOPBACK).await;
    let foreign_host = s
        .request("server/discover", json!({}))
        .header("host", "evil.example")
        .send()
        .await
        .unwrap();
    assert!(
        foreign_host.status().is_client_error(),
        "{}",
        foreign_host.status()
    );
    let origin = s
        .request("server/discover", json!({}))
        .header("origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(origin.status(), 403);
    s.stop().await;

    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nallowed_origins = [\"https://app.example:*\"]\n",
    ))
    .await;
    let allowed = s
        .request("server/discover", json!({}))
        .header("origin", "https://app.example")
        .send()
        .await
        .unwrap();
    assert_eq!(allowed.status(), 200);
    let other = s
        .request("server/discover", json!({}))
        .header("origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(other.status(), 403);
    s.stop().await;
}

/// REQ-SEC-018: a bind off loopback without callers is refused at
/// validation; on loopback without callers every request is the caller
/// `local`.
#[test]
fn req_sec_018_non_loopback_bind_needs_callers() {
    let config =
        Config::parse("config_version = 1\n[service.http]\nbind = \"0.0.0.0:8737\"\n").unwrap();
    let error = config.validate().unwrap_err();
    assert!(
        error
            .problems
            .iter()
            .any(|p| p.starts_with("service.http.bind") && p.contains("callers")),
        "{:?}",
        error.problems
    );
    let bad_bind =
        Config::parse("config_version = 1\n[service.http]\nbind = \"not an address\"\n").unwrap();
    assert!(bad_bind.validate().is_err());
    // An unspecified address needs `allowed_hosts`: the default host list
    // would admit no name a client sends (RFC 013 amendment).
    let unspecified = Config::parse(
        "config_version = 1\n[service.http]\nbind = \"0.0.0.0:8737\"\n[[service.http.auth.callers]]\nid = \"a\"\ntoken_env = \"RYPSOK_TEST_TOKEN\"\n",
    )
    .unwrap();
    let error = unspecified.validate().unwrap_err();
    assert!(
        error
            .problems
            .iter()
            .any(|p| p.starts_with("service.http.bind") && p.contains("allowed_hosts")),
        "{:?}",
        error.problems
    );
    let named = Config::parse(
        "config_version = 1\n[service.http]\nbind = \"0.0.0.0:8737\"\nallowed_hosts = [\"rypsok.example\"]\n[[service.http.auth.callers]]\nid = \"a\"\ntoken_env = \"RYPSOK_TEST_TOKEN\"\n",
    )
    .unwrap();
    assert!(named.validate().is_ok(), "{:?}", named.validate());
    let auth = Auth {
        callers: Vec::new(),
    };
    assert_eq!(
        auth.check(&http::HeaderMap::new()).unwrap(),
        CallerId::local()
    );
}

/// REQ-SEC-018, REQ-TEST-014 and THR-19: with callers configured a missing
/// or wrong token is 401 with `WWW-Authenticate`, the right token passes,
/// and the token appears in no log line.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_test_014_bearer_token_required_with_callers() {
    let token_a = token_file("a", "token-a-0123456789-0123456789-0123456789");
    let s = Server::start(&LOOPBACK.replace(
        "[engines.w]",
        &format!(
            "[[service.http.auth.callers]]\nid = \"alpha\"\ntoken_file = \"{token_a}\"\n[engines.w]"
        ),
    ))
    .await;
    let missing = s
        .request("server/discover", json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 401);
    assert_eq!(
        missing.headers().get("www-authenticate").unwrap(),
        "Bearer realm=\"rypsok\""
    );
    let wrong = s
        .request("server/discover", json!({}))
        .bearer_auth("token-a-0123456789-0123456789-012345678X")
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), 401);
    let right = s
        .request("server/discover", json!({}))
        .bearer_auth("token-a-0123456789-0123456789-0123456789")
        .send()
        .await
        .unwrap();
    assert_eq!(right.status(), 200);
    s.stop().await;
}

/// REQ-SEC-019: rate limits apply per caller: one caller exhausting its
/// limit does not affect another.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_019_limits_are_per_caller() {
    let token_b = token_file("b", "token-b-0123456789-0123456789-0123456789");
    let token_c = token_file("c", "token-c-0123456789-0123456789-0123456789");
    let s = Server::start(&LOOPBACK.replace(
        "[engines.w]",
        &format!("[[service.http.auth.callers]]\nid = \"beta\"\ntoken_file = \"{token_b}\"\n[[service.http.auth.callers]]\nid = \"gamma\"\ntoken_file = \"{token_c}\"\n[limits]\nengine_status_per_minute = 1\n[engines.w]"),
    ))
    .await;
    let status = |token: &'static str| {
        s.request(
            "tools/call",
            json!({ "name": "engine_status", "arguments": {} }),
        )
        .bearer_auth(token)
    };
    let first: Value = status("token-b-0123456789-0123456789-0123456789")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first["result"]["isError"], false, "{first}");
    let second: Value = status("token-b-0123456789-0123456789-0123456789")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(second["result"]["isError"], true, "{second}");
    assert_eq!(
        second["result"]["structuredContent"]["error"]["detail"]["reason"],
        "rate_limited"
    );
    let other: Value = status("token-c-0123456789-0123456789-0123456789")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        other["result"]["isError"], false,
        "the other caller has its own limit: {other}"
    );
    s.stop().await;
}

/// LIM-H2: a request body above the bound is 413 before it is parsed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_test_014_body_above_the_bound_is_413() {
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nmax_request_bytes = 4096\n",
    ))
    .await;
    let padding = "x".repeat(5000);
    let response = s
        .client()
        .post(&s.url)
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .body(format!("{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{{\"pad\":\"{padding}\"}}}}"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 413);
    s.stop().await;
}

/// REQ-MCP-007 over HTTP: a client that drops the connection mid-call
/// cancels the call; it is counted as cancelled and nothing is returned.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_mcp_007_dropped_connection_cancels_the_call() {
    let site = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    b"<html><body><main><p>slow</p></main></body></html>".to_vec(),
                    "text/html",
                )
                .set_delay(Duration::from_secs(5)),
        )
        .mount(&site)
        .await;
    let port = site.address().port();
    let s = Server::start(&format!(
        "config_version = 1\n[service.http]\nbind = \"127.0.0.1:0\"\n[egress]\nallow = [\"127.0.0.1/32\"]\nallow_http = true\n[fetch]\nports = [80, 443, {port}]\nrobots_txt = false\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n"
    ))
    .await;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(300))
        .build()
        .unwrap();
    let mut params =
        json!({ "name": "web_fetch", "arguments": { "url": format!("{}/slow", site.uri()) } });
    params[META] = meta();
    let dropped = client
        .post(&s.url)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", "tools/call")
        .header("mcp-name", "web_fetch")
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": params }))
        .send()
        .await;
    assert!(dropped.is_err(), "the client gave up");
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(s.service.counters().calls_of("web_fetch", "cancelled"), 1);
    assert_eq!(s.service.counters().calls_of("web_fetch", "ok"), 0);
    s.stop().await;
}

/// REQ-SEC-018: the token comparison has the same shape for every input:
/// equal and unequal lengths, a difference at the first or the last byte.
#[test]
fn req_sec_018_token_comparison_is_constant_time_in_structure() {
    assert!(constant_time_eq(b"abc", b"abc"));
    assert!(!constant_time_eq(b"abc", b"abd"));
    assert!(!constant_time_eq(b"abc", b"xbc"));
    assert!(!constant_time_eq(b"abc", b"abcd"));
    assert!(!constant_time_eq(b"", b"a"));
    assert!(constant_time_eq(b"", b""));
}

/// REQ-OPS-002 over HTTP: cancelling the shutdown token closes the
/// listener within LIM-O1 and the run future ends.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_ops_002_shutdown_closes_the_listener() {
    let s = Server::start(LOOPBACK).await;
    let url = s.url.clone();
    let started = std::time::Instant::now();
    s.shutdown.cancel();
    let outcome = tokio::time::timeout(Duration::from_millis(u64::from(SHUTDOWN_MAX_MS)), s.task)
        .await
        .expect("the listener closed within LIM-O1");
    assert!(outcome.unwrap().is_ok());
    assert!(started.elapsed() < Duration::from_millis(u64::from(SHUTDOWN_MAX_MS)));
    let refused = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap()
        .post(&url)
        .send()
        .await;
    assert!(refused.is_err(), "nothing listens any more");
}
