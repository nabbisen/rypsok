//! What a fetch sends (REQ-FETCH-007, REQ-SEC-007, REQ-SEC-009, REQ-RES-008;
//! RFC 020, TST-6): a loopback server records the request line and every
//! header of every request.
//!
//! A fetch sends `GET`, and only the headers the agent client sets
//! (`User-Agent`, `Accept`, `Accept-Encoding`) and the `Host` the HTTP
//! library writes; never `Cookie`, `Authorization` or a proxy header; a
//! `Set-Cookie` in one answer is not sent back to the same host; and a
//! proxy named by the environment is not used. The environment is set for a
//! child process of the test (`unsafe` is forbidden here, so a process cannot
//! set its own).

use std::sync::Arc;
use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rypsok_core::CancellationToken;
use rypsok_core::config::SecretDetection;
use rypsok_core::egress::policy::{EgressPolicy, parse_rule};
use rypsok_core::egress::{AgentClientSettings, ReturnedUrls};
use rypsok_core::fetch::{self, FetchService};
use rypsok_core::limits::Limits;
use rypsok_core::model::{CallerId, FetchRequest};
use rypsok_core::telemetry::Counters;

const PAGE: &str = "<html><head><title>Sent</title></head><body><main><p>The page is long enough to be the main part of the document for the extractor, and it says nothing else.</p></main></body></html>";

/// The headers the client may send: the three it sets and the `Host` the
/// library writes from the URL.
const ALLOWED: [&str; 4] = ["accept", "accept-encoding", "host", "user-agent"];

/// Starts the server and fetches `/a` (which sets a cookie), then `/b`, then
/// `/a` again, each with robots.txt on; returns the server for inspection.
async fn fetch_three_times() -> MockServer {
    let server = MockServer::start().await;
    let port = server.address().port();
    Mock::given(method("GET"))
        .and(path("/a"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", "session=opaque; Path=/")
                .set_body_raw(PAGE.as_bytes().to_vec(), "text/html; charset=utf-8"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/b"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(PAGE.as_bytes().to_vec(), "text/html; charset=utf-8"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let policy = Arc::new(EgressPolicy::new(
        vec![parse_rule("127.0.0.1/32").unwrap()],
        Vec::new(),
        true,
        vec![80, 443, port],
    ));
    let limits = Limits::default();
    let settings = AgentClientSettings {
        user_agent: "rypsok/test (+https://example.org)".into(),
        connect_timeout: Duration::from_secs(2),
        timeout: Duration::from_secs(5),
        allow_http: true,
    };
    let service = FetchService::new(&settings, Arc::clone(&policy), &limits).unwrap();
    let counters = Counters::new();
    let returned = ReturnedUrls::new(10);
    let caller = CallerId::local();
    let pipeline = fetch::Pipeline {
        limits: &limits,
        policy: &policy,
        service: &service,
        returned: &returned,
        strict_mode: false,
        robots_txt: true,
        follow_cross_host_redirects: false,
        secret_detection: SecretDetection::Refuse,
        counters: &counters,
        caller: &caller,
    };
    for page in ["a", "b", "a"] {
        fetch::run(
            &pipeline,
            FetchRequest::new(format!("http://127.0.0.1:{port}/{page}")),
            CancellationToken::new(),
        )
        .await
        .unwrap_or_else(|e| panic!("/{page}: {e:?}"));
    }
    server
}

/// Every request the server saw is a `GET` with the allowed headers only,
/// and none carries a cookie or credentials.
async fn assert_only_what_rypsok_sets(server: &MockServer) {
    let requests = server.received_requests().await.expect("recording is on");
    // robots.txt once, then /a, /b and /a again.
    let paths: Vec<&str> = requests.iter().map(|r| r.url.path()).collect();
    assert_eq!(
        paths,
        ["/robots.txt", "/a", "/b", "/a"],
        "the server saw the fetches directly"
    );
    for request in &requests {
        assert_eq!(request.method.as_str(), "GET", "{}", request.url);
        let mut names: Vec<String> = request
            .headers
            .keys()
            .map(|name| name.as_str().to_ascii_lowercase())
            .collect();
        names.sort();
        assert!(
            names.iter().all(|n| ALLOWED.contains(&n.as_str())),
            "{}: headers sent {names:?}, allowed {ALLOWED:?}",
            request.url
        );
        for forbidden in ["cookie", "authorization", "proxy-authorization"] {
            assert!(
                !names.iter().any(|n| n == forbidden),
                "{}: {forbidden} sent",
                request.url
            );
        }
        let agent = request.headers.get("user-agent").unwrap().to_str().unwrap();
        assert!(agent.starts_with("rypsok/"), "{agent}");
        assert!(request.headers.get("accept").is_some());
    }
}

#[tokio::test]
async fn req_fetch_007_a_fetch_sends_get_and_only_the_headers_rypsok_sets() {
    let server = fetch_three_times().await;
    assert_only_what_rypsok_sets(&server).await;
}

/// A proxy named by the environment is not used: the fetch reaches the
/// server directly. The parent starts the child with every proxy variable
/// naming a closed loopback port.
#[test]
fn req_sec_009_a_proxy_in_the_environment_is_not_used() {
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let proxy = format!("http://127.0.0.1:{closed}");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", "proxy_child", "--ignored", "--nocapture"]);
    for name in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        command.env(name, &proxy);
    }
    command.env_remove("NO_PROXY").env_remove("no_proxy");
    let output = command.output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success() && text.contains("1 passed"),
        "the child failed:\n{text}"
    );
}

/// The child of the proxy test: the same three fetches, the same
/// assertions, with the proxy variables set by its parent.
#[tokio::test]
#[ignore = "run by the parent test with the proxy variables set"]
async fn proxy_child() {
    for name in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"] {
        assert!(std::env::var(name).is_ok(), "{name} is set by the parent");
    }
    let server = fetch_three_times().await;
    assert_only_what_rypsok_sets(&server).await;
}
