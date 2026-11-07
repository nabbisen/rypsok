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
use rypsok_core::limits::defaults::SHUTDOWN_MAX_MS;

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
        Self::start_tuned(config_text, counters, |_| {}).await
    }

    /// Starts a listener after `tune` has changed its settings: the way a
    /// test uses a bound the configuration check would not admit, such as
    /// an idle time of a few hundred milliseconds (the range starts at 5 s).
    async fn start_tuned(
        config_text: &str,
        counters: Option<Arc<rypsok_core::telemetry::Counters>>,
        tune: impl FnOnce(&mut HttpSettings),
    ) -> Server {
        let config = Config::parse(config_text).unwrap();
        config.validate().unwrap();
        let mut service = Rypsok::new(config).unwrap();
        if let Some(counters) = counters {
            service = service.with_counters(counters);
        }
        let service = Arc::new(service);
        let mut settings = HttpSettings::from_config(service.config()).unwrap();
        tune(&mut settings);
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
        assert_eq!(sessions::tests::live(&s.sessions), round);
    }
    let (status, session, retry, body) = handshake(&s).await;
    assert_eq!(status, 503, "{body}");
    assert_eq!(session, None, "no session is opened");
    assert_eq!(retry.as_deref(), Some("30"));
    assert_eq!(body, "", "the SDK's own text is not passed on");
    assert_eq!(
        sessions::tests::live(&s.sessions),
        3,
        "a refusal changes no count"
    );

    // The stateless revision keeps no session and is not counted.
    let discover = s
        .request("server/discover", json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(discover.status(), 200);
    assert_eq!(sessions::tests::live(&s.sessions), 3);

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
    while sessions::tests::live(&s.sessions) != 2 {
        assert!(
            waited.elapsed() < Duration::from_secs(5),
            "the slot is freed"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let (status, session, _, body) = handshake(&s).await;
    assert_eq!(status, 200, "{body}");
    assert!(session.is_some());
    assert_eq!(sessions::tests::live(&s.sessions), 3);
    s.stop().await;
}

thread_local! {
    /// Where the events written on this thread go: the sink of the test that
    /// runs on it, if it asked for one.
    static SINK: std::cell::RefCell<Option<Arc<std::sync::Mutex<Vec<u8>>>>> =
        const { std::cell::RefCell::new(None) };
}

/// The writer of the one global subscriber of this test process: it hands
/// each line to the sink of the thread that wrote it.
#[derive(Clone, Copy)]
struct Routed;

struct RoutedWriter(Option<Arc<std::sync::Mutex<Vec<u8>>>>);

impl std::io::Write for RoutedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(sink) = &self.0 {
            sink.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Routed {
    type Writer = RoutedWriter;

    fn make_writer(&'a self) -> RoutedWriter {
        RoutedWriter(SINK.with(|sink| sink.borrow().clone()))
    }
}

/// Ends the capture of the test's thread.
struct Capture;

impl Drop for Capture {
    fn drop(&mut self) {
        SINK.with(|sink| *sink.borrow_mut() = None);
    }
}

/// The log of the test's own thread, read as the process writes it (the
/// layer the process installs, so span fields such as `request_id` and
/// `caller` are top-level).
///
/// The subscriber is **global and installed once** for the whole test
/// process, and only the sink it writes to is per thread. A thread-scoped
/// subscriber per test raced: `tracing` caches whether a callsite is enabled
/// for the process, and a callsite that another test's thread reached without
/// a subscriber could be cached as disabled, so the event went missing
/// (correction R3 of the 020r review). With one dispatcher for the process
/// there is nothing to race. A test that uses this runs on a current-thread
/// runtime, so that the tasks of the listener write on its thread; the
/// tests that can be run against a spawned server are in the protocol tests
/// instead.
fn routed_log() -> (Arc<std::sync::Mutex<Vec<u8>>>, Capture) {
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::prelude::*;
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry()
                .with(rypsok_core::telemetry::layer(Routed).with_filter(LevelFilter::INFO)),
        )
        .expect("no other test of this process installs a global subscriber");
    });
    let sink = Arc::new(std::sync::Mutex::new(Vec::new()));
    SINK.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&sink)));
    (sink, Capture)
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
/// behind the gate, pinned and counted: a foreign `Host` and a refused
/// `Origin` are 403, the signature of THR-18; a missing protocol header is
/// 400 and an `Accept` without both media types is 406. (What the log says
/// about them is read from a spawned server, in the protocol test
/// `req_sec_018_sdk_refusals_are_logged_once_each_without_header_values`.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_018_sdk_refusals_are_pinned_and_counted() {
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
    assert_eq!(counters.refusals_of("http", "sdk_forbidden"), 2);

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
    let (log, _guard) = routed_log();
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
    let (log, _guard) = routed_log();
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

// ---------------------------------------------------------------------------
// The bounds that had no test (handoff 020g item 1; audit NET-7): LIM-H1,
// LIM-H3 and LIM-H4 observed as behavior, on raw sockets.
// ---------------------------------------------------------------------------

/// `host:port` of a started server.
fn address(s: &Server) -> String {
    s.url
        .trim_start_matches("http://")
        .trim_end_matches(PATH)
        .to_owned()
}

