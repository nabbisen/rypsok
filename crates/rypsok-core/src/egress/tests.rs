//! Tests of the `egress` module. Every test names the requirement it verifies.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn settings(https_only: bool) -> EngineClientSettings {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-subscription-token",
        HeaderValue::from_static("test-credential"),
    );
    EngineClientSettings {
        user_agent: standard_user_agent("ops@example.org"),
        connect_timeout: Duration::from_millis(1000),
        timeout: Duration::from_millis(2000),
        proxy: None,
        https_only,
        headers,
    }
}

/// REQ-SEC-017: under the engine profile a redirect is not followed and
/// the credential header is present on the engine's origin.
#[tokio::test]
async fn req_sec_017_engine_profile_sends_credential_and_never_follows_redirects() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("x-subscription-token", "test-credential"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "http://127.0.0.1:9/elsewhere"),
        )
        .mount(&server)
        .await;
    let client = engine_client(&settings(false)).unwrap();
    let response = client
        .get(format!("{}/api", server.uri()))
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        302,
        "the redirect is returned, not followed"
    );
}

/// REQ-SEC-017 and REQ-FETCH-010: with `https_only` a plain-HTTP endpoint
/// is refused by the client itself.
#[tokio::test]
async fn req_sec_017_https_only_refuses_plain_http() {
    let server = MockServer::start().await;
    let client = engine_client(&settings(true)).unwrap();
    let error = client
        .get(format!("{}/api", server.uri()))
        .send()
        .await
        .unwrap_err();
    assert!(error.is_builder() || error.is_request(), "{error}");
}

/// REQ-CMP-004: the User-Agent is one stable string with the project URL
/// and the operator contact, and it is sent.
#[tokio::test]
async fn req_cmp_004_user_agent_is_identifying_and_sent() {
    let ua = standard_user_agent("ops@example.org");
    assert!(ua.starts_with("rypsok/"));
    assert!(ua.contains("+https://github.com/nabbisen/rypsok; ops@example.org"));
    assert_eq!(
        standard_user_agent("  "),
        format!(
            "rypsok/{} (+https://github.com/nabbisen/rypsok)",
            env!("CARGO_PKG_VERSION")
        )
    );
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("user-agent", ua.as_str()))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let client = engine_client(&settings(false)).unwrap();
    assert_eq!(client.get(server.uri()).send().await.unwrap().status(), 200);
}

/// REQ-SEC-010 with RFC 006: the body read stops at the driver's bound.
#[tokio::test]
async fn req_sec_010_body_read_stops_at_the_bound() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 4096]))
        .mount(&server)
        .await;
    let client = engine_client(&settings(false)).unwrap();
    let mut response = client.get(server.uri()).send().await.unwrap();
    assert_eq!(
        read_body(&mut response, 1024).await.unwrap_err(),
        BodyError::TooLarge
    );
    let mut response = client.get(server.uri()).send().await.unwrap();
    assert_eq!(read_body(&mut response, 8192).await.unwrap().len(), 4096);
}

/// REQ-SEC-013: byte limits apply to decompressed bytes as they are
/// produced: a small gzip body that inflates beyond the bound is refused.
#[tokio::test]
async fn req_sec_013_limit_applies_after_decompression() {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(&vec![b'x'; 200_000]).unwrap();
    let compressed = encoder.finish().unwrap();
    assert!(
        compressed.len() < 4096,
        "the compressed body is small: {}",
        compressed.len()
    );
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .set_body_bytes(compressed),
        )
        .mount(&server)
        .await;
    let client = engine_client(&settings(false)).unwrap();
    let mut response = client.get(server.uri()).send().await.unwrap();
    assert_eq!(
        read_body(&mut response, 100_000).await.unwrap_err(),
        BodyError::TooLarge
    );
    let mut response = client.get(server.uri()).send().await.unwrap();
    assert_eq!(
        read_body(&mut response, 300_000).await.unwrap().len(),
        200_000
    );
}

/// REQ-SEC-005 and REQ-TEST-015: a driver's requests pass the engine
/// profile: a redirect to another origin is not followed and the
/// credential stays on the engine's origin.
#[tokio::test]
async fn req_sec_005_driver_requests_pass_the_engine_profile() {
    let elsewhere = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&elsewhere)
        .await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(302).insert_header(
            "location",
            format!("{}/w/api.php", elsewhere.uri()).as_str(),
        ))
        .mount(&server)
        .await;
    let client = engine_client(&settings(false)).unwrap();
    let response = client
        .get(format!("{}/w/api.php", server.uri()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 302);
    assert!(
        elsewhere.received_requests().await.unwrap().is_empty(),
        "the other origin received nothing"
    );
}

/// REQ-SEC-011: every engine request has a connect timeout and a total
/// timeout from the limits.
#[test]
fn req_sec_011_engine_client_settings_carry_both_timeouts() {
    let s = settings(true);
    assert_eq!(s.connect_timeout, Duration::from_millis(1000));
    assert_eq!(s.timeout, Duration::from_millis(2000));
    assert!(engine_client(&s).is_ok());
}

// --- Mutants the per-change run of 020s found alive (RFC 020, TST-7) --------

