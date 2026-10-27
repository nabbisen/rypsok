//! The examples the tutorials include are checked here (RFC 017,
//! REQ-DOC-002): configurations validate and pass `check`, host entries
//! have the shape a runtime reads, every call matches the tool contract,
//! and the first tutorial runs end to end against a mock provider.

#[path = "protocol/session.rs"]
mod session;

use std::path::{Path, PathBuf};
use std::process::Command;

use rypsok_core::config::Config;
use rypsok_core::model::{
    ErrorEnvelope, FetchRequest, FetchResponse, SearchRequest, SearchResponse, StatusResponse,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use session::{Session, config_file};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
}

fn files(dir: &str, extension: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(examples().join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == extension))
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no {extension} files under examples/{dir}");
    out
}

/// REQ-DOC-002: every example configuration parses, validates, and
/// passes `rypsok check` with exit code 0.
#[test]
fn req_doc_002_example_configurations_validate() {
    let configs = files("configs", "toml");
    assert!(configs.len() >= 8, "{configs:?}");
    for path in configs {
        let text = std::fs::read_to_string(&path).unwrap();
        let config = Config::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        config
            .validate()
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let status = Command::new(env!("CARGO_BIN_EXE_rypsok"))
            .args(["check", "--config"])
            .arg(&path)
            .env_remove("RYPSOK_CONFIG")
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}: check failed: {}",
            path.display(),
            String::from_utf8_lossy(&status.stderr)
        );
    }
}

/// REQ-DOC-002: every host entry is JSON with one `mcpServers` object
/// whose `rypsok` entry either starts the binary or names the `/mcp`
/// endpoint of a listener.
#[test]
fn req_doc_002_example_host_entries_have_the_shape() {
    for path in files("hosts", "json") {
        let value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let entry = &value["mcpServers"]["rypsok"];
        assert!(
            entry.is_object(),
            "{}: no mcpServers.rypsok",
            path.display()
        );
        let starts = entry["command"] == json!("rypsok")
            && entry["args"][0] == json!("serve")
            && entry.get("url").is_none();
        let connects = entry["url"]
            .as_str()
            .is_some_and(|u| u.starts_with("https://") && u.ends_with("/mcp"))
            && entry["headers"]["Authorization"]
                .as_str()
                .is_some_and(|a| a.starts_with("Bearer "));
        assert!(starts || connects, "{}: {entry}", path.display());
    }
}

/// The request types deny unknown fields, so parsing alone proves the
/// arguments are the contract's.
fn parses<T: DeserializeOwned>(value: &Value, what: &str) {
    let _: T = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("{what}: does not match the type: {e}\n{value}"));
}

fn round_trips<T: DeserializeOwned + Serialize>(value: &Value, what: &str) {
    let typed: T = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("{what}: does not match the type: {e}\n{value}"));
    let back = serde_json::to_value(&typed).unwrap();
    assert_eq!(&back, value, "{what}: a field is not part of the contract");
}

