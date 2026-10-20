//! Fixture tests of the Wikipedia driver (REQ-TEST-002). Every test names
//! the requirement it verifies.

use std::time::Duration;

use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use std::collections::BTreeMap;

use super::*;
use crate::config::SafeSearch;
use crate::model::Category;
use crate::model::Language;

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("wikipedia")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn context(server: &MockServer) -> DriverContext {
    DriverContext {
        user_agent: "rypsok/test (+https://github.com/nabbisen/rypsok)".into(),
        connect_timeout: Duration::from_millis(500),
        timeout: Duration::from_millis(1000),
        proxy: None,
        https_only: false,
        base_override: Some(url::Url::parse(&format!("{}/", server.uri())).unwrap()),
    }
}

fn table() -> EngineTable {
    EngineTable {
        kind: "wikipedia".into(),
        enabled: true,
        categories: vec![Category::General],
        preference: 10,
        keys: BTreeMap::new(),
    }
}

fn request() -> EngineRequest {
    EngineRequest {
        query: "rust".into(),
        max_results: 10,
        language: Language::new("en").unwrap(),
        region: "US".into(),
        safe_search: SafeSearch::Moderate,
        time_range: None,
    }
}

async fn mount(server: &MockServer, status: u16, body: Vec<u8>) {
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .and(query_param("action", "query"))
        .and(query_param("list", "search"))
        .and(query_param("srsearch", "rust"))
        .and(query_param("format", "json"))
        .respond_with(ResponseTemplate::new(status).set_body_bytes(body))
        .mount(server)
        .await;
}

/// REQ-TEST-002 and REQ-ENG-012: a successful response parses into raw
/// results with the page URL built from the title, raw snippets kept, and
/// the date taken from the timestamp.
#[tokio::test]
async fn req_test_002_wikipedia_search_ok_parses_raw_results() {
    let server = MockServer::start().await;
    mount(&server, 200, fixture("search-ok.json")).await;
    let driver = Wikipedia::new(&table(), &context(&server)).unwrap();
    let response = driver
        .search(&request(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(response.results.len(), 2);
    let first = &response.results[0];
    assert_eq!(first.title, "Rust (programming language)");
    assert_eq!(
        first.url,
        format!("{}/wiki/Rust_(programming_language)", server.uri())
    );
    assert!(
        first
            .snippet
            .contains("<span class=\"searchmatch\">Rust</span>"),
        "raw text is delivered as is"
    );
    assert_eq!(first.date.as_ref().unwrap().as_str(), "2026-09-01");
    assert_eq!(
        response.results[1].url,
        format!("{}/wiki/Rust_&amp;_Bone", server.uri())
    );
}

/// REQ-TEST-002: an empty response is an empty answer, not a failure.
#[tokio::test]
async fn req_test_002_wikipedia_search_empty_is_an_answer() {
    let server = MockServer::start().await;
    mount(&server, 200, fixture("search-empty.json")).await;
    let driver = Wikipedia::new(&table(), &context(&server)).unwrap();
    assert_eq!(
        driver
            .search(&request(), CancellationToken::new())
            .await
            .unwrap(),
        EngineResponse::default()
    );
}

/// REQ-ENG-006 and REQ-RES-002: each error class is classified; 429 reads
/// `Retry-After`; 403 for a bad User-Agent is `blocked`; a malformed body
/// is `parse_failure`; nothing of the provider's text leaves the driver.
#[tokio::test]
async fn req_eng_006_wikipedia_errors_are_classified() {
    let server = MockServer::start().await;
    let driver = Wikipedia::new(&table(), &context(&server)).unwrap();
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "7")
                .set_body_bytes(fixture("error-429.json")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let error = driver
        .search(&request(), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(
        error,
        EngineError {
            class: EngineErrorClass::RateLimited,
            retry_after: Some(Duration::from_secs(7))
        }
    );
    server.reset().await;
    mount(&server, 403, fixture("error-403.html")).await;
    assert_eq!(
        driver
            .search(&request(), CancellationToken::new())
            .await
            .unwrap_err()
            .class,
        EngineErrorClass::Blocked
    );
    server.reset().await;
    mount(&server, 200, fixture("error-malformed.json")).await;
    assert_eq!(
        driver
            .search(&request(), CancellationToken::new())
            .await
            .unwrap_err()
            .class,
        EngineErrorClass::ParseFailure
    );
    server.reset().await;
    mount(&server, 503, b"upstream down".to_vec()).await;
    assert_eq!(
        driver
            .search(&request(), CancellationToken::new())
            .await
            .unwrap_err()
            .class,
        EngineErrorClass::RequestFailure
    );
    server.reset().await;
    mount(&server, 200, fixture("error-429.json")).await;
    assert_eq!(
        driver
            .search(&request(), CancellationToken::new())
            .await
            .unwrap_err()
            .class,
        EngineErrorClass::RequestFailure,
        "an error object with status 200 is a request failure"
    );
}

/// REQ-ENG-006: a response slower than the timeout is `timeout`; a
/// cancelled call ends without a result.
#[tokio::test]
async fn req_eng_006_wikipedia_timeout_and_cancellation() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(1500))
                .set_body_bytes(fixture("search-ok.json")),
        )
        .mount(&server)
        .await;
    let driver = Wikipedia::new(&table(), &context(&server)).unwrap();
    assert_eq!(
        driver
            .search(&request(), CancellationToken::new())
            .await
            .unwrap_err()
            .class,
        EngineErrorClass::Timeout
    );
    let cancel = CancellationToken::new();
    let cancel_soon = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel_soon.cancel();
    });
    let started = std::time::Instant::now();
    assert!(driver.search(&request(), cancel).await.is_err());
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "cancellation aborts the request (REQ-MCP-007)"
    );
}