/// A one-shot raw server: reads the request head, writes `head` and then
/// `body` (the connection is kept open afterwards when `hold` is set), and
/// returns its address.
async fn raw_server(head: &'static str, body: Vec<u8>, hold: bool) -> std::net::SocketAddr {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = [0u8; 2048];
        let _ = stream.read(&mut buffer).await;
        stream.write_all(head.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
        if hold {
            std::future::pending::<()>().await;
        }
        let _ = stream.shutdown().await;
    });
    address
}

/// REQ-SEC-010: a body of exactly the bound is read, one byte more is
/// refused, declared or streamed; a declared length over the bound is
/// refused at once, before any byte of the body (the early refusal is what a
/// server that announces a gigabyte and then stalls meets; mutants of both
/// comparisons were missed).
#[tokio::test]
async fn req_sec_010_the_bound_is_inclusive_and_a_declared_excess_is_refused_at_once() {
    let client = engine_client(&settings(false)).unwrap();

    // Declared and delivered: exactly the bound.
    let at = raw_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n",
        vec![b'x'; 1024],
        false,
    )
    .await;
    let mut response = client.get(format!("http://{at}/")).send().await.unwrap();
    assert_eq!(read_body(&mut response, 1024).await.unwrap().len(), 1024);

    // Streamed (no length): exactly the bound, then one more.
    let streamed = |extra: usize| async move {
        let chunk = |n: usize| format!("{n:x}\r\n{}\r\n", "x".repeat(n)).into_bytes();
        let mut body = chunk(1024);
        if extra > 0 {
            body.extend(chunk(extra));
        }
        body.extend_from_slice(b"0\r\n\r\n");
        raw_server(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            body,
            false,
        )
        .await
    };
    let at = streamed(0).await;
    let mut response = client.get(format!("http://{at}/")).send().await.unwrap();
    assert_eq!(read_body(&mut response, 1024).await.unwrap().len(), 1024);
    let over = streamed(1).await;
    let mut response = client.get(format!("http://{over}/")).send().await.unwrap();
    assert_eq!(
        read_body(&mut response, 1024).await.unwrap_err(),
        BodyError::TooLarge
    );

    // A declared length over the bound, and no body at all: refused at once.
    let stalled = raw_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 1000000000\r\nConnection: close\r\n\r\n",
        Vec::new(),
        true,
    )
    .await;
    let mut response = client
        .get(format!("http://{stalled}/"))
        .send()
        .await
        .unwrap();
    let refused = tokio::time::timeout(Duration::from_secs(1), read_body(&mut response, 1024))
        .await
        .expect("a declared excess is refused before the body is waited for");
    assert_eq!(refused.unwrap_err(), BodyError::TooLarge);
}

/// REQ-RES-008 and REQ-SEC-017: `[egress].proxy` is the only proxy path: an
/// empty value means none (it is not parsed as a proxy), and a value is used
/// for every request of the engine (mutants of the guard were missed).
#[tokio::test]
async fn req_res_008_the_configured_proxy_is_used_and_an_empty_one_is_none() {
    let mut empty = settings(false);
    empty.proxy = Some(String::new());
    assert!(engine_client(&empty).is_ok(), "an empty proxy is no proxy");

    let proxy = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("via the proxy"))
        .mount(&proxy)
        .await;
    let mut configured = settings(false);
    configured.proxy = Some(proxy.uri());
    let client = engine_client(&configured).unwrap();
    // A name nothing resolves: only the proxy can answer.
    let response = client
        .get("http://engine.invalid/search?q=x")
        .send()
        .await
        .unwrap();
    assert_eq!(response.text().await.unwrap(), "via the proxy");
    let seen = proxy.received_requests().await.unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].url.host_str(), Some("engine.invalid"));

    // No proxy configured: the request goes to the name itself and fails.
    let direct = engine_client(&settings(false)).unwrap();
    assert!(direct.get("http://engine.invalid/").send().await.is_err());
}

/// An error with a cause, like the one `reqwest` returns for a client it
/// cannot build.
#[derive(Debug)]
struct Outer(
    &'static str,
    Option<Box<dyn std::error::Error + Send + Sync>>,
);

impl std::fmt::Display for Outer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Outer {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// REQ-OPS-001 (RFC 020, F-3): a build failure is described by its cause,
/// from the source chain, and never by the generic `builder error` alone.
#[test]
fn req_ops_001_a_build_failure_is_described_by_its_cause() {
    let chain = Outer(
        "builder error",
        Some(Box::new(Outer(
            "first cause",
            Some(Box::new(Outer("second cause", None))),
        ))),
    );
    assert_eq!(describe_build_failure(&chain), "first cause: second cause");
    // An error without a cause is shown as its own text.
    assert_eq!(
        describe_build_failure(&Outer("only text", None)),
        "only text"
    );
}

/// REQ-OPS-001 (RFC 020, F-3): when no root certificate could be loaded the
/// message says so and where roots come from.
#[test]
fn req_ops_001_missing_roots_are_named_with_where_they_come_from() {
    let no_roots = Outer(
        "builder error",
        Some(Box::new(rustls::Error::General(
            "No CA certificates were loaded from the system".to_owned(),
        ))),
    );
    let text = describe_build_failure(&no_roots);
    assert_eq!(
        text,
        "no root certificates could be loaded (the system store, or SSL_CERT_FILE / SSL_CERT_DIR when set; see the deployment page, \"Which certificates the fetches trust\")"
    );
    // The clause "could not be built" comes once, from the error type.
    let shown = ClientError::Build(text).to_string();
    assert_eq!(shown.matches("could not be built").count(), 1, "{shown}");
}
