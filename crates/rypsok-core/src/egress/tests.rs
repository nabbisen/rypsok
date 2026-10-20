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