/// REQ-SEC-017 with RFC 006: `base_url` must be https unless loopback.
#[test]
fn req_sec_017_base_url_must_be_https_unless_loopback() {
    let context = DriverContext {
        user_agent: "rypsok/test".into(),
        connect_timeout: Duration::from_millis(500),
        timeout: Duration::from_millis(1000),
        proxy: None,
        https_only: true,
        base_override: None,
    };
    let with = |base: &str| {
        let mut keys = BTreeMap::new();
        keys.insert("base_url".to_owned(), toml::Value::String(base.to_owned()));
        EngineTable { keys, ..table() }
    };
    assert!(Wikipedia::new(&with("https://wiki.example.org/"), &context).is_ok());
    assert!(Wikipedia::new(&with("http://127.0.0.1:8080/"), &context).is_ok());
    assert!(Wikipedia::new(&with("http://localhost:8080/"), &context).is_ok());
    assert!(matches!(
        Wikipedia::new(&with("http://wiki.example.org/"), &context),
        Err(DriverBuildError::Key(_, _))
    ));
    assert!(matches!(
        Wikipedia::new(&with("not a url"), &context),
        Err(DriverBuildError::Key(_, _))
    ));
}

/// REQ-ENG-011 and REQ-ENG-005: the engine is asked for LIM-S3 results
/// through its own parameter, and the driver owns the parsing.
#[tokio::test]
async fn req_eng_011_results_per_engine_is_requested() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .and(query_param("srlimit", "10"))
        .and(query_param("srprop", "snippet|timestamp"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture("search-ok.json")))
        .mount(&server)
        .await;
    let driver = Wikipedia::new(&table(), &context(&server)).unwrap();
    assert_eq!(
        driver
            .search(&request(), CancellationToken::new())
            .await
            .unwrap()
            .results
            .len(),
        2
    );
}

/// REQ-ENG-005: a format change of the provider stays inside the driver as
/// a classified `parse_failure`; nothing of the body leaks.
#[tokio::test]
async fn req_eng_005_format_drift_is_confined_to_the_driver() {
    let server = MockServer::start().await;
    mount(
        &server,
        200,
        br#"{"query": {"pages": [{"title": "new shape"}]}}"#.to_vec(),
    )
    .await;
    let driver = Wikipedia::new(&table(), &context(&server)).unwrap();
    let response = driver
        .search(&request(), CancellationToken::new())
        .await
        .unwrap();
    assert!(
        response.results.is_empty(),
        "a missing list is an empty answer"
    );
    mount(&server, 200, b"<html>not json</html>".to_vec()).await;
    server.reset().await;
    mount(&server, 200, b"<html>not json</html>".to_vec()).await;
    let error = driver
        .search(&request(), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error, EngineError::new(EngineErrorClass::ParseFailure));
}
