//! Tests of the drivers module. Every test names the requirement it verifies.

use std::time::Duration;

use super::*;

/// REQ-ENG-002 with RFC 004: every driver type declares its keys; an
/// unknown type declares none.
#[test]
fn req_eng_002_drivers_declare_their_keys() {
    assert_eq!(declared_keys("wikipedia"), Some(&[][..]));
    assert_eq!(
        declared_keys("brave"),
        Some(&["api_key_env", "api_key_file", "endpoint"][..])
    );
    assert_eq!(declared_keys("altavista"), None);
    assert!(needs_credential("brave"));
    assert!(!needs_credential("wikipedia"));
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
