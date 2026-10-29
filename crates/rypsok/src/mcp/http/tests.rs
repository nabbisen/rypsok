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
    sessions: Arc<BoundedSessions<LocalSessionManager>>,
    shutdown: CancellationToken,
    task: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl Server {
    async fn start(config_text: &str) -> Server {
        Self::start_with(config_text, None).await
    }

    /// Starts a listener whose service counts into `counters` when given.
    async fn start_with(
        config_text: &str,
        counters: Option<Arc<rypsok_core::telemetry::Counters>>,
    ) -> Server {
        let config = Config::parse(config_text).unwrap();
        config.validate().unwrap();
        let mut service = Rypsok::new(config).unwrap();
        if let Some(counters) = counters {
            service = service.with_counters(counters);
        }
        let service = Arc::new(service);
        let settings = HttpSettings::from_config(service.config()).unwrap();
        let listener = bind(&settings).await.unwrap();
        let url = format!("http://{}{PATH}", listener.local_addr().unwrap());
        let shutdown = CancellationToken::new();
        let adapter = Adapter::new(Arc::clone(&service));
        let sessions = Arc::new(BoundedSessions::new(
            LocalSessionManager::default(),
            settings.max_sessions as usize,
        ));
        let task = tokio::spawn(run_with(
            adapter,
            listener,
            settings,
            shutdown.clone(),
            Arc::clone(&sessions),
        ));
        Server {
            url,
            service,
            sessions,
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

    /// A legacy `initialize` (revision 2025-11-25), which opens a session.
    async fn initialize(&self) -> reqwest::Response {
        self.client()
            .post(&self.url)
            .header("accept", "application/json, text/event-stream")
            .json(&json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }
            }))
            .send()
            .await
            .unwrap()
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
    assert_eq!(
        foreign_host.status(),
        403,
        "the SDK answers a foreign Host with 403 (RFC 019 D6)"
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
        auth.check(&http::HeaderMap::new(), "ipv4").unwrap(),
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

/// REQ-SEC-018 (RFC 019 D4): lengths that differ by a multiple of 256 do
/// not alias to equal; a token followed by 256 zero bytes is another token.
#[test]
fn req_sec_018_token_comparison_has_no_length_aliasing() {
    let token = [b'A'; 32];
    for extra in [1usize, 255, 256, 257, 512, 768] {
        let mut longer = token.to_vec();
        longer.extend(std::iter::repeat_n(0u8, extra));
        assert!(!constant_time_eq(&token, &longer), "{extra} zero bytes");
        assert!(!constant_time_eq(&longer, &token), "{extra} zero bytes");
    }
    assert!(constant_time_eq(&token, &token));
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

/// REQ-SEC-022 and THR-26 (RFC 018): a client that sends its headers and
/// then a body one byte at a time is answered 408 after LIM-H5, the
/// connection permit is released, and the listener keeps serving.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_a_slow_body_is_ended_with_408() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let s = Server::start(
        "config_version = 1\n[fetch]\ndeadline_ms = 1000\n[service.http]\nbind = \"127.0.0.1:0\"\nrequest_timeout_s = 3\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n",
    )
    .await;
    let address = s.url.trim_start_matches("http://").trim_end_matches(PATH);
    let started = std::time::Instant::now();
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream
        .write_all(
            format!(
                "POST {PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2026-07-28\r\nMcp-Method: tools/call\r\nMcp-Name: engine_status\r\nContent-Length: 400\r\n\r\n{{"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut answer = Vec::new();
    let read = tokio::time::timeout(Duration::from_secs(8), stream.read_to_end(&mut answer)).await;
    assert!(read.is_ok(), "no answer within 8 s");
    let elapsed = started.elapsed();
    let text = String::from_utf8_lossy(&answer);
    assert!(text.starts_with("HTTP/1.1 408"), "{text}");
    assert!(elapsed >= Duration::from_secs(3), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(6), "{elapsed:?}");
    // The listener still serves: the permit was released.
    let response = s
        .request("server/discover", json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(s.service.counters().calls_of("engine_status", "ok"), 0);
    assert_eq!(
        s.service.counters().refusals_of("http", "request_timeout"),
        1,
        "the 408 is counted (RFC 019 D6)"
    );
    s.stop().await;
}

/// What a legacy handshake answered: status, session id, `Retry-After`,
/// body.
async fn handshake(s: &Server) -> (u16, Option<String>, Option<String>, String) {
    let response = s.initialize().await;
    let status = response.status().as_u16();
    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .map(|v| v.to_str().unwrap().to_owned())
    };
    let (session, retry) = (header("mcp-session-id"), header("retry-after"));
    (status, session, retry, response.text().await.unwrap())
}

/// REQ-SEC-024 (RFC 019 D5): legacy sessions are bounded. With a bound of
/// three, three handshakes open a session and the fourth is answered 503
/// with `Retry-After: 30`, an empty body and no session; the stateless
/// revision is unaffected; closing a session (`DELETE`) frees its slot; the
/// count equals the live sessions throughout.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_024_sessions_are_bounded() {
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nmax_sessions = 3\n",
    ))
    .await;
    let mut ids = Vec::new();
    for round in 1..=3 {
        let (status, session, _, body) = handshake(&s).await;
        assert_eq!(status, 200, "handshake {round}: {body}");
        ids.push(session.expect("a session id"));
        assert_eq!(s.sessions.live(), round);
    }
    let (status, session, retry, body) = handshake(&s).await;
    assert_eq!(status, 503, "{body}");
    assert_eq!(session, None, "no session is opened");
    assert_eq!(retry.as_deref(), Some("30"));
    assert_eq!(body, "", "the SDK's own text is not passed on");
    assert_eq!(s.sessions.live(), 3, "a refusal changes no count");

    // The stateless revision keeps no session and is not counted.
    let discover = s
        .request("server/discover", json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(discover.status(), 200);
    assert_eq!(s.sessions.live(), 3);

    // Closing a session frees its slot.
    let closed = s
        .client()
        .delete(&s.url)
        .header("mcp-session-id", &ids[0])
        .send()
        .await
        .unwrap();
    assert!(closed.status().is_success(), "{}", closed.status());
    let waited = std::time::Instant::now();
    while s.sessions.live() != 2 {
        assert!(
            waited.elapsed() < Duration::from_secs(5),
            "the slot is freed"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let (status, session, _, body) = handshake(&s).await;
    assert_eq!(status, 200, "{body}");
    assert!(session.is_some());
    assert_eq!(s.sessions.live(), 3);
    s.stop().await;
}

/// REQ-SEC-024 and REQ-OBS-005 (RFC 019 D5): a refused handshake is logged
/// once as `http_refused` with the reason `session_limit` and the address
/// family, and with nothing of the request. (A current-thread runtime, so
/// that the test's subscriber sees the listener's tasks.)
#[tokio::test]
async fn req_sec_024_a_refused_handshake_is_logged() {
    let (log, _guard) = capture_log();
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nmax_sessions = 1\n",
    ))
    .await;
    assert_eq!(handshake(&s).await.0, 200);
    log.lock().unwrap().clear();
    assert_eq!(handshake(&s).await.0, 503);
    let text = String::from_utf8(log.lock().unwrap().clone()).unwrap();
    let refused: Vec<Value> = text
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v["event"] == "http_refused")
        .collect();
    assert_eq!(refused.len(), 1, "{text}");
    assert_eq!(refused[0]["reason"], "session_limit");
    assert_eq!(refused[0]["family"], "ipv4");
    s.stop().await;
}

