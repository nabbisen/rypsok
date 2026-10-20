//! Tests of the registry. Every test names the requirement it verifies.

use super::*;

fn config(text: &str) -> Config {
    let config = Config::parse(text).unwrap();
    config.validate().unwrap();
    config
}

/// REQ-ENG-007: selection returns active engines of the category by
/// preference, then identifier, at most the limit.
#[test]
fn req_eng_007_selection_by_category_preference_and_limit() {
    let config = config(
        "config_version = 1\n\
         [engines.b]\ntype = \"wikipedia\"\ncategories = [\"general\"]\npreference = 10\n\
         [engines.a]\ntype = \"wikipedia\"\ncategories = [\"general\", \"science\"]\npreference = 10\n\
         [engines.c]\ntype = \"wikipedia\"\ncategories = [\"general\"]\npreference = 5\n\
         [engines.d]\ntype = \"wikipedia\"\ncategories = [\"news\"]\n",
    );
    let registry = Registry::build(&config, None, true);
    let ids = |engines: Vec<&Engine>| {
        engines
            .iter()
            .map(|e| e.id.as_str().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(registry.select(Category::General, 8)),
        vec!["c", "a", "b"]
    );
    assert_eq!(ids(registry.select(Category::General, 2)), vec!["c", "a"]);
    assert_eq!(ids(registry.select(Category::Science, 8)), vec!["a"]);
    assert_eq!(
        ids(registry.select(Category::Code, 8)),
        Vec::<String>::new()
    );
}

/// REQ-ENG-003 and REQ-OPS-003: a disabled engine and an engine without a
/// credential are registered as `disabled` and never selected.
#[test]
fn req_ops_003_inactive_engines_are_disabled_not_fatal() {
    let config = config(
        "config_version = 1\n\
         [engines.wikipedia]\ntype = \"wikipedia\"\ncategories = [\"general\"]\nenabled = false\n\
         [engines.brave]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key_env = \"RYPSOK_TEST_UNSET_KEY_42\"\n\
         [engines.keyless_brave]\ntype = \"brave\"\ncategories = [\"general\"]\n",
    );
    let registry = Registry::build(&config, None, true);
    assert_eq!(registry.engines().len(), 3);
    for engine in registry.engines() {
        assert_eq!(engine.state(), EngineState::Disabled, "{}", engine.id);
        assert!(engine.driver.is_none());
    }
    assert!(registry.select(Category::General, 8).is_empty());
}

/// REQ-ENG-010 and REQ-CMP-002: an active engine carries its driver's
/// admission properties and a limiter at the record's rate.
#[test]
fn req_eng_010_active_engine_carries_properties() {
    let config = config(
        "config_version = 1\n[engines.wikipedia]\ntype = \"wikipedia\"\ncategories = [\"general\"]\n",
    );
    let registry = Registry::build(&config, None, true);
    let engine = &registry.engines()[0];
    assert_eq!(engine.state(), EngineState::Closed);
    let properties = engine.properties().unwrap();
    assert_eq!(properties.requests_per_minute, 100);
    assert_eq!(properties.max_in_flight, 1);
    assert!(!engine.limiter.is_blocked());
}