/// What happened to a connection that was watched for a while.
#[derive(Debug)]
enum Watched {
    /// The server closed it after this long; the bytes it sent first.
    Closed(Duration, Vec<u8>),
    /// Still open at the end of the watch.
    Open,
}

/// Reads from `stream` until the server closes it or `limit` has passed.
async fn watch(stream: &mut (impl tokio::io::AsyncRead + Unpin), limit: Duration) -> Watched {
    use tokio::io::AsyncReadExt;
    let started = std::time::Instant::now();
    let mut received = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let left = limit.saturating_sub(started.elapsed());
        match tokio::time::timeout(left, stream.read(&mut buffer)).await {
            Err(_) => return Watched::Open,
            Ok(Ok(0)) | Ok(Err(_)) => return Watched::Closed(started.elapsed(), received),
            Ok(Ok(n)) => received.extend_from_slice(&buffer[..n]),
        }
    }
}

/// A legitimate request on a fresh connection is answered 200, within
/// `within`; connections that are refused or still being freed are retried.
async fn answered_within(s: &Server, within: Duration) -> bool {
    let started = std::time::Instant::now();
    while started.elapsed() < within {
        if let Ok(response) = s.request("server/discover", json!({})).send().await
            && response.status() == 200
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

/// A server with `max_connections` and an idle time of 400 ms (a settings
/// override; the configuration's range starts at 5 s).
async fn short_idle_server(max_connections: u32) -> Server {
    Server::start_tuned(
        &LOOPBACK.replace(
            "bind = \"127.0.0.1:0\"\n",
            &format!("bind = \"127.0.0.1:0\"\nmax_connections = {max_connections}\n"),
        ),
        None,
        |settings| settings.idle_timeout = Duration::from_millis(400),
    )
    .await
}

/// REQ-SEC-018 (LIM-H1): with `max_connections = 2` a third connection is
/// closed without a response and counted as `too_many_connections`; once
/// one connection closes, a new one works.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_018_the_connection_cap_refuses_the_third_and_frees_on_close() {
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nmax_connections = 2\n",
    ))
    .await;
    let first = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    let second = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    // The accept loop takes connections in order; the third finds no permit.
    let mut third = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    match watch(&mut third, Duration::from_secs(3)).await {
        Watched::Closed(_, bytes) => assert!(bytes.is_empty(), "no response: {bytes:?}"),
        Watched::Open => panic!("the third connection was not closed"),
    }
    assert_eq!(
        s.service
            .counters()
            .refusals_of("http", "too_many_connections"),
        1
    );
    // The two held connections still hold their permits: a request is
    // refused (closed without an answer) while both are open.
    let refused = s
        .client()
        .post(&s.url)
        .json(&json!({}))
        .send()
        .await
        .is_err();
    assert!(refused, "a fourth connection must not be served");
    drop(first);
    assert!(
        answered_within(&s, Duration::from_secs(3)).await,
        "a new connection works after one closed"
    );
    drop(second);
    s.stop().await;
}

/// REQ-SEC-022 (LIM-H3), first case: a connection that sends **nothing** is
/// closed after the idle time and frees its permit (NET-1).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_a_silent_connection_is_closed_after_the_idle_time() {
    let s = short_idle_server(1).await;
    let mut silent = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    match watch(&mut silent, Duration::from_secs(4)).await {
        Watched::Closed(after, bytes) => {
            assert!(
                after >= Duration::from_millis(300),
                "closed too early: {after:?}"
            );
            assert!(bytes.is_empty(), "no response to nothing: {bytes:?}");
        }
        Watched::Open => panic!("a silent connection was still open after 4 s"),
    }
    assert!(
        answered_within(&s, Duration::from_secs(3)).await,
        "the permit is free again"
    );
    s.stop().await;
}

/// REQ-SEC-022 (LIM-H3), NET-1 as the audit stated it: `max_connections`
/// silent connections do not keep a legitimate caller out for longer than
/// the idle time.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_silent_connections_cannot_hold_every_permit() {
    let s = short_idle_server(4).await;
    let mut silent = Vec::new();
    for _ in 0..4 {
        silent.push(tokio::net::TcpStream::connect(address(&s)).await.unwrap());
    }
    // At once every permit is held and the caller is turned away ...
    assert!(
        s.request("server/discover", json!({}))
            .send()
            .await
            .is_err()
    );
    // ... and after the idle time the caller is served, with the silent
    // connections still open on the client side.
    assert!(
        answered_within(&s, Duration::from_secs(4)).await,
        "the silent connections still hold the permits"
    );
    drop(silent);
    s.stop().await;
}

/// What a client sends before it stalls, for the protocol-detection tests:
/// the whole connection must be closed after the idle time and free its
/// permit (G1 of the 020g review). `counted` says whether the close belongs
/// to the first-bytes deadline (counted as `idle_connection`) or to hyper's
/// own header timeout once HTTP/1 is decided (not counted, like a partial
/// header).
async fn stalls_after_sending(prefix: &[u8], counted: bool) {
    use tokio::io::AsyncWriteExt;
    let s = short_idle_server(1).await;
    let mut stalled = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    stalled.write_all(prefix).await.unwrap();
    match watch(&mut stalled, Duration::from_secs(4)).await {
        Watched::Closed(after, bytes) => {
            assert!(
                after >= Duration::from_millis(300),
                "closed too early: {after:?}"
            );
            assert!(bytes.is_empty(), "no response to {prefix:?}: {bytes:?}");
        }
        Watched::Open => panic!("{prefix:?} then silence: still open after 4 s"),
    }
    assert!(
        answered_within(&s, Duration::from_secs(3)).await,
        "the permit is free again after {prefix:?}"
    );
    assert_eq!(
        s.service.counters().refusals_of("http", "idle_connection"),
        u64::from(counted),
        "counted under the same reason: {counted}"
    );
    s.stop().await;
}

