//! Tests of the `search` module. Every test names the requirement it verifies.

use super::*;
use crate::model::ErrorKind;

fn policy() -> SearchPolicy {
    SearchPolicy {
        query_max_chars: 400,
        max_results_default: 5,
        max_results_ceiling: 20,
        languages: vec![Language::new("en").unwrap(), Language::new("ja").unwrap()],
        categories: [Category::General, Category::Science].into_iter().collect(),
    }
}

/// REQ-SEARCH-001: the query is 1 to LIM-S1 characters after trimming.
#[test]
fn req_search_001_query_length_after_trimming() {
    let p = policy();
    assert!(validate(&SearchRequest::new("   "), &p).is_err());
    assert!(validate(&SearchRequest::new(""), &p).is_err());
    let ok = validate(&SearchRequest::new("  rust  "), &p).unwrap();
    assert_eq!(ok.query.expose(), "rust");
    let long = "あ".repeat(401);
    let err = validate(&SearchRequest::new(long), &p).unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidRequest);
    assert_eq!(err.detail.parameter.as_deref(), Some("query"));
    assert_eq!(err.detail.max, Some(400));
    assert!(validate(&SearchRequest::new("あ".repeat(400)), &p).is_ok());
}

/// REQ-SEARCH-002: the category defaults to `general` and is refused when
/// no enabled engine serves it, naming the valid values.
#[test]
fn req_search_002_category_default_and_served_only() {
    let p = policy();
    assert_eq!(
        validate(&SearchRequest::new("x"), &p).unwrap().category,
        Category::General
    );
    let req = SearchRequest {
        category: Some(Category::News),
        ..SearchRequest::new("x")
    };
    let err = validate(&req, &p).unwrap_err();
    assert_eq!(
        err.detail.allowed,
        Some(vec!["general".to_owned(), "science".to_owned()])
    );
    assert_eq!(err.message, "category must be one of: general, science");
}

/// REQ-SEARCH-003: `max_results` defaults to LIM-S2 and is refused outside
/// its range with the range stated.
#[test]
fn req_search_003_max_results_default_and_range() {
    let p = policy();
    assert_eq!(
        validate(&SearchRequest::new("x"), &p).unwrap().max_results,
        5
    );
    for bad in [0u8, 21, 255] {
        let req = SearchRequest {
            max_results: Some(bad),
            ..SearchRequest::new("x")
        };
        let err = validate(&req, &p).unwrap_err();
        assert_eq!(err.message, "max_results must be between 1 and 20");
        assert_eq!((err.detail.min, err.detail.max), (Some(1), Some(20)));
    }
    let req = SearchRequest {
        max_results: Some(20),
        ..SearchRequest::new("x")
    };
    assert_eq!(validate(&req, &p).unwrap().max_results, 20);
}

/// REQ-SEARCH-004: the language defaults to the operator's first and is
/// refused outside the configured list.
#[test]
fn req_search_004_language_default_and_configured_only() {
    let p = policy();
    assert_eq!(
        validate(&SearchRequest::new("x"), &p)
            .unwrap()
            .language
            .as_str(),
        "en"
    );
    let req = SearchRequest {
        language: Some(Language::new("ja").unwrap()),
        ..SearchRequest::new("x")
    };
    assert_eq!(validate(&req, &p).unwrap().language.as_str(), "ja");
    let req = SearchRequest {
        language: Some(Language::new("fr").unwrap()),
        ..SearchRequest::new("x")
    };
    let err = validate(&req, &p).unwrap_err();
    assert_eq!(
        err.detail.allowed,
        Some(vec!["en".to_owned(), "ja".to_owned()])
    );
}

/// REQ-SEARCH-005: `time_range` is optional and passed through.
#[test]
fn req_search_005_time_range_passes_through() {
    let p = policy();
    assert_eq!(
        validate(&SearchRequest::new("x"), &p).unwrap().time_range,
        None
    );
    let req = SearchRequest {
        time_range: Some(TimeRange::Month),
        ..SearchRequest::new("x")
    };
    assert_eq!(
        validate(&req, &p).unwrap().time_range,
        Some(TimeRange::Month)
    );
}
