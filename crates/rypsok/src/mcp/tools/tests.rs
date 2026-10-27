//! Tests of the tool definitions. Every test names the requirement it verifies.

use std::collections::BTreeSet;

use super::*;
use rypsok_core::limits::Limits;
use rypsok_core::model::ErrorKind;

fn policy(languages: &[&str]) -> SearchPolicy {
    SearchPolicy {
        query_max_chars: 400,
        max_results_default: 5,
        max_results_ceiling: 20,
        languages: languages
            .iter()
            .map(|l| Language::new(*l).unwrap())
            .collect(),
        categories: [Category::General, Category::Science]
            .into_iter()
            .collect::<BTreeSet<_>>(),
    }
}

/// REQ-MCP-008: all definitions and the instructions fit LIM-I1; every
/// tool has at most LIM-I2 parameters; every schema is flat (LIM-I3).
#[test]
fn req_mcp_008_definitions_fit_the_budget_and_are_flat() {
    let tools = tool_list(&policy(&["en", "ja"]), &Limits::default(), true);
    assert!(
        definitions_size(&tools) <= 2000,
        "definitions take {} characters",
        definitions_size(&tools)
    );
    for tool in &tools {
        let schema = Value::Object(tool.input_schema.as_ref().clone());
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], false);
        let properties = schema["properties"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        assert!(
            properties.len() <= 5,
            "{} has {} parameters",
            tool.name,
            properties.len()
        );
        for (name, property) in properties {
            let kind = property["type"].as_str().unwrap_or_default();
            assert!(
                matches!(kind, "string" | "integer" | "boolean"),
                "{name} is a {kind}"
            );
        }
    }
}

/// REQ-MCP-009: annotations are fixed; `openWorldHint` is true for the
/// tools that reach the network and false for status; the order is
/// deterministic.
#[test]
fn req_mcp_009_annotations_and_order() {
    let tools = tool_list(&policy(&["en"]), &Limits::default(), true);
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    assert_eq!(names, vec!["web_search", "web_fetch", "engine_status"]);
    for tool in &tools {
        let a = tool.annotations.as_ref().unwrap();
        assert_eq!(a.read_only_hint, Some(true));
        assert_eq!(a.destructive_hint, Some(false));
        assert_eq!(a.idempotent_hint, Some(true));
        assert_eq!(a.open_world_hint, Some(tool.name != "engine_status"));
    }
}

/// REQ-STATUS-003: `engine_status` is absent from the list when disabled.
#[test]
fn req_status_003_status_tool_absent_when_disabled() {
    let tools = tool_list(&policy(&["en"]), &Limits::default(), false);
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "web_search");
    assert_eq!(tools[1].name, "web_fetch");
}

/// REQ-SEARCH-002 and REQ-SEARCH-004: the schema enumerates served
/// categories and configured languages; `language` is omitted for one.
#[test]
fn req_search_002_schema_enumerates_served_categories_and_languages() {
    let schema = web_search_input_schema(&policy(&["en", "ja"]));
    assert_eq!(
        schema["properties"]["category"]["enum"],
        json!(["general", "science"])
    );
    assert_eq!(
        schema["properties"]["language"]["enum"],
        json!(["en", "ja"])
    );
    assert_eq!(schema["properties"]["max_results"]["default"], json!(5));
    assert_eq!(schema["properties"]["query"]["maxLength"], json!(400));
    let single = web_search_input_schema(&policy(&["en"]));
    assert!(single["properties"].get("language").is_none());
}

/// REQ-MCP-004: arguments are checked field by field; failures name the
/// parameter and never repeat caller text.
#[test]
fn req_mcp_004_arguments_are_validated_per_parameter() {
    let args = |v: Value| -> Option<JsonObject> { v.as_object().cloned() };
    let err = parse_search_arguments(args(json!({})).as_ref(), 20).unwrap_err();
    assert_eq!(
        (err.kind, err.detail.parameter.as_deref()),
        (ErrorKind::InvalidRequest, Some("query"))
    );
    let err =
        parse_search_arguments(args(json!({ "query": "x", "page": 2 })).as_ref(), 20).unwrap_err();
    assert!(!err.message.contains("page"));
    assert_eq!(err.detail.allowed.as_ref().map(Vec::len), Some(5));
    let err = parse_search_arguments(args(json!({ "query": 5 })).as_ref(), 20).unwrap_err();
    assert_eq!(err.message, "query must be a string");
    let err = parse_search_arguments(
        args(json!({ "query": "x", "max_results": "many" })).as_ref(),
        20,
    )
    .unwrap_err();
    assert_eq!(err.detail.parameter.as_deref(), Some("max_results"));
    let err = parse_search_arguments(
        args(json!({ "query": "x", "category": "shopping" })).as_ref(),
        20,
    )
    .unwrap_err();
    assert_eq!(err.detail.parameter.as_deref(), Some("category"));
    assert!(!err.message.contains("shopping"));
    let ok = parse_search_arguments(
        args(json!({ "query": "x", "max_results": 3, "time_range": "week", "language": "ja" }))
            .as_ref(),
        20,
    )
    .unwrap();
    assert_eq!(ok.max_results, Some(3));
    assert_eq!(ok.time_range, Some(TimeRange::Week));
    assert_eq!(ok.language.unwrap().as_str(), "ja");
}

/// REQ-MCP-011: output schemas are generated from the types and checked
/// in as fixtures, so that a change is visible in review (RFC 003).
#[test]
fn req_mcp_011_output_schemas_match_the_fixtures() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("schemas");
    for (name, schema) in [
        ("search_response.json", search_output_schema()),
        ("fetch_response.json", fetch_output_schema()),
        ("status_response.json", status_output_schema()),
    ] {
        let path = root.join(name);
        let rendered = serde_json::to_string_pretty(&schema).unwrap() + "\n";
        if std::env::var_os("RYPSOK_UPDATE_FIXTURES").is_some() {
            std::fs::write(&path, &rendered).unwrap();
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {e} (set RYPSOK_UPDATE_FIXTURES=1 to write it)",
                path.display()
            )
        });
        assert_eq!(
            expected.replace("\r\n", "\n"),
            rendered,
            "{} changed; review it and set RYPSOK_UPDATE_FIXTURES=1 to accept",
            path.display()
        );
        assert!(
            schema["properties"].get("outcome").is_some(),
            "{name} declares outcome"
        );
    }
}

/// Contract audit of RFC 016: an out-of-range `max_results` names the
/// operator's ceiling, never a constant, whether the value fits a byte or
/// not.
#[test]
fn req_main_006_max_results_errors_name_the_configured_ceiling() {
    let args = |v: Value| -> Option<JsonObject> { v.as_object().cloned() };
    let huge = parse_search_arguments(
        args(json!({ "query": "x", "max_results": 300 })).as_ref(),
        10,
    )
    .unwrap_err();
    assert_eq!(huge.detail.parameter.as_deref(), Some("max_results"));
    assert_eq!(huge.detail.max, Some(10));
    assert!(huge.message.contains("10"), "{}", huge.message);
    let zero = parse_search_arguments(args(json!({ "query": "x", "max_results": 0 })).as_ref(), 7)
        .unwrap_err();
    assert_eq!(zero.detail.max, Some(7));
}
