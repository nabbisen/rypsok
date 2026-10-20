//! Tests of the `model` module. Every test names the requirement it verifies.

use serde_json::{Value, json};

use super::*;

fn example_search_response() -> Value {
    json!({
      "outcome": "partial",
      "notices": [
        { "code": "engine_unavailable", "engine": "brave", "reason": "rate_limited" },
        { "code": "snippet_truncated", "count": 2 }
      ],
      "results": [
        {
          "title": "…",
          "url": "https://example.org/page",
          "snippet": "…",
          "date": "2026-09-01",
          "engines": ["wikipedia"]
        }
      ]
    })
}

/// REQ-CTX-002: the `web_search` example of external design §3.1 round-trips
/// through the types without a changed byte.
#[test]
fn req_ctx_002_search_response_example_round_trips() {
    let example = example_search_response();
    let parsed: SearchResponse =
        serde_json::from_value(example.clone()).expect("example deserializes");
    assert_eq!(parsed.outcome, Outcome::Partial);
    assert_eq!(parsed.notices[0].reason, Some(Reason::RateLimited));
    assert_eq!(serde_json::to_value(&parsed).expect("serializes"), example);
    let text = serde_json::to_string(&parsed).expect("serializes");
    let again: SearchResponse = serde_json::from_str(&text).expect("text deserializes");
    assert_eq!(again, parsed);
}

/// REQ-CTX-002: the `engine_status` example of external design §3.3 round-trips.
#[test]
fn req_ctx_002_status_response_example_round_trips() {
    let example = json!({
      "outcome": "ok",
      "notices": [],
      "engines": [
        { "id": "wikipedia", "state": "closed", "latency_ms": 180 },
        { "id": "brave", "state": "open", "latency_ms": null }
      ]
    });
    let parsed: StatusResponse =
        serde_json::from_value(example.clone()).expect("example deserializes");
    assert_eq!(parsed.engines[1].state, EngineState::Open);
    assert_eq!(serde_json::to_value(&parsed).expect("serializes"), example);
}

/// REQ-ERR-002: the error example of external design §4 round-trips.
#[test]
fn req_err_002_error_example_round_trips() {
    let example = json!({
      "error": {
        "kind": "invalid_request",
        "message": "max_results must be between 1 and 20",
        "detail": { "parameter": "max_results", "min": 1, "max": 20 }
      }
    });
    let built = ToolError::out_of_range("max_results", 1, 20).envelope();
    assert_eq!(serde_json::to_value(&built).expect("serializes"), example);
    let parsed: ErrorEnvelope = serde_json::from_value(example).expect("deserializes");
    assert_eq!(parsed, built);
}

/// REQ-SEC-016: every vocabulary serializes to the snake_case names of the
/// external design and nothing else deserializes.
#[test]
fn req_sec_016_vocabularies_are_closed() {
    assert_eq!(
        serde_json::to_value(NoticeCode::InstructionLikeContent).unwrap(),
        json!("instruction_like_content")
    );
    assert_eq!(
        serde_json::to_value(Reason::UrlInvalid).unwrap(),
        json!("url_invalid")
    );
    assert_eq!(
        serde_json::to_value(ErrorKind::UnsupportedContent).unwrap(),
        json!("unsupported_content")
    );
    assert_eq!(
        serde_json::to_value(DetailReason::RobotsTxt).unwrap(),
        json!("robots_txt")
    );
    assert_eq!(
        serde_json::to_value(EngineState::HalfOpen).unwrap(),
        json!("half_open")
    );
    assert_eq!(
        serde_json::to_value(Category::Science).unwrap(),
        json!("science")
    );
    assert_eq!(
        serde_json::to_value(TimeRange::Week).unwrap(),
        json!("week")
    );
    assert!(serde_json::from_value::<NoticeCode>(json!("Engine_Unavailable")).is_err());
    assert!(serde_json::from_value::<Outcome>(json!("success")).is_err());
}

