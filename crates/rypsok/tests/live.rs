//! Live driver checks (REQ-TEST-003): one fixed query per driver against
//! the real provider, within its terms. Ignored by default; the scheduled
//! workflow runs them with `--ignored`. They report format drift and never
//! gate.

use rypsok_core::CancellationToken;
use rypsok_core::config::{Config, SafeSearch};
use rypsok_core::engine::EngineRequest;
use rypsok_core::engine::registry::Registry;
use rypsok_core::model::{Category, Language};

fn request(query: &str) -> EngineRequest {
    let mut request = EngineRequest::new(query, Language::new("en").unwrap());
    request.max_results = 5;
    request.safe_search = SafeSearch::Moderate;
    request
}

fn config(text: &str) -> Config {
    let contact = std::env::var("RYPSOK_LIVE_CONTACT")
        .unwrap_or_else(|_| "https://github.com/nabbisen/rypsok/issues".into());
    let config = Config::parse(&format!(
        "config_version = 1\n[service]\nuser_agent_contact = \"{contact}\"\n{text}"
    ))
    .unwrap();
    config.validate().unwrap();
    config
}

/// REQ-TEST-003: the Wikipedia driver parses a real response.
#[tokio::test]
#[ignore = "reaches the real provider; run by the scheduled workflow"]
async fn live_wikipedia_search() {
    let config = config("[engines.wikipedia]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n");
    let registry = Registry::build(&config, None, true);
    let engine = registry
        .select(Category::General, 1)
        .into_iter()
        .next()
        .expect("wikipedia active");
    let driver = engine.driver.as_deref().unwrap();
    let response = driver
        .search(
            &request("Rust (programming language)"),
            CancellationToken::new(),
        )
        .await
        .expect("wikipedia answers");
    assert!(!response.results.is_empty(), "wikipedia returned no result");
    assert!(
        response.results[0]
            .url
            .starts_with("https://en.wikipedia.org/wiki/")
    );
}

/// REQ-TEST-003: the Brave driver parses a real response; skipped and
/// reported without the repository secret.
#[tokio::test]
#[ignore = "reaches the real provider; run by the scheduled workflow"]
async fn live_brave_search() {
    if std::env::var("BRAVE_API_KEY")
        .map(|k| k.trim().is_empty())
        .unwrap_or(true)
    {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr(),
            "live_brave_search: skipped, BRAVE_API_KEY is not set"
        );
        return;
    }
    let config = config(
        "[engines.brave]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key_env = \"BRAVE_API_KEY\"\n",
    );
    let registry = Registry::build(&config, None, true);
    let engine = registry
        .select(Category::General, 1)
        .into_iter()
        .next()
        .expect("brave active");
    let driver = engine.driver.as_deref().unwrap();
    let response = driver
        .search(
            &request("rust programming language"),
            CancellationToken::new(),
        )
        .await
        .expect("brave answers");
    assert!(!response.results.is_empty(), "brave returned no result");
}

/// REQ-TEST-003 and RFC 015: the DuckDuckGo HTML driver against the real
/// endpoint with the honest User-Agent. A `blocked` answer is the
/// provider's decision and is reported as such, never worked around
/// (REQ-ENG-014).
#[tokio::test]
#[ignore = "reaches the real provider; run by the scheduled workflow"]
async fn live_duckduckgo_search() {
    let config = config(
        "[scraping]\nenabled = true\n[engines.ddg]\ntype = \"duckduckgo\"\ncategories = [\"general\"]\n",
    );
    let registry = Registry::build(&config, None, true);
    let engine = registry
        .select(Category::General, 1)
        .into_iter()
        .next()
        .expect("duckduckgo active");
    let driver = engine.driver.as_deref().unwrap();
    match driver
        .search(
            &request("Rust programming language"),
            CancellationToken::new(),
        )
        .await
    {
        Ok(response) => {
            assert!(
                !response.results.is_empty(),
                "duckduckgo returned no result"
            );
            assert!(response.results[0].url.starts_with("http"));
        }
        Err(error) => panic!(
            "duckduckgo answered {:?}: the provider refuses the honest agent or its format drifted",
            error.class
        ),
    }
}