/// A subscriber for the current thread whose output the test can read.
fn capture_log() -> (
    Arc<std::sync::Mutex<Vec<u8>>>,
    tracing::subscriber::DefaultGuard,
) {
    use std::io::Write;
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct Sink(Arc<std::sync::Mutex<Vec<u8>>>);
    struct SinkWriter(Arc<std::sync::Mutex<Vec<u8>>>);
    impl Write for SinkWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for Sink {
        type Writer = SinkWriter;
        fn make_writer(&'a self) -> SinkWriter {
            SinkWriter(Arc::clone(&self.0))
        }
    }

    let sink = Sink::default();
    let buffer = Arc::clone(&sink.0);
    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_target(false)
        .without_time()
        .with_writer(sink)
        .finish();
    (buffer, tracing::subscriber::set_default(subscriber))
}

/// The events of one name in captured JSON log lines.
fn events(log: &Arc<std::sync::Mutex<Vec<u8>>>, name: &str) -> Vec<Value> {
    let text = String::from_utf8(log.lock().unwrap().clone()).unwrap();
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v["event"] == name)
        .collect()
}

/// A request of the 2026-07-28 revision without its `MCP-Protocol-Version`
/// header.
fn without_protocol_header(s: &Server) -> reqwest::RequestBuilder {
    s.client()
        .post(&s.url)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-method", "server/discover")
        .json(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "server/discover",
            "params": { META: meta() }
        }))
}