/// REQ-MCP-004: requests refuse unknown fields.
#[test]
fn req_mcp_004_requests_refuse_unknown_fields() {
    let err =
        serde_json::from_value::<SearchRequest>(json!({ "query": "x", "page": 2 })).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
    let ok: SearchRequest = serde_json::from_value(json!({ "query": "x" })).unwrap();
    assert_eq!(ok, SearchRequest::new("x"));
}

/// REQ-ERR-001 and REQ-ERR-005: every row of the outcome decision table of
/// external design §7.
#[test]
fn req_err_001_decision_table_rows() {
    assert_eq!(decide(0, 0, 0, 0), Decision::NoEngine);
    assert_eq!(decide(2, 0, 2, 0), Decision::AllFailed);
    assert_eq!(decide(2, 2, 0, 3), Decision::Outcome(Outcome::Ok));
    assert_eq!(decide(2, 1, 1, 3), Decision::Outcome(Outcome::Partial));
    assert_eq!(decide(2, 2, 0, 0), Decision::Outcome(Outcome::Empty));
    assert_eq!(decide(2, 1, 1, 0), Decision::Outcome(Outcome::Empty));
}

/// REQ-ERR-005: `empty` is never returned when no engine answered.
#[test]
fn req_err_005_empty_requires_an_answer() {
    for n in 1..=4 {
        assert_eq!(decide(n, 0, n, 0), Decision::AllFailed);
    }
}

/// REQ-ERR-004: messages are built from templates; none exceeds LIM-I4 and
/// none repeats caller text.
#[test]
fn req_err_004_messages_are_bounded_templates() {
    let errors = [
        ToolError::out_of_range("max_results", 1, 20),
        ToolError::not_allowed("category", vec!["general".into(), "news".into()]),
        ToolError::missing("query"),
        ToolError::wrong_type("max_results", "an integer"),
        ToolError::unknown_parameter(&["query", "category"]),
        ToolError::query_length(400),
        ToolError::secret_in_query(SecretKind::GithubToken, 12),
        ToolError::secret_in_url(SecretKind::BasicAuthUrl, 8),
        ToolError::no_engine(Category::Code),
        ToolError::all_failed(Some(1500)),
        ToolError::deadline(),
        ToolError::rate_limited(1000),
        ToolError::busy(),
    ];
    for error in errors {
        assert!(error.message.chars().count() <= 200, "{}", error.message);
    }
    let secret = ToolError::secret_in_query(SecretKind::GithubToken, 12);
    assert_eq!(
        secret.message,
        "query contains what looks like a github token at character 12; remove it and retry"
    );
    assert_eq!(
        ToolError::no_engine(Category::Code).message,
        "no engine is enabled for category \"code\""
    );
}

/// REQ-CTX-001: `date` and `engines` are absent, not null, when not supplied.
#[test]
fn req_ctx_001_optional_result_fields_are_absent() {
    let result = SearchResult {
        title: "t".into(),
        url: "https://a/".into(),
        snippet: "s".into(),
        date: None,
        engines: None,
    };
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(
        value,
        json!({ "title": "t", "url": "https://a/", "snippet": "s" })
    );
}

/// REQ-RUST-003: identifiers and dates are validated at the boundary.
#[test]
fn req_rust_003_identifiers_and_dates_are_validated() {
    assert!(EngineId::new("wikipedia").is_ok());
    assert!(EngineId::new("brave_news").is_ok());
    assert!(EngineId::new("Brave").is_err());
    assert!(EngineId::new("1x").is_err());
    assert!(EngineId::new("").is_err());
    assert!(Language::new("en").is_ok());
    assert!(Language::new("pt-BR").is_ok());
    assert!(Language::new("EN").is_err());
    assert!(Language::new("en-us").is_err());
    assert!(Date::new("2026-09-01").is_ok());
    assert!(Date::new("2024-02-29").is_ok());
    assert!(Date::new("2023-02-29").is_err());
    assert!(Date::new("2026-13-01").is_err());
    assert!(Date::new("2026-9-1").is_err());
    assert!(serde_json::from_value::<EngineId>(json!("Bad Id")).is_err());
}