/// REQ-SEC-022 (LIM-H3), G1: one byte that is both the start of `POST` and
/// of the HTTP/2 preface decides nothing; the connection is still closed
/// after the idle time.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_one_ambiguous_byte_then_silence_is_closed_after_the_idle_time() {
    stalls_after_sending(b"P", true).await;
}

/// REQ-SEC-022 (LIM-H3), G1: `PO` already differs from `PRI` at the second
/// byte, so HTTP/1 is decided and hyper's header timeout closes it (it
/// passed on the tree before the fix; kept as the boundary of the case).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_two_bytes_that_decide_http1_then_silence_is_closed_after_the_idle_time() {
    stalls_after_sending(b"PO", false).await;
}

/// REQ-SEC-022 (LIM-H3), G1: 21 of the 24 bytes of the HTTP/2 preface.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_a_partial_http2_preface_then_silence_is_closed_after_the_idle_time() {
    stalls_after_sending(b"PRI * HTTP/2.0\r\n\r\nSM\r\n", true).await;
}

/// REQ-SEC-022 (LIM-H3), G1: a valid preface sent one byte every third of
/// the idle time is closed at the idle time from accept, not extended by
/// each byte.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_a_trickled_preface_does_not_extend_the_deadline() {
    use tokio::io::AsyncWriteExt;
    const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
    let s = short_idle_server(1).await;
    let mut trickle = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    let started = std::time::Instant::now();
    let mut closed_at = None;
    for byte in PREFACE {
        if trickle.write_all(&[*byte]).await.is_err() {
            closed_at = Some(started.elapsed());
            break;
        }
        tokio::time::sleep(Duration::from_millis(130)).await;
        if started.elapsed() > Duration::from_secs(4) {
            break;
        }
    }
    let closed_at = match closed_at {
        Some(at) => at,
        None => match watch(&mut trickle, Duration::from_secs(1)).await {
            Watched::Closed(_, _) => started.elapsed(),
            Watched::Open => panic!("the trickle held the connection open"),
        },
    };
    // 24 bytes at 130 ms would take 3.1 s; the deadline is 400 ms.
    assert!(
        closed_at < Duration::from_millis(1500),
        "closed only after {closed_at:?}"
    );
    assert!(
        answered_within(&s, Duration::from_secs(3)).await,
        "the permit is free again"
    );
    s.stop().await;
}

/// REQ-SEC-022 (LIM-H3), second case: a connection that sent a **partial
/// header** is closed after the idle time and frees its permit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_a_partial_header_is_closed_after_the_idle_time() {
    use tokio::io::AsyncWriteExt;
    let s = short_idle_server(1).await;
    let mut slow = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    slow.write_all(b"POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Slow: ")
        .await
        .unwrap();
    match watch(&mut slow, Duration::from_secs(4)).await {
        Watched::Closed(after, _) => {
            assert!(
                after >= Duration::from_millis(300),
                "closed too early: {after:?}"
            );
        }
        Watched::Open => panic!("a partial header was still open after 4 s"),
    }
    assert!(
        answered_within(&s, Duration::from_secs(3)).await,
        "the permit is free again"
    );
    s.stop().await;
}

/// REQ-SEC-022 (LIM-H3), third case: a keep-alive connection that is idle
/// **between requests** is closed after the idle time and frees its permit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_an_idle_keep_alive_connection_is_closed_after_the_idle_time() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let s = short_idle_server(1).await;
    let mut kept = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "server/discover",
        "params": { META: meta() }
    })
    .to_string();
    kept.write_all(
        format!(
            "POST {PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2026-07-28\r\nMcp-Method: server/discover\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    )
    .await
    .unwrap();
    // Read the whole answer: the head, then `Content-Length` bytes.
    let mut answer = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let n = tokio::time::timeout(Duration::from_secs(3), kept.read(&mut buffer))
            .await
            .expect("an answer within 3 s")
            .unwrap();
        assert!(n > 0, "closed before the answer");
        answer.extend_from_slice(&buffer[..n]);
        let text = String::from_utf8_lossy(&answer).into_owned();
        if let Some(head_end) = text.find("\r\n\r\n") {
            let length = text[..head_end]
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .expect("a content-length");
            if answer.len() >= head_end + 4 + length {
                break;
            }
        }
    }
    assert!(String::from_utf8_lossy(&answer).starts_with("HTTP/1.1 200"));
    match watch(&mut kept, Duration::from_secs(4)).await {
        Watched::Closed(after, bytes) => {
            assert!(
                after >= Duration::from_millis(300),
                "closed too early: {after:?}"
            );
            assert!(bytes.is_empty(), "nothing more is sent: {bytes:?}");
        }
        Watched::Open => panic!("an idle keep-alive connection was still open after 4 s"),
    }
    assert!(
        answered_within(&s, Duration::from_secs(3)).await,
        "the permit is free again"
    );
    s.stop().await;
}

