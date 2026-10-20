//! Tests of the `engine` module. Every test names the requirement it verifies.

use super::*;

fn properties() -> EngineProperties {
    EngineProperties {
        max_query_chars: 10,
        max_query_words: Some(3),
        languages: LanguageSupport::Only(vec![Language::new("en").unwrap()]),
        time_ranges: TimeRangeSupport::None,
        requests_per_minute: 60,
        max_in_flight: 1,
        cacheable: true,
        attribution_required: false,
        user_agent: UserAgentFormat::Standard,
        max_response_bytes: 1024,
    }
}

/// REQ-ENG-007 with REQ-SEARCH-004 and REQ-SEARCH-005: an engine that
/// cannot honor the query length, the language or the period is skipped
/// with the matching reason, checked in that order.
#[test]
fn req_eng_007_skip_reasons_in_order() {
    let p = properties();
    let en = Language::new("en").unwrap();
    let ja = Language::new("ja").unwrap();
    assert_eq!(p.skip_reason("short", &en, &en, None), None);
    assert_eq!(
        p.skip_reason("far too long query", &en, &en, None),
        Some(Reason::QueryTooLong)
    );
    assert_eq!(
        p.skip_reason("a b c d", &en, &en, None),
        Some(Reason::QueryTooLong)
    );
    assert_eq!(
        p.skip_reason("short", &ja, &en, None),
        Some(Reason::Language)
    );
    assert_eq!(
        p.skip_reason("short", &en, &en, Some(TimeRange::Day)),
        Some(Reason::TimeRange)
    );
    let mut all = properties();
    all.languages = LanguageSupport::All;
    all.time_ranges = TimeRangeSupport::All;
    assert_eq!(
        all.skip_reason("short", &ja, &en, Some(TimeRange::Day)),
        None
    );
    let mut none = properties();
    none.languages = LanguageSupport::None;
    assert_eq!(none.skip_reason("short", &en, &en, None), None);
    assert_eq!(
        none.skip_reason("short", &ja, &en, None),
        Some(Reason::Language)
    );
}

/// REQ-ENG-006 and REQ-SEC-016: every error class maps to a notice reason
/// from the fixed vocabulary and carries no text.
#[test]
fn req_eng_006_error_classes_map_to_reasons() {
    let classes = [
        (EngineErrorClass::Timeout, Reason::Timeout),
        (EngineErrorClass::RateLimited, Reason::RateLimited),
        (EngineErrorClass::Blocked, Reason::Blocked),
        (EngineErrorClass::ParseFailure, Reason::ParseFailure),
        (EngineErrorClass::AuthFailure, Reason::AuthFailure),
        (EngineErrorClass::RequestFailure, Reason::RequestFailure),
    ];
    for (class, reason) in classes {
        assert_eq!(class.reason(), reason);
        assert_eq!(EngineError::new(class).retry_after, None);
    }
}
