//! Tests of the drivers module. Every test names the requirement it verifies.

use std::time::Duration;

use super::*;

/// REQ-ENG-002 with RFC 004: every driver type declares its keys; an
/// unknown type declares none.
#[test]
fn req_eng_002_drivers_declare_their_keys() {
    assert_eq!(declared_keys("wikipedia"), Some(&["base_url"][..]));
    assert_eq!(
        declared_keys("brave"),
        Some(&["api_key_env", "api_key_file", "endpoint"][..])
    );
    assert_eq!(declared_keys("duckduckgo"), Some(&[][..]));
    assert_eq!(declared_keys("altavista"), None);
    assert!(needs_credential("brave"));
    assert!(!needs_credential("wikipedia"));
    assert!(!needs_credential("duckduckgo"));
    assert!(is_scraping("duckduckgo"));
    assert!(!is_scraping("wikipedia") && !is_scraping("brave"));
}

/// REQ-RES-002: `Retry-After` is read in seconds and bounded.
#[test]
fn req_res_002_retry_after_header_is_parsed_and_bounded() {
    let mut headers = HeaderMap::new();
    headers.insert("retry-after", "30".parse().unwrap());
    assert_eq!(
        retry_after(&headers, "retry-after"),
        Some(Duration::from_secs(30))
    );
    headers.insert(
        "retry-after",
        "Wed, 21 Oct 2026 07:28:00 GMT".parse().unwrap(),
    );
    assert_eq!(retry_after(&headers, "retry-after"), None);
    headers.insert("retry-after", "999999".parse().unwrap());
    assert_eq!(retry_after(&headers, "retry-after"), None);
    assert_eq!(retry_after(&headers, "x-ratelimit-reset"), None);
}

/// REQ-SEC-016 and external design §16 I5 (RFC 018): a provider's error
/// body reaches a `debug` line only, cut to `PROVIDER_TEXT_MAX_CHARS`
/// characters and never a byte more.
#[test]
fn req_sec_016_provider_text_in_logs_is_bounded_and_debug_only() {
    let body = "x".repeat(PROVIDER_TEXT_MAX_CHARS * 3);
    let text = bounded_provider_text(body.as_bytes());
    assert_eq!(text.chars().count(), PROVIDER_TEXT_MAX_CHARS);
    assert_eq!(PROVIDER_TEXT_MAX_CHARS, 200);
    let multibyte = "é".repeat(PROVIDER_TEXT_MAX_CHARS + 5);
    assert_eq!(
        bounded_provider_text(multibyte.as_bytes()).chars().count(),
        PROVIDER_TEXT_MAX_CHARS
    );
    // The only call sites are the drivers' `debug_provider_text`, which
    // logs at `debug`: the source says so.
    let source = include_str!("../drivers.rs");
    assert!(source.contains("tracing::debug!(event = \"provider_error\""));
    assert!(!source.contains("tracing::info!(event = \"provider_error\""));
}