/// One HTTP/2 frame: length, type, flags, stream, payload.
fn h2_frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let length = u32::try_from(payload.len()).unwrap().to_be_bytes();
    let mut frame = vec![length[1], length[2], length[3], kind, flags];
    frame.extend_from_slice(&stream.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// A header block of a POST to `/mcp` that promises a body and never sends
/// it, written as HPACK literals (RFC 7541 §6.2.2, no Huffman, no index).
fn h2_open_request_headers() -> Vec<u8> {
    fn literal(block: &mut Vec<u8>, name: &str, value: &str) {
        block.push(0x00);
        block.push(u8::try_from(name.len()).unwrap());
        block.extend_from_slice(name.as_bytes());
        block.push(u8::try_from(value.len()).unwrap());
        block.extend_from_slice(value.as_bytes());
    }
    let mut block = vec![0x83, 0x86]; // :method POST, :scheme http
    block.extend_from_slice(&[0x04, 4]); // :path, literal value
    block.extend_from_slice(PATH.as_bytes());
    block.extend_from_slice(&[0x01, 9]); // :authority
    block.extend_from_slice(b"127.0.0.1");
    literal(&mut block, "content-type", "application/json");
    literal(&mut block, "accept", "application/json, text/event-stream");
    literal(&mut block, "mcp-protocol-version", "2026-07-28");
    literal(&mut block, "mcp-method", "server/discover");
    literal(&mut block, "content-length", "1000");
    block
}

/// Reads one frame (kind, flags, stream, payload), or `None` at the end of
/// the stream or after `limit`.
async fn h2_read_frame(
    stream: &mut tokio::net::TcpStream,
    limit: Duration,
) -> Option<(u8, u8, u32, Vec<u8>)> {
    use tokio::io::AsyncReadExt;
    let mut head = [0u8; 9];
    tokio::time::timeout(limit, stream.read_exact(&mut head))
        .await
        .ok()?
        .ok()?;
    let length = u32::from_be_bytes([0, head[0], head[1], head[2]]) as usize;
    let id = u32::from_be_bytes([head[5] & 0x7f, head[6], head[7], head[8]]);
    let mut payload = vec![0u8; length];
    tokio::time::timeout(limit, stream.read_exact(&mut payload))
        .await
        .ok()?
        .ok()?;
    Some((head[3], head[4], id, payload))
}

/// REQ-SEC-022 (LIM-H4): with `max_streams = 2` the server advertises
/// `SETTINGS_MAX_CONCURRENT_STREAMS = 2`, serves two open streams and
/// **refuses the third with `RST_STREAM(REFUSED_STREAM)`**: that is what
/// the HTTP/2 implementation under hyper does (it does not queue the stream
/// and does not close the connection), seen here with a client that ignores
/// the advertised limit, because a compliant client queues the request on its
/// own side and never lets the server see it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_022_an_http2_stream_beyond_max_streams_is_refused() {
    use tokio::io::AsyncWriteExt;
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nmax_streams = 2\n",
    ))
    .await;
    let mut stream = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    stream
        .write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .await
        .unwrap();
    stream.write_all(&h2_frame(0x4, 0, 0, &[])).await.unwrap();

    // The server's SETTINGS: MAX_CONCURRENT_STREAMS (id 3) is max_streams.
    let mut advertised = None;
    while advertised.is_none() {
        let (kind, flags, _, payload) = h2_read_frame(&mut stream, Duration::from_secs(3))
            .await
            .expect("the server's SETTINGS");
        if kind == 0x4 && flags & 0x1 == 0 {
            for entry in payload.chunks(6) {
                if u16::from_be_bytes([entry[0], entry[1]]) == 3 {
                    advertised = Some(u32::from_be_bytes([entry[2], entry[3], entry[4], entry[5]]));
                }
            }
            stream.write_all(&h2_frame(0x4, 0x1, 0, &[])).await.unwrap();
            break;
        }
    }
    assert_eq!(advertised, Some(2), "the advertised stream limit");

    // Streams 1 and 3 are opened and left waiting for their bodies.
    let headers = h2_open_request_headers();
    for id in [1u32, 3] {
        stream
            .write_all(&h2_frame(0x1, 0x4, id, &headers))
            .await
            .unwrap();
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    // Stream 5 is one too many.
    stream
        .write_all(&h2_frame(0x1, 0x4, 5, &headers))
        .await
        .unwrap();
    let mut refused = false;
    while let Some((kind, _, id, payload)) =
        h2_read_frame(&mut stream, Duration::from_secs(3)).await
    {
        match kind {
            0x3 if id == 5 => {
                let code = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                assert_eq!(code, 0x7, "REFUSED_STREAM");
                refused = true;
                break;
            }
            0x7 => panic!("the connection was closed (GOAWAY) instead of the stream refused"),
            _ => {}
        }
    }
    assert!(refused, "stream 5 was not refused");
    s.stop().await;
}

