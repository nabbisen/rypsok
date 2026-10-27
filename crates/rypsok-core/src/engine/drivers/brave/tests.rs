//! Fixture tests of the Brave driver (REQ-TEST-002). Every test names the
//! requirement it verifies.

use std::collections::BTreeMap;
use std::time::Duration;

use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::model::{Category, Language};

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("brave")
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
        parsing: std::sync::Arc::new(tokio::sync::Semaphore::new(2)),
        html_max_nodes: 50_000,
    }
}

fn table(endpoint: Option<&str>) -> EngineTable {
    let mut keys = BTreeMap::new();
    if let Some(endpoint) = endpoint {
        keys.insert(
            "endpoint".to_owned(),
            toml::Value::String(endpoint.to_owned()),
        );
    }
    EngineTable {
        kind: "brave".into(),
        enabled: true,
        categories: vec![Category::General],
        preference: 20,
        keys,
    }
}

fn secret() -> Secret<String> {
    Secret::new("test-token".to_owned())
}

fn request(range: Option<TimeRange>) -> EngineRequest {
    EngineRequest {
        query: "rust programming".into(),
        max_results: 10,
        language: Language::new("en").unwrap(),
        region: "US".into(),
        safe_search: SafeSearch::Moderate,
        time_range: range,
    }
}

/// REQ-TEST-002 and REQ-SEARCH-008: a successful web response parses into
/// raw results; the credential, language, region, safe search and
/// freshness are sent as the admission record describes.
#[tokio::test]
async fn req_test_002_brave_web_search_ok_sends_parameters_and_parses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/web/search"))
        .and(header("x-subscription-token", "test-token"))
        .and(query_param("q", "rust programming"))
        .and(query_param("count", "10"))
        .and(query_param("search_lang", "en"))
        .and(query_param("country", "US"))
        .and(query_param("safesearch", "moderate"))
        .and(query_param("freshness", "pw"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture("search-ok.json")))
        .mount(&server)
        .await;
    let driver = Brave::new(&table(None), Some(&secret()), &context(&server)).unwrap();
    let response = driver
        .search(&request(Some(TimeRange::Week)), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(response.results.len(), 2);
    assert_eq!(response.results[0].title, "Rust Programming Language");
    assert_eq!(
        response.results[0].url, "https://www.rust-lang.org/?utm_source=brave",
        "the URL is delivered as the provider gave it"
    );
    assert_eq!(
        response.results[0].date.as_ref().unwrap().as_str(),
        "2026-08-30"
    );
    assert_eq!(response.results[1].date, None);
    assert!(
        response.results[1].snippet.contains("&quot;"),
        "raw text is delivered as is"
    );
}

/// REQ-TEST-002: the news endpoint is selected by the `endpoint` key and
/// parses its own shape; an unknown endpoint value is refused at build.
#[tokio::test]
async fn req_test_002_brave_news_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/res/v1/news/search"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture("news-ok.json")))
        .mount(&server)
        .await;
    let driver = Brave::new(&table(Some("news")), Some(&secret()), &context(&server)).unwrap();
    let response = driver
        .search(&request(None), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(
        response.results[0].date.as_ref().unwrap().as_str(),
        "2026-10-19"
    );
    assert!(matches!(
        Brave::new(&table(Some("images")), Some(&secret()), &context(&server)),
        Err(DriverBuildError::Key(_, _))
    ));
    assert!(matches!(
        Brave::new(&table(None), None, &context(&server)),
        Err(DriverBuildError::Key(_, _))
    ));
}

/// REQ-TEST-002: an empty response is an empty answer.
#[tokio::test]
async fn req_test_002_brave_search_empty_is_an_answer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture("search-empty.json")))
        .mount(&server)
        .await;
    let driver = Brave::new(&table(None), Some(&secret()), &context(&server)).unwrap();
    assert_eq!(
        driver
            .search(&request(None), CancellationToken::new())
            .await
            .unwrap(),
        EngineResponse::default()
    );
}

/// REQ-ENG-006 and REQ-RES-002: 429 is `rate_limited` with
/// `X-RateLimit-Reset` as the retry hint; 401 is `auth_failure`; a
/// malformed body is `parse_failure`; 5xx is `request_failure`.
#[tokio::test]
async fn req_eng_006_brave_errors_are_classified() {
    let server = MockServer::start().await;
    let driver = Brave::new(&table(None), Some(&secret()), &context(&server)).unwrap();
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-reset", "1")
                .set_body_bytes(fixture("error-429.json")),
        )
        .mount(&server)
        .await;
    let error = driver
        .search(&request(None), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(
        error,
        EngineError {
            class: EngineErrorClass::RateLimited,
            retry_after: Some(Duration::from_secs(1))
        }
    );
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_bytes(fixture("error-401.json")))
        .mount(&server)
        .await;
    assert_eq!(
        driver
            .search(&request(None), CancellationToken::new())
            .await
            .unwrap_err()
            .class,
        EngineErrorClass::AuthFailure
    );
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(fixture("error-malformed.json")))
        .mount(&server)
        .await;
    assert_eq!(
        driver
            .search(&request(None), CancellationToken::new())
            .await
            .unwrap_err()
            .class,
        EngineErrorClass::ParseFailure
    );
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    assert_eq!(
        driver
            .search(&request(None), CancellationToken::new())
            .await
            .unwrap_err()
            .class,
        EngineErrorClass::RequestFailure
    );
}