/// REQ-SEC-018 and REQ-OBS-007 (RFC 019 D6): the four answers the SDK gives
/// behind the gate, pinned. A foreign `Host` and a refused `Origin` are 403,
/// the signature of THR-18, and leave one `http_refused` event
/// (`sdk_forbidden`, the status, the address family and no header value);
/// a missing protocol header is 400 and an `Accept` without both media
/// types is 406, counted and not logged per request.
#[tokio::test]
async fn req_sec_018_sdk_refusals_are_pinned_logged_and_counted() {
    let (log, _guard) = capture_log();
    let s = Server::start(LOOPBACK).await;
    let counters = s.service.counters();

    let host = s
        .request("server/discover", json!({}))
        .header("host", "evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(host.status(), 403);
    let origin = s
        .request("server/discover", json!({}))
        .header("origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(origin.status(), 403);
    let forbidden = events(&log, "http_refused");
    assert_eq!(forbidden.len(), 2, "{forbidden:?}");
    for event in &forbidden {
        assert_eq!(event["reason"], "sdk_forbidden");
        assert_eq!(event["status"], 403);
        assert_eq!(event["family"], "ipv4");
    }
    // The gate's own event carries no header value. (The SDK writes its own
    // warning for the same request, with the value; that line is the SDK's.)
    assert!(
        !serde_json::to_string(&forbidden)
            .unwrap()
            .contains("evil.example"),
        "no header value in the gate's event"
    );
    assert_eq!(counters.refusals_of("http", "sdk_forbidden"), 2);

    log.lock().unwrap().clear();
    let bad = without_protocol_header(&s).send().await.unwrap();
    assert_eq!(bad.status(), 400);
    let not_acceptable = s
        .client()
        .post(&s.url)
        .header("accept", "application/json")
        .header("mcp-protocol-version", "2026-07-28")
        .header("mcp-method", "server/discover")
        .json(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "server/discover",
            "params": { META: meta() }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(not_acceptable.status(), 406);
    assert!(
        events(&log, "http_refused").is_empty(),
        "400 and 406 are counted, not logged"
    );
    assert_eq!(counters.refusals_of("http", "sdk_bad_request"), 1);
    assert_eq!(counters.refusals_of("http", "sdk_not_acceptable"), 1);
    s.stop().await;
}

/// REQ-OBS-007 (RFC 019 D6): the gate's own refusals are counted per
/// reason: 401, a declared body over LIM-H2 (413) and a refused handshake.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_obs_007_gate_refusals_are_counted_per_reason() {
    let token = token_file("counted", "token-c-0123456789-0123456789-0123456789");
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n[engines.w]",
        &format!(
            "bind = \"127.0.0.1:0\"\nmax_request_bytes = 4096\nmax_sessions = 1\n[[service.http.auth.callers]]\nid = \"alpha\"\ntoken_file = \"{token}\"\n[engines.w]"
        ),
    ))
    .await;
    let counters = s.service.counters();
    let auth = "token-c-0123456789-0123456789-0123456789";
    assert_eq!(
        s.request("server/discover", json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let padding = "x".repeat(5000);
    let big = s
        .client()
        .post(&s.url)
        .bearer_auth(auth)
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .body(format!("{{\"pad\":\"{padding}\"}}"))
        .send()
        .await
        .unwrap();
    assert_eq!(big.status(), 413);
    let initialize = |s: &Server| {
        s.client()
            .post(&s.url)
            .bearer_auth(auth)
            .header("accept", "application/json, text/event-stream")
            .json(&json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }
            }))
            .send()
    };
    assert_eq!(initialize(&s).await.unwrap().status(), 200);
    assert_eq!(initialize(&s).await.unwrap().status(), 503);
    assert_eq!(counters.refusals_of("http", "unauthorized"), 1);
    assert_eq!(counters.refusals_of("http", "body_too_large"), 1);
    assert_eq!(counters.refusals_of("http", "session_limit"), 1);
    s.stop().await;
}