/// REQ-SEC-022 and REQ-OBS-007 (handoff 020g, NET-1): a silent connection
/// closed after the idle time is counted as `http.idle_connection` and
/// writes **no log line**, so that a flood of silent connections cannot write
/// a flood of lines. (A current-thread runtime, so that the test's
/// subscriber sees the listener's tasks.)
#[tokio::test]
async fn req_sec_022_idle_closes_are_counted_and_not_logged() {
    use rypsok_core::telemetry::Counters;
    let (log, _guard) = routed_log();
    let counters = Arc::new(Counters::with_flush_interval(Duration::from_secs(3600)));
    let s = Server::start_tuned(LOOPBACK, Some(Arc::clone(&counters)), |settings| {
        settings.idle_timeout = Duration::from_millis(300);
    })
    .await;
    let mut silent = Vec::new();
    for _ in 0..5 {
        silent.push(tokio::net::TcpStream::connect(address(&s)).await.unwrap());
    }
    for stream in &mut silent {
        assert!(matches!(
            watch(stream, Duration::from_secs(4)).await,
            Watched::Closed(..)
        ));
    }
    assert_eq!(counters.refusals_of("http", "idle_connection"), 5);
    assert!(
        events(&log, "http_refused").is_empty(),
        "no per-connection log line: {}",
        String::from_utf8_lossy(&log.lock().unwrap())
    );
    s.stop().await;
}

/// REQ-SEC-018 (handoff 020g, NET-2): after an accept error the loop waits
/// 100 ms, and the error is logged at most once per 10 s, with the number of
/// errors since the last line.
#[test]
fn req_sec_018_accept_errors_back_off_and_are_logged_once_per_interval() {
    let start = std::time::Instant::now();
    let mut backoff = AcceptBackoff::default();
    assert_eq!(backoff.delay(), Duration::from_millis(100));
    // The first error is logged, alone.
    assert_eq!(backoff.should_log(start), Some(1));
    // Errors inside the interval are counted, not logged.
    for second in 1..10u64 {
        assert_eq!(
            backoff.should_log(start + Duration::from_secs(second)),
            None
        );
    }
    // The first error at or after 10 s is logged with the nine before it
    // and itself: 9 suppressed (seconds 1 to 9) + 1.
    assert_eq!(
        backoff.should_log(start + Duration::from_secs(10)),
        Some(10)
    );
    // The count starts again, and so does the interval.
    assert_eq!(backoff.should_log(start + Duration::from_secs(11)), None);
    assert_eq!(backoff.should_log(start + Duration::from_secs(19)), None);
    assert_eq!(backoff.should_log(start + Duration::from_secs(20)), Some(3));
}

/// REQ-SEC-018: a clock that does not move forward never logs twice, and the
/// delay does not depend on the history.
#[test]
fn req_sec_018_accept_backoff_survives_a_clock_that_stands_still() {
    let now = std::time::Instant::now();
    let mut backoff = AcceptBackoff::default();
    assert_eq!(backoff.should_log(now), Some(1));
    for _ in 0..1000 {
        assert_eq!(backoff.should_log(now), None);
        assert_eq!(backoff.delay(), Duration::from_millis(100));
    }
    assert_eq!(
        backoff.should_log(now + ACCEPT_ERROR_LOG_INTERVAL),
        Some(1001)
    );
}

/// REQ-SEC-018 (handoff 020g, NET-11): the `Bearer` scheme is matched
/// case-insensitively (RFC 9110 §11.1), followed by one or more spaces. A
/// tab or another separator, no scheme, another scheme and an empty value
/// stay refused with 401.
#[test]
fn req_sec_018_the_bearer_scheme_is_case_insensitive() {
    const TOKEN: &str = "token-a-0123456789-0123456789-0123456789";
    let auth = Auth {
        callers: vec![Caller {
            id: CallerId::new("alpha"),
            token: Secret::new(TOKEN.to_owned()),
        }],
    };
    let check = |value: &str| {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(value).expect("a header value"),
        );
        auth.check(&headers, "ipv4").is_ok()
    };
    for scheme in ["Bearer", "bearer", "BEARER", "bEaReR"] {
        assert!(check(&format!("{scheme} {TOKEN}")), "{scheme}, one space");
        assert!(check(&format!("{scheme}  {TOKEN}")), "{scheme}, two spaces");
    }
    for refused in [
        format!("Bearer\t{TOKEN}"),
        format!("Bearer \t{TOKEN}"),
        format!("Bearer:{TOKEN}"),
        format!("Bearer{TOKEN}"),
        format!("Basic {TOKEN}"),
        format!("Token {TOKEN}"),
        TOKEN.to_owned(),
        "Bearer ".to_owned(),
        "Bearer".to_owned(),
        String::new(),
    ] {
        assert!(!check(&refused), "{refused:?} must be refused");
    }
    // No header at all.
    assert!(auth.check(&http::HeaderMap::new(), "ipv4").is_err());
    // A wrong token with a good scheme is still refused.
    assert!(!check("BEARER token-a-0123456789-0123456789-012345678X"));
}

