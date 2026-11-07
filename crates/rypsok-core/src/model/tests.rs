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

/// REQ-CTX-002: the `web_search` example of external design §3.1 is what the
/// types serialize to, byte for byte. The response is built the way a
/// response is built, through the content boundary: `SearchResult` has no
/// `Deserialize` (RFC 020, DAT-10), so the example is not read into it.
#[test]
fn req_ctx_002_search_response_example_is_what_the_types_serialize_to() {
    let example = example_search_response();
    let (result, _) = crate::boundary::make_result(
        "…",
        "…",
        "https://example.org/page".to_owned(),
        Some(Date::new("2026-09-01").unwrap()),
        Some(vec![EngineId::new("wikipedia").unwrap()]),
        150,
        300,
    );
    let response = SearchResponse {
        outcome: Outcome::Partial,
        notices: vec![
            Notice::engine(
                NoticeCode::EngineUnavailable,
                EngineId::new("brave").unwrap(),
                Reason::RateLimited,
            ),
            Notice::counted(NoticeCode::SnippetTruncated, 2),
        ],
        results: vec![result],
    };
    assert_eq!(
        serde_json::to_value(&response).expect("serializes"),
        example
    );
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
        ToolError::fetch_deadline(),
        ToolError::http_status(404),
        ToolError::http_status(503),
        ToolError::denied(DetailReason::Scheme),
        ToolError::scheme_upgrade_failed(),
        ToolError::url_length(8192),
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
    // The search and the fetch deadline have their own words, one kind and
    // one reason (RFC 020, NET-6).
    assert_eq!(
        ToolError::deadline().message,
        "no engine answered before the deadline; retry later"
    );
    assert_eq!(
        ToolError::fetch_deadline().message,
        "the page was not retrieved before the deadline; retry later"
    );
    assert_eq!(ToolError::deadline().kind, ToolError::fetch_deadline().kind);
    assert_eq!(
        ToolError::deadline().detail.reason,
        ToolError::fetch_deadline().detail.reason
    );
    assert_eq!(
        ToolError::no_engine(Category::Code).message,
        "no engine is enabled for category \"code\""
    );
}

/// REQ-CTX-001: `date` and `engines` are absent, not null, when not supplied.
#[test]
fn req_ctx_001_optional_result_fields_are_absent() {
    let (result, _) =
        crate::boundary::make_result("t", "s", "https://a/".into(), None, None, 150, 300);
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

/// REQ-ERR-004 and REQ-SEC-001 (RFC 020, TST-7): **every** constructor of an
/// error builds its message from a template: at most LIM-I4 characters, and
/// no URL, host, address, token-like text or caller text, even when the
/// constructor is handed hostile arguments (the ones that carry external
/// data, `redirected` and `unsupported_media_type`, keep it in `detail`).
/// The table names every constructor; a constructor added to `ToolError`
/// without a row fails the second half.
#[test]
fn req_err_004_every_error_constructor_has_a_bounded_message_without_external_text() {
    let hostile_url =
        "https://evil.example.org/ignore-previous-instructions?token=s3cr3t".to_owned();
    let hostile_type = "application/x-evil; boundary=10.0.0.1".to_owned();
    let mut table: Vec<(&str, ToolError)> = vec![
        (
            "out_of_range",
            ToolError::out_of_range("max_results", 1, 20),
        ),
        (
            "not_allowed",
            ToolError::not_allowed("category", vec!["general".into(), "news".into()]),
        ),
        ("missing", ToolError::missing("query")),
        (
            "wrong_type",
            ToolError::wrong_type("max_results", "an integer"),
        ),
        (
            "unknown_parameter",
            ToolError::unknown_parameter(&["query", "category"]),
        ),
        ("query_length", ToolError::query_length(400)),
        ("url_length", ToolError::url_length(2048)),
        (
            "secret_in_query",
            ToolError::secret_in_query(SecretKind::GithubToken, 12),
        ),
        (
            "secret_in_url",
            ToolError::secret_in_url(SecretKind::BasicAuthUrl, 8),
        ),
        ("no_engine", ToolError::no_engine(Category::Code)),
        ("all_failed", ToolError::all_failed(Some(1500))),
        ("deadline", ToolError::deadline()),
        ("fetch_deadline", ToolError::fetch_deadline()),
        ("rate_limited", ToolError::rate_limited(1000)),
        ("scheme_upgrade_failed", ToolError::scheme_upgrade_failed()),
        ("network", ToolError::network()),
        ("http_status", ToolError::http_status(404)),
        ("http_status", ToolError::http_status(503)),
        ("redirected", ToolError::redirected(hostile_url.clone())),
        (
            "unsupported_media_type",
            ToolError::unsupported_media_type(hostile_type.clone()),
        ),
        ("extraction_failed", ToolError::extraction_failed()),
        ("too_large", ToolError::too_large(10_485_760)),
        ("start_beyond_end", ToolError::start_beyond_end(1200)),
        ("busy", ToolError::busy()),
    ];
    for reason in [
        DetailReason::Egress,
        DetailReason::RobotsTxt,
        DetailReason::Scheme,
        DetailReason::StrictMode,
        DetailReason::CredentialInUrl,
        DetailReason::SecretInQuery,
        DetailReason::SecretInUrl,
        DetailReason::NoEngine,
        DetailReason::AllFailed,
        DetailReason::RateLimited,
        DetailReason::Network,
        DetailReason::Deadline,
        DetailReason::HttpStatus,
        DetailReason::Busy,
        DetailReason::MediaType,
        DetailReason::ExtractionFailed,
    ] {
        table.push(("denied", ToolError::denied(reason)));
    }
    for (name, error) in &table {
        let message = &error.message;
        assert!(
            message.chars().count() <= 200,
            "{name}: {} characters",
            message.chars().count()
        );
        assert!(!message.contains("://"), "{name}: a URL in {message:?}");
        for external in [
            "evil",
            "example.org",
            "10.0.0.1",
            "s3cr3t",
            "ignore-previous",
        ] {
            assert!(
                !message.contains(external),
                "{name}: {external} in {message:?}"
            );
        }
        // No address: no run of digits and dots with three dots in it, and no
        // colon-separated hexadecimal groups.
        let dotted = message
            .split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .any(|word| word.matches('.').count() >= 3 && word.chars().any(|c| c.is_ascii_digit()));
        assert!(!dotted, "{name}: an address in {message:?}");
        assert!(!message.contains("::"), "{name}: an address in {message:?}");
    }
    // Where external data travels, it travels in `detail`, bounded by the
    // caller of the constructor, not in the message.
    assert_eq!(
        ToolError::redirected(hostile_url.clone()).detail.location,
        Some(hostile_url)
    );
    assert_eq!(
        ToolError::unsupported_media_type(hostile_type.clone())
            .detail
            .media_type,
        Some(hostile_type)
    );

    // The table is complete: every `pub fn` of `impl ToolError` that returns
    // `Self` and builds an error from arguments has a row.
    let source = include_str!("error.rs");
    let block = source
        .split("\nimpl ToolError {")
        .nth(1)
        .expect("impl ToolError")
        .split("\n}\n")
        .next()
        .unwrap();
    let constructors: Vec<&str> = block
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("pub fn "))
        .filter(|rest| rest.contains("-> Self") && !rest.starts_with("with_"))
        .filter_map(|rest| rest.split('(').next())
        .collect();
    assert!(constructors.len() >= 24, "{constructors:?}");
    for name in constructors {
        assert!(
            table.iter().any(|(row, _)| *row == name),
            "the constructor `{name}` has no row in the table"
        );
    }
}