/// REQ-DOC-002: every example call is a `tools/call` or `server/discover`
/// request of the 2026-07-28 revision whose arguments match the request
/// types, and whose answer round-trips through the response types (or
/// the error envelope), so that no field outside the contract appears.
#[test]
fn req_doc_002_example_calls_match_the_contract() {
    let calls = files("calls", "json");
    assert!(calls.len() >= 11, "{calls:?}");
    for path in calls {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let request = &doc["request"];
        let response = &doc["response"];
        assert_eq!(request["jsonrpc"], json!("2.0"), "{name}");
        assert_eq!(request["id"], response["id"], "{name}: ids differ");
        let meta = &request["params"]["_meta"];
        assert_eq!(
            meta["io.modelcontextprotocol/protocolVersion"],
            json!("2026-07-28"),
            "{name}"
        );
        if request["method"] == json!("server/discover") {
            assert!(
                response["result"]["supportedVersions"]
                    .as_array()
                    .is_some_and(|v| v.contains(&json!("2026-07-28"))),
                "{name}"
            );
            continue;
        }
        assert_eq!(request["method"], json!("tools/call"), "{name}");
        let tool = request["params"]["name"].as_str().unwrap();
        let arguments = &request["params"]["arguments"];
        let result = &response["result"];
        let structured = &result["structuredContent"];
        // The text form is the same JSON (REQ-MCP-011).
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(&text, structured, "{name}: text and structure differ");
        if result["isError"] == json!(true) {
            round_trips::<ErrorEnvelope>(structured, &name);
            continue;
        }
        match tool {
            "web_search" => {
                parses::<SearchRequest>(arguments, &format!("{name} arguments"));
                round_trips::<SearchResponse>(structured, &name);
            }
            "web_fetch" => {
                parses::<FetchRequest>(arguments, &format!("{name} arguments"));
                round_trips::<FetchResponse>(structured, &name);
            }
            "engine_status" => {
                assert_eq!(arguments, &json!({}), "{name}");
                round_trips::<StatusResponse>(structured, &name);
            }
            other => panic!("{name}: unknown tool {other}"),
        }
    }
}

/// REQ-DOC-002: the first tutorial's two lines, piped into the binary
/// with its configuration (the engine pointed at a mock serving the
/// Wikipedia fixture), give an answer with the outcome and the fields the
/// tutorial shows.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn req_doc_002_first_tutorial_runs_end_to_end() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rypsok-core/fixtures/wikipedia/search-ok.json");
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(std::fs::read(fixture).unwrap(), "application/json"),
        )
        .mount(&server)
        .await;
    // The tutorial's file, with the engine pointed at the mock: the
    // engine table is the last table of the file, so the key appends.
    let shipped = std::fs::read_to_string(examples().join("configs/first-search.toml")).unwrap();
    assert!(
        shipped
            .trim_end()
            .ends_with("categories = [\"general\", \"science\"]")
    );
    let config = config_file(&format!("{shipped}base_url = \"{}/\"\n", server.uri()));
    let lines = std::fs::read_to_string(examples().join("calls/first-search.jsonl")).unwrap();
    let sample: Value = serde_json::from_str(
        &std::fs::read_to_string(examples().join("calls/web-search-ok.json")).unwrap(),
    )
    .unwrap();
    tokio::task::spawn_blocking(move || {
        let mut s = Session::start(Some(&config));
        let mut answers = Vec::new();
        for line in lines.lines().filter(|l| !l.trim().is_empty()) {
            let message: Value = serde_json::from_str(line).unwrap();
            answers.push(s.request_raw(message));
        }
        assert_eq!(answers.len(), 2);
        assert!(
            answers[0]["supportedVersions"]
                .as_array()
                .is_some_and(|v| v.contains(&json!("2026-07-28"))),
            "discover: {}",
            answers[0]
        );
        let answer = &answers[1]["structuredContent"];
        let shown = &sample["response"]["result"]["structuredContent"];
        assert_eq!(answer["outcome"], shown["outcome"], "{answer}");
        assert_eq!(answer["notices"], shown["notices"]);
        assert_eq!(
            answer["results"].as_array().unwrap().len(),
            shown["results"].as_array().unwrap().len()
        );
        for (got, sample) in answer["results"]
            .as_array()
            .unwrap()
            .iter()
            .zip(shown["results"].as_array().unwrap())
        {
            assert_eq!(got["title"], sample["title"]);
            assert_eq!(got["snippet"], sample["snippet"]);
            assert_eq!(got["date"], sample["date"]);
            // The URL's host is the mock's here and Wikipedia's in the
            // sample (examples/README.md); the path is the same.
            let got_url = reqwest::Url::parse(got["url"].as_str().unwrap()).unwrap();
            let sample_url = reqwest::Url::parse(sample["url"].as_str().unwrap()).unwrap();
            assert_eq!(got_url.path(), sample_url.path());
        }
        s.finish();
    })
    .await
    .unwrap();
}