/// Sends one raw HTTP/1.1 request to `/mcp` with the extra header lines and
/// returns the status line of the answer: an empty string when the
/// connection ended before the server's first byte reached the test (a reset
/// discards what the stack had not delivered yet), and `still open` when
/// nothing ended it within five seconds.
///
/// The request is written by a task of its own while this one reads, so that
/// an answer the server sends early (a 431 after 16 KiB, while the rest of
/// the header is still on its way) is read as it arrives and not after the
/// write has finished.
async fn raw_status(s: &Server, extra_headers: &str) -> String {
    use tokio::io::AsyncWriteExt;
    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "server/discover",
        "params": { META: meta() }
    })
    .to_string();
    let stream = tokio::net::TcpStream::connect(address(s)).await.unwrap();
    let (mut reader, mut writer) = stream.into_split();
    let request = format!(
        "POST {PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2026-07-28\r\nMcp-Method: server/discover\r\nConnection: close\r\n{extra_headers}Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    // The server may answer 431 and close before everything is written; the
    // write then fails, which is not the test's business.
    let write = tokio::spawn(async move {
        let _ = writer.write_all(request.as_bytes()).await;
        writer
    });
    let status = match watch(&mut reader, Duration::from_secs(5)).await {
        Watched::Closed(_, bytes) => String::from_utf8_lossy(&bytes)
            .lines()
            .next()
            .unwrap_or("")
            .to_owned(),
        Watched::Open => "still open".to_owned(),
    };
    let _ = write.await;
    status
}

/// REQ-SEC-018 (handoff 020g, NET-12; LIM-H7): the header block of an HTTP/1
/// request is bounded at 16 KiB: a header of 20,000 bytes is answered 431 by
/// hyper, and so is a single one of 400,000 bytes, which was accepted before;
/// headers below the bound still succeed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_018_an_oversized_header_block_is_431() {
    let s = Server::start(LOOPBACK).await;
    // Just above the bound: the 431 is sent after 16 KiB and the few KiB
    // that follow are still unread when the server closes. It must arrive.
    let status = raw_status(&s, &format!("X-Pad: {}\r\n", "a".repeat(20_000))).await;
    assert!(status.starts_with("HTTP/1.1 431"), "20,000 bytes: {status}");
    // Far above it: hyper answers 431 after 16 KiB and closes while the
    // client is still writing hundreds of KiB. On Windows a close with
    // unread data in the server's receive buffer sends a TCP reset, and a
    // reset makes the client's stack discard the 431 it had not read yet, so
    // the connection can end before the first byte reaches the test. That is
    // the platform's TCP, not the product: the product sends the 431, and
    // what must never happen is a served request or a connection left open.
    let status = raw_status(&s, &format!("X-Pad: {}\r\n", "a".repeat(400_000))).await;
    assert!(
        status.starts_with("HTTP/1.1 431") || status.is_empty(),
        "400,000 bytes: 431, or a reset before any byte: {status:?}"
    );
    // Ten kilobytes of headers in all are well below the bound.
    let status = raw_status(&s, &format!("X-Pad: {}\r\n", "a".repeat(10_000))).await;
    assert!(status.starts_with("HTTP/1.1 200"), "{status}");
    s.stop().await;
}

/// REQ-SEC-018 (LIM-H7): the largest headers a legitimate caller sends
/// (a 64-character token, a long `Origin` that is allowed, a long user
/// agent) are still served.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_018_the_largest_legitimate_headers_still_succeed() {
    let token = "t".repeat(64);
    let token_path = token_file("h7", &token);
    let origin = format!("https://{}.example.org", "o".repeat(200));
    let s = Server::start(&LOOPBACK.replace(
        "[engines.w]",
        &format!("allowed_origins = [\"{origin}\"]\n[[service.http.auth.callers]]\nid = \"alpha\"\ntoken_file = \"{token_path}\"\n[engines.w]"),
    ))
    .await;
    let response = s
        .request("server/discover", json!({}))
        .bearer_auth(&token)
        .header("origin", &origin)
        .header("user-agent", "u".repeat(1000))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    s.stop().await;
}