/// REQ-OBS-007 (RFC 019 D6): refused requests inside the flush interval
/// are counted and leave no event. An interval of an hour stands for "the
/// minute has not passed", whatever the speed of the machine. (A
/// current-thread runtime, so that the test's subscriber sees the
/// listener's tasks.)
#[tokio::test]
async fn req_obs_007_refusals_inside_the_interval_are_counted_not_logged() {
    use rypsok_core::telemetry::Counters;
    let (log, _guard) = capture_log();
    let counters = Arc::new(Counters::with_flush_interval(Duration::from_secs(3600)));
    let s = Server::start_with(LOOPBACK, Some(Arc::clone(&counters))).await;
    for _ in 0..3 {
        let response = s
            .request("server/discover", json!({}))
            .header("host", "evil.example")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
    }
    assert_eq!(counters.refusals_of("http", "sdk_forbidden"), 3);
    assert!(
        events(&log, "counters").is_empty(),
        "inside the interval nothing is flushed"
    );
    s.stop().await;
}

/// REQ-OBS-007 (RFC 019 D6): a flood of refused requests with no tool call
/// is still logged: counting a refusal runs the flush check a call runs.
/// An interval of zero stands for "the minute has passed": each refusal,
/// whatever its reason, makes a `counters` event with the totals so far, and
/// the last one lists all three. No sleep, so no dependence on the speed of
/// the machine. (A current-thread runtime, so that the test's subscriber
/// sees the listener's tasks.)
#[tokio::test]
async fn req_obs_007_a_flood_of_refusals_is_logged_without_a_tool_call() {
    use rypsok_core::telemetry::Counters;
    let (log, _guard) = capture_log();
    let counters = Arc::new(Counters::with_flush_interval(Duration::ZERO));
    let s = Server::start_with(LOOPBACK, Some(counters)).await;
    for _ in 0..3 {
        let response = s
            .request("server/discover", json!({}))
            .header("host", "evil.example")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
    }
    assert_eq!(
        without_protocol_header(&s).send().await.unwrap().status(),
        400
    );
    let flushed = events(&log, "counters");
    assert!(!flushed.is_empty(), "no event: the refusal did not flush");
    let refusals = flushed.last().unwrap()["refusals"].as_str().unwrap();
    assert!(refusals.contains("http.sdk_forbidden=3"), "{refusals}");
    assert!(refusals.contains("http.sdk_bad_request=1"), "{refusals}");
    assert!(
        s.service.counters().calls_of("engine_status", "ok") == 0
            && s.service.counters().calls_of("web_search", "ok") == 0,
        "no tool call was made"
    );
    s.stop().await;
}

/// REQ-SEC-024 (RFC 019 D5) over HTTP/2: hyper serves each stream in a task
/// of its own, and the gate's cell must still reach the session wrapper
/// inside it, so the refusal is a 503 here as well, not the SDK's 500.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_024_a_refused_handshake_is_503_over_http2_too() {
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nmax_sessions = 1\n",
    ))
    .await;
    let client = reqwest::Client::builder()
        .no_proxy()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } }
    });
    let mut statuses = Vec::new();
    for _ in 0..2 {
        let response = client
            .post(&s.url)
            .header("accept", "application/json, text/event-stream")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.version(), reqwest::Version::HTTP_2);
        statuses.push((
            response.status().as_u16(),
            response
                .headers()
                .get("retry-after")
                .map(|v| v.to_str().unwrap().to_owned()),
            response.text().await.unwrap(),
        ));
    }
    assert_eq!(statuses[0].0, 200, "{statuses:?}");
    assert_eq!(statuses[1], (503, Some("30".to_owned()), String::new()));
    s.stop().await;
}
