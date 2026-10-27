//! Tests of the DuckDuckGo driver (RFC 015; REQ-TEST-018) against a
//! loopback mock. Every test names the requirement it verifies.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::model::Language;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/fixtures/scrape/duckduckgo/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn context(server: &MockServer, max_nodes: u32) -> DriverContext {
    DriverContext {
        user_agent: "rypsok/test (+https://github.com/nabbisen/rypsok)".into(),
        connect_timeout: Duration::from_millis(500),
        timeout: Duration::from_millis(1000),
        proxy: None,
        https_only: false,
        base_override: Some(url::Url::parse(&format!("{}/", server.uri())).unwrap()),
        parsing: Arc::new(Semaphore::new(2)),
        html_max_nodes: max_nodes,
    }
}

fn request(range: Option<TimeRange>) -> EngineRequest {
    EngineRequest {
        query: "rust programming".into(),
        max_results: 5,
        language: Language::new("en").unwrap(),
        region: "US".into(),
        safe_search: SafeSearch::Moderate,
        time_range: range,
    }
}

fn html(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.as_bytes().to_vec(), "text/html; charset=utf-8")
}

/// REQ-ENG-014 and REQ-CMP-004: the standard User-Agent and the `Accept`
/// header, nothing that resembles a browser; the parameters of the
/// record are sent; results parse.
#[tokio::test]
async fn req_eng_014_honest_headers_and_the_record_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/html/"))
        .and(query_param("q", "rust programming"))
        .and(query_param("kl", "us-en"))
        .and(query_param("kp", "-1"))
        .and(query_param("df", "w"))
        .respond_with(html(200, &fixture("results.html")))
        .mount(&server)
        .await;
    let driver = DuckDuckGo::new(&context(&server, 50_000)).unwrap();
    let response = driver
        .search(&request(Some(TimeRange::Week)), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(response.results.len(), 3);
    assert_eq!(response.results[0].url, "https://www.rust-lang.org/");
    let received = server.received_requests().await.unwrap();
    let headers = &received[0].headers;
    assert!(
        headers
            .get("user-agent")
            .is_some_and(|v| v.to_str().unwrap().starts_with("rypsok/")),
        "{headers:?}"
    );
    assert_eq!(
        headers.get("accept").unwrap().to_str().unwrap(),
        "text/html"
    );
    // A whitelist, not a blacklist (RFC 015 amendment): exactly what the
    // engine client sends and nothing a browser would add.
    let mut sent: Vec<String> = headers.keys().map(|k| k.as_str().to_owned()).collect();
    sent.sort();
    sent.dedup();
    let allowed = ["accept", "accept-encoding", "host", "user-agent"];
    assert!(
        sent.iter().all(|h| allowed.contains(&h.as_str())),
        "only the engine client's headers are sent: {sent:?}"
    );
    assert!(driver.properties().scraping);
    assert_eq!(driver.kind(), "duckduckgo");
}

/// REQ-ENG-015 and REQ-ENG-009: the no-results page is an empty answer;
/// the challenge page and 403 are `blocked`; 429 and 503 are
/// `rate_limited` with `Retry-After`; a drifted page is `parse_failure`.
#[tokio::test]
async fn req_eng_015_page_states_and_statuses_are_classified() {
    let cases: Vec<(u16, String, Option<&str>)> = vec![
        (200, fixture("no-results.html"), None),
        (200, fixture("challenge.html"), None),
        (403, "<html>forbidden</html>".into(), None),
        (429, "<html>slow down</html>".into(), Some("30")),
        (503, "<html>busy</html>".into(), Some("5")),
        (200, fixture("drifted.html"), None),
    ];
    let mut outcomes = Vec::new();
    for (status, body, retry) in cases {
        let server = MockServer::start().await;
        let mut template = html(status, &body);
        if let Some(seconds) = retry {
            template = template.insert_header("retry-after", seconds);
        }
        Mock::given(method("GET"))
            .and(path("/html/"))
            .respond_with(template)
            .mount(&server)
            .await;
        let driver = DuckDuckGo::new(&context(&server, 50_000)).unwrap();
        outcomes.push(
            driver
                .search(&request(None), CancellationToken::new())
                .await,
        );
    }
    assert_eq!(outcomes[0].as_ref().unwrap().results.len(), 0);
    assert_eq!(
        outcomes[1].as_ref().unwrap_err().class,
        EngineErrorClass::Blocked
    );
    assert_eq!(
        outcomes[2].as_ref().unwrap_err().class,
        EngineErrorClass::Blocked
    );
    let limited = outcomes[3].as_ref().unwrap_err();
    assert_eq!(limited.class, EngineErrorClass::RateLimited);
    assert_eq!(limited.retry_after, Some(Duration::from_secs(30)));
    let busy = outcomes[4].as_ref().unwrap_err();
    assert_eq!(busy.class, EngineErrorClass::RateLimited);
    assert_eq!(busy.retry_after, Some(Duration::from_secs(5)));
    assert_eq!(
        outcomes[5].as_ref().unwrap_err().class,
        EngineErrorClass::ParseFailure
    );
}

/// REQ-ENG-015: a redirect is never followed, on or off the host; a body
/// above the bound and a page above the node bound end without a panic.
#[tokio::test]
async fn req_eng_015_redirects_are_not_followed_and_bounds_hold() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/html/"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "https://example.org/elsewhere"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let driver = DuckDuckGo::new(&context(&server, 50_000)).unwrap();
    let error = driver
        .search(&request(None), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.class, EngineErrorClass::RequestFailure);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    let server = MockServer::start().await;
    let big = format!("<html><body>{}</body></html>", "x".repeat(300_000));
    Mock::given(method("GET"))
        .and(path("/html/"))
        .respond_with(html(200, &big))
        .mount(&server)
        .await;
    let driver = DuckDuckGo::new(&context(&server, 50_000)).unwrap();
    let error = driver
        .search(&request(None), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(
        error.class,
        EngineErrorClass::ParseFailure,
        "above LIM bytes"
    );

    let server = MockServer::start().await;
    let padded = fixture("results.html").replace(
        "<div id=\"links\"",
        &format!("{}<div id=\"links\"", "<span>x</span>".repeat(2_000)),
    );
    Mock::given(method("GET"))
        .and(path("/html/"))
        .respond_with(html(200, &padded))
        .mount(&server)
        .await;
    let driver = DuckDuckGo::new(&context(&server, 300)).unwrap();
    let error = driver
        .search(&request(None), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.class, EngineErrorClass::ParseFailure, "above LIM-F10");
}

/// REQ-MCP-007: cancellation during the request ends it.
#[tokio::test]
async fn req_mcp_007_cancellation_ends_the_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/html/"))
        .respond_with(html(200, &fixture("results.html")).set_delay(Duration::from_millis(500)))
        .mount(&server)
        .await;
    let driver = DuckDuckGo::new(&context(&server, 50_000)).unwrap();
    let cancel = CancellationToken::new();
    let canceller = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        canceller.cancel();
    });
    let started = std::time::Instant::now();
    let error = driver.search(&request(None), cancel).await.unwrap_err();
    assert_eq!(error.class, EngineErrorClass::RequestFailure);
    assert!(started.elapsed() < Duration::from_millis(400));
}