/// REQ-SEC-018 (LIM-H7), HTTP/2: the server advertises
/// `SETTINGS_MAX_HEADER_LIST_SIZE = 16384`, and a header list above it is
/// not served.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_sec_018_the_http2_header_list_is_bounded() {
    use tokio::io::AsyncWriteExt;
    let s = Server::start(LOOPBACK).await;
    let mut stream = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    stream
        .write_all(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n")
        .await
        .unwrap();
    stream.write_all(&h2_frame(0x4, 0, 0, &[])).await.unwrap();
    let mut advertised = None;
    loop {
        let (kind, flags, _, payload) = h2_read_frame(&mut stream, Duration::from_secs(3))
            .await
            .expect("the server's SETTINGS");
        if kind == 0x4 && flags & 0x1 == 0 {
            for entry in payload.chunks(6) {
                if u16::from_be_bytes([entry[0], entry[1]]) == 6 {
                    advertised = Some(u32::from_be_bytes([entry[2], entry[3], entry[4], entry[5]]));
                }
            }
            stream.write_all(&h2_frame(0x4, 0x1, 0, &[])).await.unwrap();
            break;
        }
    }
    assert_eq!(advertised, Some(16384), "MAX_HEADER_LIST_SIZE");

    // A request whose header list is 40,000 bytes: HEADERS without
    // END_HEADERS, then CONTINUATION frames of 16,000 bytes.
    let mut block = h2_open_request_headers();
    block.extend_from_slice(&[0x00, 5]);
    block.extend_from_slice(b"x-pad");
    // A string literal with a multi-byte length (RFC 7541 §5.1, 7-bit prefix).
    let value = vec![b'a'; 40_000];
    block.extend_from_slice(&[0x7f, 0xff - 0x7f]);
    let mut rest = value.len() - 127;
    while rest >= 128 {
        block.push((rest % 128) as u8 | 0x80);
        rest /= 128;
    }
    block.push(rest as u8);
    block.extend_from_slice(&value);
    let mut chunks = block.chunks(16_000);
    stream
        .write_all(&h2_frame(0x1, 0, 1, chunks.next().unwrap()))
        .await
        .unwrap();
    let remaining: Vec<&[u8]> = chunks.collect();
    for (i, chunk) in remaining.iter().enumerate() {
        let flags = if i + 1 == remaining.len() { 0x4 } else { 0 };
        stream
            .write_all(&h2_frame(0x9, flags, 1, chunk))
            .await
            .unwrap();
    }
    // What hyper does is recorded by the assertion: a response with status
    // 431 (HEADERS on stream 1) or a reset/close; never a served request.
    let mut outcome = String::from("nothing");
    while let Some((kind, _, id, payload)) =
        h2_read_frame(&mut stream, Duration::from_secs(3)).await
    {
        match kind {
            0x1 if id == 1 => {
                outcome = format!("headers {payload:02x?}");
                break;
            }
            0x3 if id == 1 => {
                outcome = "rst_stream".to_owned();
                break;
            }
            0x7 => {
                outcome = "goaway".to_owned();
                break;
            }
            _ => {}
        }
    }
    // h2 ends the connection (GOAWAY) when a header list outgrows the
    // advertised bound; a reset of the stream would do as well.
    assert!(
        outcome == "goaway" || outcome == "rst_stream",
        "an oversized header list must not be served: {outcome}"
    );
    s.stop().await;
}

// --- Mutants the audit run left alive (RFC 020, TST-7) ----------------------

/// REQ-SEC-018 and REQ-CFG-005: a caller's token of exactly the minimum
/// length is accepted and one byte less is refused with the key and the
/// number (a mutant of the comparison was missed).
#[test]
fn req_sec_018_a_token_of_the_minimum_length_is_accepted_and_one_less_is_not() {
    let settings = |token: &str| {
        let path = token_file("min-length", token);
        let config = Config::parse(&format!(
            "config_version = 1\n[service.http]\nbind = \"127.0.0.1:0\"\n[[service.http.auth.callers]]\nid = \"alpha\"\ntoken_file = \"{path}\"\n"
        ))
        .unwrap();
        HttpSettings::from_config(&config)
    };
    assert_eq!(TOKEN_MIN_BYTES, 32);
    assert!(settings(&"t".repeat(TOKEN_MIN_BYTES)).is_ok());
    let error = settings(&"t".repeat(TOKEN_MIN_BYTES - 1)).unwrap_err();
    let text = format!("{error:?}");
    assert!(text.contains("at least 32 bytes"), "{text}");
}

/// REQ-SEC-018: the `Host` names a listener accepts when `allowed_hosts` is
/// not set are the loopback names, and the bind address when it is a
/// specific one that is not already among them; an unspecified bind
/// (`0.0.0.0`) adds nothing, since no client sends it (mutants of the
/// condition were missed).
#[test]
fn req_sec_018_default_allowed_hosts_follow_the_bind_address() {
    let hosts = |bind: &str| {
        let config = Config::parse(&format!(
            "config_version = 1\n[service.http]\nbind = \"{bind}\"\n"
        ))
        .unwrap();
        HttpSettings::from_config(&config).unwrap().allowed_hosts
    };
    let loopback = ["localhost", "127.0.0.1", "::1"];
    assert_eq!(hosts("127.0.0.1:8080"), loopback, "no duplicate");
    assert_eq!(hosts("[::1]:8080"), loopback, "no duplicate");
    assert_eq!(
        hosts("0.0.0.0:8080"),
        loopback,
        "nothing for an unspecified bind"
    );
    assert_eq!(
        hosts("[::]:8080"),
        loopback,
        "nothing for an unspecified bind"
    );
    let mut specific = loopback.to_vec();
    specific.push("192.0.2.5");
    assert_eq!(hosts("192.0.2.5:8080"), specific);
}

