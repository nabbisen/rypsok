//! Tests of the `config` module. Every test names the requirement it verifies.

use super::*;

/// REQ-CFG-003: an empty configuration yields the documented defaults and
/// the keyless engines.
#[test]
fn req_cfg_003_empty_file_yields_defaults() {
    let config = Config::parse("config_version = 1\n").unwrap();
    config.validate().unwrap();
    assert_eq!(config.limits(), Limits::default());
    assert_eq!(config.search.languages, vec![Language::new("en").unwrap()]);
    assert_eq!(config.search.region, "US");
    assert_eq!(config.search.safe_search, SafeSearch::Moderate);
    assert_eq!(config.privacy.secret_detection, SecretDetection::Refuse);
    assert!(config.status.enabled);
    assert!(config.service.structured_content);
    assert_eq!(config.engines.len(), 2);
    assert_eq!(config.engines["wikipedia"].kind, "wikipedia");
    assert_eq!(
        config.engines["brave"].secret_source(),
        Some(SecretSource::Env("BRAVE_API_KEY".into()))
    );
    assert_eq!(
        config.served_categories(),
        vec![Category::General, Category::Science]
    );
}

/// REQ-CFG-002: every unknown key and table is refused with its path.
#[test]
fn req_cfg_002_unknown_keys_are_refused() {
    let err = Config::parse("config_version = 1\n[search]\nmax_result = 3\n").unwrap_err();
    assert!(
        err.problems[0].starts_with("unknown key `max_result`"),
        "{err}"
    );
    let err = Config::parse("config_version = 1\n[serach]\nmax_results = 3\n").unwrap_err();
    assert!(err.problems[0].starts_with("unknown key `serach`"), "{err}");
    let config = Config::parse("config_version = 1\n[engines.brave]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key_env = \"K\"\ncolour = 1\n").unwrap();
    let err = config.validate().unwrap_err();
    assert!(
        err.problems
            .iter()
            .any(|p| p == "engines.brave.colour: not a key of driver brave"),
        "{err}"
    );
}

/// REQ-MAIN-003: an unknown schema version is refused; a missing one too.
#[test]
fn req_main_003_schema_version_is_checked() {
    let err = Config::parse("config_version = 2\n")
        .unwrap()
        .validate()
        .unwrap_err();
    assert_eq!(err.problems, vec!["config_version: only 1 is supported"]);
    let err = Config::parse("[search]\nmax_results = 3\n").unwrap_err();
    assert!(
        err.problems[0].starts_with("config_version: required"),
        "{err}"
    );
}

/// REQ-CFG-005: values outside their range and the deadline rule are
/// refused with the key and the range; all problems are reported at once.
#[test]
fn req_cfg_005_ranges_and_rules_name_the_key() {
    let text = "config_version = 1\n[search]\nmax_results = 0\nquery_max_chars = 601\nengine_timeout_ms = 2400\nreserve_ms = 200\n[resilience]\nfirst_cooldown_s = 700\nmax_cooldown_s = 100\n";
    let err = Config::parse(text).unwrap().validate().unwrap_err();
    let problems = err.problems.join("\n");
    assert!(
        problems.contains("search.max_results: must be between 1 and 20"),
        "{problems}"
    );
    assert!(
        problems.contains("search.query_max_chars: must be between 1 and 600"),
        "{problems}"
    );
    assert!(
        problems
            .contains("search.engine_timeout_ms + search.reserve_ms exceeds search.deadline_ms"),
        "{problems}"
    );
    assert!(
        problems.contains("resilience.first_cooldown_s: must be between 10 and 600"),
        "{problems}"
    );
    assert!(
        problems.contains("resilience.first_cooldown_s exceeds resilience.max_cooldown_s"),
        "{problems}"
    );
}

/// REQ-CFG-005 and REQ-MCP-003: the HTTP transport is refused before M4.
#[test]
fn req_cfg_005_http_transport_refused_before_m4() {
    let err = Config::parse("config_version = 1\n[service]\ntransport = \"http\"\n")
        .unwrap()
        .validate()
        .unwrap_err();
    assert_eq!(
        err.problems,
        vec!["service.transport: http is not available in this release"]
    );
}

/// REQ-CFG-004: an inline secret is refused at startup.
#[test]
fn req_cfg_004_inline_secret_is_refused() {
    let text = "config_version = 1\n[engines.brave]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key = \"sk-xxx\"\n";
    let err = Config::parse(text).unwrap().validate().unwrap_err();
    assert_eq!(
        err.problems,
        vec!["engines.brave.api_key: give the secret by api_key_env or api_key_file"]
    );
}

/// REQ-CFG-004 and REQ-OPS-003: a secret is read from the environment or a
/// file; an unset source is an error that names the source, not a value.
#[test]
fn req_cfg_004_secret_sources_resolve_by_reference() {
    let name = "RYPSOK_TEST_SECRET_7F3A";
    assert_eq!(
        SecretSource::Env(name.into()).resolve().unwrap_err(),
        SecretError::EnvUnset(name.into())
    );
    let dir = std::env::temp_dir().join(format!("rypsok-secret-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("brave");
    std::fs::write(&path, " the-key \n").unwrap();
    let (secret, _warning) = SecretSource::File(path.clone()).resolve().unwrap();
    assert_eq!(secret.expose_secret(), "the-key");
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(matches!(
        SecretSource::File(path).resolve().unwrap_err(),
        SecretError::FileUnreadable(_)
    ));
}

/// REQ-SEC-020: a secret is redacted in debug output.
#[test]
fn req_sec_020_secret_debug_is_redacted() {
    let secret = Secret::new("the-key".to_owned());
    let rendered = format!("{secret:?}");
    assert_eq!(rendered, "[redacted]");
    assert!(!rendered.contains("the-key"));
}

/// REQ-ENG-003: engines are enabled and disabled by configuration.
#[test]
fn req_eng_003_engines_toggle_by_configuration() {
    let text = "config_version = 1\n[engines.wikipedia]\ntype = \"wikipedia\"\nenabled = false\ncategories = [\"general\"]\n";
    let config = Config::parse(text).unwrap();
    config.validate().unwrap();
    assert!(!config.engines["wikipedia"].enabled);
    assert!(config.served_categories().is_empty());
    let err = Config::parse(
        "config_version = 1\n[engines.x]\ntype = \"altavista\"\ncategories = [\"general\"]\n",
    )
    .unwrap()
    .validate()
    .unwrap_err();
    assert_eq!(err.problems, vec!["engines.x.type: no such driver"]);
}

/// REQ-CFG-001: precedence is `--config`, then `RYPSOK_CONFIG`, then defaults.
#[test]
fn req_cfg_001_precedence_flag_env_defaults() {
    let dir = std::env::temp_dir().join(format!("rypsok-config-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let flag = dir.join("flag.toml");
    std::fs::write(&flag, "config_version = 1\n[search]\nmax_results = 7\n").unwrap();
    let (config, source) = Config::locate(Some(&flag)).unwrap();
    assert_eq!(config.search.max_results, 7);
    assert_eq!(source, ConfigSource::Flag(flag.clone()));
    let missing = dir.join("missing.toml");
    let err = Config::locate(Some(&missing)).unwrap_err();
    assert!(err.problems[0].starts_with("cannot read configuration file"));
    std::fs::remove_dir_all(&dir).unwrap();
}