/// A POST to `/mcp` with the given headers and a body written as it is, on a
/// raw connection: the status line and what was sent back.
async fn raw_post(s: &Server, head: &str, body: &[u8]) -> String {
    use tokio::io::AsyncWriteExt;
    let mut stream = tokio::net::TcpStream::connect(address(s)).await.unwrap();
    let request = format!(
        "POST {PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2026-07-28\r\nMcp-Method: server/discover\r\nConnection: close\r\n{head}\r\n"
    );
    let _ = stream.write_all(request.as_bytes()).await;
    let _ = stream.write_all(body).await;
    match watch(&mut stream, Duration::from_secs(5)).await {
        Watched::Closed(_, bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Watched::Open => "still open".to_owned(),
    }
}

/// REQ-SEC-022 and REQ-OBS-007 (LIM-H2): a body whose declared length is
/// over the bound is refused by the gate itself, 413, counted and logged
/// (`body_too_large`), before the SDK sees it; a body of exactly the bound is
/// not refused (mutants of the comparison were missed: the SDK's own 413
/// hid them, but it logs nothing).
#[tokio::test]
async fn req_sec_022_the_gate_refuses_a_declared_body_over_the_bound_and_only_that() {
    let (log, _guard) = routed_log();
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nmax_request_bytes = 4096\n",
    ))
    .await;
    let counters = s.service.counters();
    // Exactly the bound: a valid JSON-RPC request padded with white space.
    let base = json!({
        "jsonrpc": "2.0", "id": 1, "method": "server/discover",
        "params": { META: meta() }
    })
    .to_string();
    let padded = format!("{base}{}", " ".repeat(4096 - base.len()));
    assert_eq!(padded.len(), 4096);
    let answer = raw_post(
        &s,
        &format!("Content-Length: {}\r\n", padded.len()),
        padded.as_bytes(),
    )
    .await;
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert_eq!(counters.refusals_of("http", "body_too_large"), 0);
    assert!(events(&log, "http_refused").is_empty());
    // One byte over, declared. The headers only: the gate answers before it
    // reads the body, and a body left unread at the close would make some
    // systems reset the connection and lose the answer.
    let answer = raw_post(
        &s,
        &format!("Content-Length: {}\r\n", padded.len() + 1),
        b"",
    )
    .await;
    assert!(answer.starts_with("HTTP/1.1 413"), "{answer}");
    assert_eq!(counters.refusals_of("http", "body_too_large"), 1);
    let refused = events(&log, "http_refused");
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(refused[0]["reason"], "body_too_large");
    s.stop().await;
}

/// REQ-SEC-022 and REQ-OBS-007: a body with no declared length that grows
/// past the bound is refused by the SDK behind the gate, 413, and the gate
/// counts it under the same reason (the arm of `note_sdk_refusal` was
/// missed).
#[tokio::test]
async fn req_sec_022_a_streamed_body_over_the_bound_is_refused_and_counted() {
    let s = Server::start(&LOOPBACK.replace(
        "bind = \"127.0.0.1:0\"\n",
        "bind = \"127.0.0.1:0\"\nmax_request_bytes = 4096\n",
    ))
    .await;
    let counters = s.service.counters();
    let chunk = "x".repeat(6000);
    let body = format!("{:x}\r\n{chunk}\r\n0\r\n\r\n", chunk.len());
    let answer = raw_post(&s, "Transfer-Encoding: chunked\r\n", body.as_bytes()).await;
    // The SDK answers 413 and closes while the rest of the body is still
    // arriving; on some systems that close resets the connection and the
    // answer is lost before the client reads it (as in the 431 test). The
    // count is made when the answer is produced, so it is the assertion.
    assert!(
        answer.starts_with("HTTP/1.1 413") || answer.is_empty(),
        "{answer:?}"
    );
    assert_eq!(counters.refusals_of("http", "body_too_large"), 1);
    s.stop().await;
}

/// REQ-OBS-005 (RFC 019 D6): the address family in a gate event is the
/// connection's: a refusal on an IPv6 connection says `ipv6` (a mutant that
/// left the listener's default in place was missed). Skipped where the
/// machine has no IPv6 loopback.
#[tokio::test]
async fn req_obs_005_a_gate_event_names_the_family_of_the_connection() {
    if std::net::TcpListener::bind("[::1]:0").is_err() {
        return;
    }
    let (log, _guard) = routed_log();
    let s = Server::start(&LOOPBACK.replace("127.0.0.1:0", "[::1]:0")).await;
    let response = s
        .request("server/discover", json!({}))
        .header("host", "evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    let refused = events(&log, "http_refused");
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(refused[0]["family"], "ipv6");
    s.stop().await;
}

/// REQ-OPS-002 and REQ-CFG-005 (RFC 020, D-9): the time the listener waits
/// for its connections to finish at shutdown is `service.shutdown_ms`
/// (LIM-O1): it is in the settings, and a connection that is still open
/// when the listener is told to stop does not keep it longer than that.
#[tokio::test]
async fn req_ops_002_the_listener_stops_within_the_configured_shutdown_time() {
    let config = Config::parse(
        "config_version = 1\n[service]\nshutdown_ms = 600\n[service.http]\nbind = \"127.0.0.1:0\"\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n",
    )
    .unwrap();
    assert_eq!(
        HttpSettings::from_config(&config).unwrap().shutdown,
        Duration::from_millis(600)
    );
    let s = Server::start_tuned(
        "config_version = 1\n[service.http]\nbind = \"127.0.0.1:0\"\n[engines.w]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n",
        None,
        |settings| settings.shutdown = Duration::from_millis(600),
    )
    .await;
    // A connection in the middle of a request: the first line and no more.
    use tokio::io::AsyncWriteExt;
    let mut open = tokio::net::TcpStream::connect(address(&s)).await.unwrap();
    open.write_all(b"POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\n")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = std::time::Instant::now();
    s.shutdown.cancel();
    let ended = tokio::time::timeout(Duration::from_secs(5), s.task)
        .await
        .expect("the listener stops");
    assert!(ended.unwrap().is_ok());
    let took = started.elapsed();
    assert!(
        took >= Duration::from_millis(500) && took < Duration::from_millis(2500),
        "stopped after {took:?}, the shutdown time was 600 ms"
    );
    drop(open);
}
