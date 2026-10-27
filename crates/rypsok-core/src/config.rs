//! Configuration: the TOML schema, its validation and secrets by reference
//! (external design §13.2; RFC 004).
//!
//! One file, located by `--config` or `RYPSOK_CONFIG`, with documented
//! defaults for every key (REQ-CFG-001). Unknown keys are refused
//! (REQ-CFG-002). Every default is the value of the limits table
//! (REQ-CFG-003). Validation names the offending key (REQ-CFG-005).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::limits::{Limits, defaults};
use crate::model::{Category, EngineId, Language};

pub mod secret;

pub use secret::{Secret, SecretError, SecretSource};

/// The environment variable that names the configuration file when
/// `--config` is absent.
pub const CONFIG_ENV: &str = "RYPSOK_CONFIG";

/// The only schema version this release reads (REQ-MAIN-003).
pub const CONFIG_VERSION: u32 = 1;

/// Every operational setting of a process (external design §13.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Schema version; must be [`CONFIG_VERSION`].
    pub config_version: u32,
    /// `[service]`.
    #[serde(default)]
    pub service: ServiceConfig,
    /// `[search]`.
    #[serde(default)]
    pub search: SearchConfig,
    /// `[fetch]` (used from milestone M3).
    #[serde(default)]
    pub fetch: FetchConfig,
    /// `[egress]`.
    #[serde(default)]
    pub egress: EgressConfig,
    /// `[limits]`.
    #[serde(default)]
    pub limits: LimitsConfig,
    /// `[resilience]` (used from milestone M2).
    #[serde(default)]
    pub resilience: ResilienceConfig,
    /// `[privacy]`.
    #[serde(default)]
    pub privacy: PrivacyConfig,
    /// `[logging]`.
    #[serde(default)]
    pub logging: LoggingConfig,
    /// `[status]`.
    #[serde(default)]
    pub status: StatusConfig,
    /// `[cache]` (RFC 014).
    #[serde(default)]
    pub cache: CacheConfig,
    /// `[engines.<id>]`, keyed by engine identifier.
    #[serde(default = "default_engines")]
    pub engines: BTreeMap<String, EngineTable>,
}

/// `[service]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServiceConfig {
    /// Whether tool results carry `structuredContent` (REQ-MCP-011).
    pub structured_content: bool,
    /// Appended to the User-Agent: a URL or an address (REQ-CMP-004).
    pub user_agent_contact: String,
    /// `[service.http]`.
    pub http: HttpConfig,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        ServiceConfig {
            structured_content: true,
            user_agent_contact: String::new(),
            http: HttpConfig::default(),
        }
    }
}

/// `[service.http]` (RFC 013).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpConfig {
    /// The address and port to bind; loopback by default (REQ-SEC-018).
    pub bind: String,
    /// `Host` values accepted; empty means loopback names and the bind host.
    pub allowed_hosts: Vec<String>,
    /// `Origin` values accepted; empty means every present `Origin` is refused.
    pub allowed_origins: Vec<String>,
    /// Serve the handshake-based revisions with sessions in memory
    /// (REQ-MCP-006).
    pub legacy_revisions: bool,
    /// LIM-H1.
    pub max_connections: u32,
    /// LIM-H2.
    pub max_request_bytes: u64,
    /// LIM-H3.
    pub idle_timeout_s: u32,
    /// `[service.http.auth]`.
    pub auth: HttpAuthConfig,
}

impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            bind: "127.0.0.1:8737".to_owned(),
            allowed_hosts: Vec::new(),
            allowed_origins: Vec::new(),
            legacy_revisions: true,
            max_connections: defaults::HTTP_MAX_CONNECTIONS,
            max_request_bytes: defaults::HTTP_MAX_REQUEST_BYTES,
            idle_timeout_s: defaults::HTTP_IDLE_TIMEOUT_S,
            auth: HttpAuthConfig::default(),
        }
    }
}

impl HttpConfig {
    /// The bind address, when it parses.
    pub fn bind_address(&self) -> Option<std::net::SocketAddr> {
        self.bind.parse().ok()
    }
}

/// `[service.http.auth]` (RFC 013, D-27).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpAuthConfig {
    /// `[[service.http.auth.callers]]`: the callers and their tokens by
    /// reference.
    pub callers: Vec<CallerConfig>,
}

/// One caller: an identifier and a token read from the environment or a
/// file, never inline (REQ-CFG-004).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CallerConfig {
    /// The identifier: lowercase letters, digits and underscores, starting
    /// with a letter.
    pub id: String,
    /// The environment variable that holds the token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_env: Option<String>,
    /// The file whose trimmed content is the token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_file: Option<String>,
}

impl CallerConfig {
    /// Where the token is read from.
    pub fn secret_source(&self) -> Option<SecretSource> {
        match (&self.token_env, &self.token_file) {
            (Some(env), _) => Some(SecretSource::Env(env.clone())),
            (None, Some(file)) => Some(SecretSource::File(PathBuf::from(file))),
            (None, None) => None,
        }
    }
}

/// The safe-search level passed to engines that support it (REQ-SEARCH-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SafeSearch {
    /// Off.
    Off,
    /// Moderate; the default.
    Moderate,
    /// Strict.
    Strict,
}

/// `[search]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchConfig {
    /// Languages offered to callers; the first is the default (REQ-SEARCH-004).
    pub languages: Vec<Language>,
    /// Default region, two uppercase letters (REQ-SEARCH-008).
    pub region: String,
    /// Safe-search level (REQ-SEARCH-008).
    pub safe_search: SafeSearch,
    /// LIM-S2, default.
    pub max_results: u8,
    /// LIM-S2, upper bound.
    pub max_results_ceiling: u8,
    /// LIM-S1.
    pub query_max_chars: u32,
    /// LIM-S4.
    pub deadline_ms: u32,
    /// LIM-S5.
    pub engine_timeout_ms: u32,
    /// LIM-S6.
    pub reserve_ms: u32,
    /// LIM-S8.
    pub engines_per_request: u8,
    /// LIM-S3.
    pub results_per_engine: u8,
    /// LIM-S9.
    pub snippet_max_chars: u32,
    /// LIM-S10.
    pub title_max_chars: u32,
    /// Drop results with instruction-like content (REQ-SEC-003).
    pub drop_flagged_results: bool,
    /// Always carry `results[].engines` (REQ-CTX-001).
    pub engine_attribution: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            languages: vec![Language::new("en").expect("en is a language")],
            region: "US".to_owned(),
            safe_search: SafeSearch::Moderate,
            max_results: defaults::MAX_RESULTS,
            max_results_ceiling: defaults::MAX_RESULTS_CEILING,
            query_max_chars: defaults::QUERY_MAX_CHARS,
            deadline_ms: defaults::SEARCH_DEADLINE_MS,
            engine_timeout_ms: defaults::ENGINE_TIMEOUT_MS,
            reserve_ms: defaults::SEARCH_RESERVE_MS,
            engines_per_request: defaults::ENGINES_PER_REQUEST,
            results_per_engine: defaults::RESULTS_PER_ENGINE,
            snippet_max_chars: defaults::SNIPPET_MAX_CHARS,
            title_max_chars: defaults::TITLE_MAX_CHARS,
            drop_flagged_results: false,
            engine_attribution: false,
        }
    }
}

/// `[fetch]` (milestone M3; keys accepted from M1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FetchConfig {
    /// LIM-F1.
    pub max_chars: u32,
    /// LIM-F2.
    pub max_chars_ceiling: u32,
    /// LIM-F3.
    pub max_bytes: u64,
    /// LIM-F4.
    pub deadline_ms: u32,
    /// LIM-F6.
    pub connect_timeout_ms: u32,
    /// LIM-F7.
    pub max_redirects: u8,
    /// LIM-F8.
    pub ports: Vec<u16>,
    /// Honor robots.txt (REQ-CMP-005).
    pub robots_txt: bool,
    /// Follow redirects to another host (REQ-FETCH-006).
    pub follow_cross_host_redirects: bool,
    /// Strict mode (REQ-SEC-021).
    pub strict_mode: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        FetchConfig {
            max_chars: defaults::FETCH_MAX_CHARS,
            max_chars_ceiling: defaults::FETCH_MAX_CHARS_CEILING,
            max_bytes: defaults::FETCH_MAX_BYTES,
            deadline_ms: defaults::FETCH_DEADLINE_MS,
            connect_timeout_ms: defaults::FETCH_CONNECT_TIMEOUT_MS,
            max_redirects: defaults::FETCH_MAX_REDIRECTS,
            ports: defaults::FETCH_PORTS.to_vec(),
            robots_txt: true,
            follow_cross_host_redirects: false,
            strict_mode: false,
        }
    }
}

/// `[egress]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EgressConfig {
    /// Allow plain HTTP fetches (REQ-FETCH-010).
    pub allow_http: bool,
    /// CIDRs or host names admitted explicitly.
    pub allow: Vec<String>,
    /// CIDRs or host names refused in addition.
    pub deny: Vec<String>,
    /// Proxy for engine traffic only; empty means none (REQ-RES-008).
    pub proxy: String,
}

/// `[limits]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LimitsConfig {
    /// LIM-C1.
    pub outbound_in_flight: u32,
    /// LIM-C2.
    pub per_engine_in_flight: u8,
    /// LIM-C3.
    pub fetches_in_flight: u32,
    /// LIM-C5.
    pub parsing_in_flight: u32,
    /// LIM-C6.
    pub web_search_per_minute: u32,
    /// LIM-C7.
    pub web_fetch_per_minute: u32,
    /// LIM-C8.
    pub engine_status_per_minute: u32,
    /// LIM-F14.
    pub strict_mode_urls: u32,
    /// LIM-F15.
    pub robots_cache_entries: u32,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        LimitsConfig {
            outbound_in_flight: defaults::OUTBOUND_IN_FLIGHT,
            per_engine_in_flight: defaults::PER_ENGINE_IN_FLIGHT,
            fetches_in_flight: defaults::FETCHES_IN_FLIGHT,
            parsing_in_flight: defaults::PARSING_IN_FLIGHT,
            web_search_per_minute: defaults::WEB_SEARCH_PER_MINUTE,
            web_fetch_per_minute: defaults::WEB_FETCH_PER_MINUTE,
            engine_status_per_minute: defaults::ENGINE_STATUS_PER_MINUTE,
            strict_mode_urls: defaults::STRICT_MODE_URLS,
            robots_cache_entries: defaults::ROBOTS_CACHE_ENTRIES,
        }
    }
}

/// `[resilience]` (RFC 010).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResilienceConfig {
    /// LIM-B1.
    pub failures_to_open: u8,
    /// LIM-B2.
    pub first_cooldown_s: u32,
    /// LIM-B3.
    pub max_cooldown_s: u32,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        ResilienceConfig {
            failures_to_open: defaults::FAILURES_TO_OPEN,
            first_cooldown_s: defaults::FIRST_COOLDOWN_S,
            max_cooldown_s: defaults::MAX_COOLDOWN_S,
        }
    }
}

/// The secret-detection mode (LIM-P1; RFC 009).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretDetection {
    /// No scan.
    Off,
    /// Log and count; the call proceeds unchanged.
    Warn,
    /// Replace each finding with `[redacted]` and notice it.
    Mask,
    /// Refuse the call; the default.
    Refuse,
}

/// `[privacy]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrivacyConfig {
    /// LIM-P1.
    pub secret_detection: SecretDetection,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        PrivacyConfig {
            secret_detection: SecretDetection::Refuse,
        }
    }
}

/// The log level (LIM-P2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Process-level failures only.
    Error,
    /// Policy refusals and configuration warnings.
    Warn,
    /// One event per call; the default.
    Info,
    /// Per-engine events.
    Debug,
}

/// `[logging]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingConfig {
    /// LIM-P2.
    pub level: LogLevel,
    /// Debug mode of REQ-PRIV-004: query text in `debug` events; warns at start.
    pub log_queries: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: LogLevel::Info,
            log_queries: false,
        }
    }
}

/// `[status]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatusConfig {
    /// Whether `engine_status` is offered (REQ-STATUS-003).
    pub enabled: bool,
}

impl Default for StatusConfig {
    fn default() -> Self {
        StatusConfig { enabled: true }
    }
}

/// `[cache]`: the result cache of RFC 014. Off by default (LIM-K1). When
/// on, the queries of the last `ttl_s` seconds are retained in memory
/// as cache keys (REQ-CACHE-007).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CacheConfig {
    /// LIM-K1.
    pub enabled: bool,
    /// LIM-K2.
    pub ttl_s: u32,
    /// LIM-K3.
    pub max_entries: u32,
    /// LIM-K4.
    pub max_entry_bytes: u32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            enabled: false,
            ttl_s: defaults::CACHE_TTL_S,
            max_entries: defaults::CACHE_ENTRIES,
            max_entry_bytes: defaults::CACHE_ENTRY_BYTES,
        }
    }
}

/// `[engines.<id>]`: the common keys and the keys the driver declares.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineTable {
    /// The driver: `wikipedia` or `brave`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Whether the engine is used (REQ-ENG-003).
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// The categories the engine serves.
    #[serde(default)]
    pub categories: Vec<Category>,
    /// Lower first (REQ-ENG-007).
    #[serde(default = "default_preference")]
    pub preference: u32,
    /// Driver-declared keys, including `api_key_env` and `api_key_file`.
    #[serde(flatten)]
    pub keys: BTreeMap<String, toml::Value>,
}

impl EngineTable {
    /// The key as a string, when present and a string.
    pub fn key_str(&self, key: &str) -> Option<&str> {
        self.keys.get(key).and_then(toml::Value::as_str)
    }

    /// Where the engine's credential comes from, when the table names one.
    pub fn secret_source(&self) -> Option<SecretSource> {
        if let Some(env) = self.key_str("api_key_env") {
            Some(SecretSource::Env(env.to_owned()))
        } else {
            self.key_str("api_key_file")
                .map(|p| SecretSource::File(PathBuf::from(p)))
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_preference() -> u32 {
    100
}

fn default_engines() -> BTreeMap<String, EngineTable> {
    let mut engines = BTreeMap::new();
    engines.insert(
        "wikipedia".to_owned(),
        EngineTable {
            kind: "wikipedia".to_owned(),
            enabled: true,
            categories: vec![Category::General, Category::Science],
            preference: 10,
            keys: BTreeMap::new(),
        },
    );
    let mut brave_keys = BTreeMap::new();
    brave_keys.insert(
        "api_key_env".to_owned(),
        toml::Value::String("BRAVE_API_KEY".to_owned()),
    );
    engines.insert(
        "brave".to_owned(),
        EngineTable {
            kind: "brave".to_owned(),
            enabled: true,
            categories: vec![Category::General],
            preference: 20,
            keys: brave_keys,
        },
    );
    engines
}

impl Default for Config {
    /// The configuration of a process started without a file: every default
    /// of the limits table and the keyless engines (REQ-CFG-003).
    fn default() -> Self {
        Config {
            config_version: CONFIG_VERSION,
            service: ServiceConfig::default(),
            search: SearchConfig::default(),
            fetch: FetchConfig::default(),
            egress: EgressConfig::default(),
            limits: LimitsConfig::default(),
            resilience: ResilienceConfig::default(),
            privacy: PrivacyConfig::default(),
            logging: LoggingConfig::default(),
            status: StatusConfig::default(),
            cache: CacheConfig::default(),
            engines: default_engines(),
        }
    }
}

/// Why a configuration was refused: one line per problem, each naming the
/// key (REQ-CFG-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// The problems, in the order found.
    pub problems: Vec<String>,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, problem) in self.problems.iter().enumerate() {
            if i > 0 {
                f.write_str("\n")?;
            }
            f.write_str(problem)?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

/// Where a configuration came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    /// The file named by `--config`.
    Flag(PathBuf),
    /// The file named by `RYPSOK_CONFIG`.
    Env(PathBuf),
    /// No file: the defaults.
    Defaults,
}

impl Config {
    /// Locates and reads the configuration: `--config`, then
    /// `RYPSOK_CONFIG`, then the defaults (REQ-CFG-001). The result is
    /// parsed but not yet validated; call [`Config::validate`].
    pub fn locate(flag: Option<&Path>) -> Result<(Config, ConfigSource), ConfigError> {
        if let Some(path) = flag {
            return Config::read(path).map(|c| (c, ConfigSource::Flag(path.to_path_buf())));
        }
        if let Some(env) = std::env::var_os(CONFIG_ENV) {
            let path = PathBuf::from(env);
            return Config::read(&path).map(|c| (c, ConfigSource::Env(path)));
        }
        Ok((Config::default(), ConfigSource::Defaults))
    }

    /// Reads and parses one file.
    pub fn read(path: &Path) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|e| ConfigError {
            problems: vec![format!(
                "cannot read configuration file {}: {e}",
                path.display()
            )],
        })?;
        Config::parse(&text)
    }

    /// Parses TOML text. Unknown keys and tables are refused with their path
    /// (REQ-CFG-002); the schema version is checked (REQ-MAIN-003).
    pub fn parse(text: &str) -> Result<Config, ConfigError> {
        let config: Config = toml::from_str(text).map_err(|e| {
            let message = e.message().to_owned();
            let location = e
                .span()
                .map(|s| format!(" (at byte {})", s.start))
                .unwrap_or_default();
            let problem =
                if message.starts_with("unknown field") || message.starts_with("unknown variant") {
                    format!(
                        "unknown key {}{location}",
                        message
                            .trim_start_matches("unknown field ")
                            .trim_start_matches("unknown variant ")
                    )
                } else if message.starts_with("missing field `config_version`") {
                    "config_version: required; this release supports 1".to_owned()
                } else {
                    format!("{message}{location}")
                };
            ConfigError {
                problems: vec![problem],
            }
        })?;
        Ok(config)
    }

    /// Validates every rule of RFC 004 and returns all problems at once.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut problems = Vec::new();
        if self.config_version != CONFIG_VERSION {
            problems.push(format!(
                "config_version: only {CONFIG_VERSION} is supported"
            ));
        }
        let h = &self.service.http;
        match h.bind_address() {
            None => problems.push(
                "service.http.bind: must be an address and a port, such as 127.0.0.1:8737"
                    .to_owned(),
            ),
            Some(address) if !address.ip().is_loopback() && h.auth.callers.is_empty() => {
                problems.push(
                    "service.http.bind: a non-loopback bind needs [service.http.auth] callers"
                        .to_owned(),
                );
            }
            Some(_) => {}
        }
        // An unspecified address is not a name a client sends in `Host`,
        // so the default host list would admit nothing useful: the
        // operator names the hosts instead of discovering 400s (RFC 013
        // amendment of 2026-10-27).
        if h.bind_address().is_some_and(|a| a.ip().is_unspecified()) && h.allowed_hosts.is_empty() {
            problems.push(
                "service.http.bind: a bind on an unspecified address (0.0.0.0, ::) needs service.http.allowed_hosts"
                    .to_owned(),
            );
        }
        for (i, host) in h.allowed_hosts.iter().enumerate() {
            if host.trim().is_empty()
                || host.chars().any(char::is_whitespace)
                || host.contains("://")
            {
                problems.push(format!(
                    "service.http.allowed_hosts[{i}]: a host name or host:port, without a scheme"
                ));
            }
        }
        for (i, origin) in h.allowed_origins.iter().enumerate() {
            if origin != "null" && !origin.contains("://") {
                problems.push(format!(
                    "service.http.allowed_origins[{i}]: an origin with its scheme, such as https://app.example:443"
                ));
            }
        }
        range(
            &mut problems,
            "service.http.max_connections",
            h.max_connections,
            1,
            4096,
        );
        range(
            &mut problems,
            "service.http.max_request_bytes",
            h.max_request_bytes,
            4096,
            1_048_576,
        );
        range(
            &mut problems,
            "service.http.idle_timeout_s",
            h.idle_timeout_s,
            5,
            600,
        );
        let mut ids = std::collections::BTreeSet::new();
        for (i, caller) in h.auth.callers.iter().enumerate() {
            let key = format!("service.http.auth.callers[{i}]");
            let id_ok = caller
                .id
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase())
                && caller
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if !id_ok {
                problems.push(format!(
                    "{key}.id: lowercase letters, digits and underscores, starting with a letter"
                ));
            }
            if !ids.insert(caller.id.clone()) {
                problems.push(format!("{key}.id: duplicate"));
            }
            match (&caller.token_env, &caller.token_file) {
                (Some(_), Some(_)) => {
                    problems.push(format!("{key}: give token_env or token_file, not both"));
                }
                (None, None) => {
                    problems.push(format!("{key}: token_env or token_file is required"));
                }
                _ => {}
            }
        }
        let s = &self.search;
        if s.languages.is_empty() {
            problems.push("search.languages: at least one language is required".to_owned());
        }
        if !(s.region.len() == 2 && s.region.chars().all(|c| c.is_ascii_uppercase())) {
            problems.push("search.region: must be two uppercase letters".to_owned());
        }
        range(
            &mut problems,
            "search.query_max_chars",
            s.query_max_chars,
            1,
            600,
        );
        range(
            &mut problems,
            "search.max_results_ceiling",
            u32::from(s.max_results_ceiling),
            1,
            20,
        );
        range(
            &mut problems,
            "search.max_results",
            u32::from(s.max_results),
            1,
            u32::from(s.max_results_ceiling.clamp(1, 20)),
        );
        range(
            &mut problems,
            "search.results_per_engine",
            u32::from(s.results_per_engine),
            5,
            20,
        );
        range(
            &mut problems,
            "search.deadline_ms",
            s.deadline_ms,
            500,
            10000,
        );
        range(&mut problems, "search.reserve_ms", s.reserve_ms, 50, 1000);
        range(
            &mut problems,
            "search.engine_timeout_ms",
            s.engine_timeout_ms,
            100,
            s.deadline_ms.max(100),
        );
        if s.engine_timeout_ms + s.reserve_ms > s.deadline_ms {
            problems.push(
                "search.engine_timeout_ms + search.reserve_ms exceeds search.deadline_ms"
                    .to_owned(),
            );
        }
        range(
            &mut problems,
            "search.engines_per_request",
            u32::from(s.engines_per_request),
            1,
            16,
        );
        range(
            &mut problems,
            "search.snippet_max_chars",
            s.snippet_max_chars,
            50,
            1000,
        );
        range(
            &mut problems,
            "search.title_max_chars",
            s.title_max_chars,
            50,
            300,
        );
        let f = &self.fetch;
        range(
            &mut problems,
            "fetch.max_chars_ceiling",
            f.max_chars_ceiling,
            500,
            100_000,
        );
        range(
            &mut problems,
            "fetch.max_chars",
            f.max_chars,
            500,
            f.max_chars_ceiling.clamp(500, 100_000),
        );
        range(
            &mut problems,
            "fetch.max_bytes",
            f.max_bytes,
            65_536,
            20_971_520,
        );
        range(
            &mut problems,
            "fetch.deadline_ms",
            f.deadline_ms,
            1000,
            30000,
        );
        range(
            &mut problems,
            "fetch.connect_timeout_ms",
            f.connect_timeout_ms,
            500,
            10000,
        );
        range(
            &mut problems,
            "fetch.max_redirects",
            u32::from(f.max_redirects),
            0,
            10,
        );
        for port in &f.ports {
            if *port == 0 {
                problems.push("fetch.ports: 0 is not a port".to_owned());
            }
            if *port != 80 && *port != 443 && self.egress.allow.is_empty() {
                problems.push(format!("fetch.ports: {port} needs an egress.allow rule"));
            }
        }
        let l = &self.limits;
        range(
            &mut problems,
            "limits.outbound_in_flight",
            l.outbound_in_flight,
            1,
            256,
        );
        range(
            &mut problems,
            "limits.per_engine_in_flight",
            u32::from(l.per_engine_in_flight),
            1,
            8,
        );
        range(
            &mut problems,
            "limits.fetches_in_flight",
            l.fetches_in_flight,
            1,
            64,
        );
        range(
            &mut problems,
            "limits.parsing_in_flight",
            l.parsing_in_flight,
            1,
            16,
        );
        range(
            &mut problems,
            "limits.web_search_per_minute",
            l.web_search_per_minute,
            1,
            6000,
        );
        range(
            &mut problems,
            "limits.web_fetch_per_minute",
            l.web_fetch_per_minute,
            1,
            6000,
        );
        range(
            &mut problems,
            "limits.engine_status_per_minute",
            l.engine_status_per_minute,
            1,
            6000,
        );
        range(
            &mut problems,
            "limits.strict_mode_urls",
            l.strict_mode_urls,
            100,
            1_000_000,
        );
        range(
            &mut problems,
            "limits.robots_cache_entries",
            l.robots_cache_entries,
            10,
            100_000,
        );
        range(&mut problems, "cache.ttl_s", self.cache.ttl_s, 10, 3600);
        range(
            &mut problems,
            "cache.max_entries",
            self.cache.max_entries,
            1,
            100_000,
        );
        range(
            &mut problems,
            "cache.max_entry_bytes",
            self.cache.max_entry_bytes,
            1024,
            1_048_576,
        );
        if let Err(rule_problems) = crate::egress::EgressPolicy::from_texts(
            &self.egress.allow,
            &self.egress.deny,
            self.egress.allow_http,
            &self.fetch.ports,
        ) {
            problems.extend(rule_problems);
        }
        let r = &self.resilience;
        range(
            &mut problems,
            "resilience.failures_to_open",
            u32::from(r.failures_to_open),
            1,
            10,
        );
        range(
            &mut problems,
            "resilience.first_cooldown_s",
            r.first_cooldown_s,
            10,
            600,
        );
        range(
            &mut problems,
            "resilience.max_cooldown_s",
            r.max_cooldown_s,
            10,
            3600,
        );
        if r.first_cooldown_s > r.max_cooldown_s {
            problems
                .push("resilience.first_cooldown_s exceeds resilience.max_cooldown_s".to_owned());
        }
        for (id, table) in &self.engines {
            if EngineId::new(id.as_str()).is_err() {
                problems.push(format!("engines.{id}: not an engine identifier (lowercase letters, digits, underscores)"));
            }
            match crate::engine::drivers::declared_keys(&table.kind) {
                None => problems.push(format!("engines.{id}.type: no such driver")),
                Some(declared) => {
                    for key in table.keys.keys() {
                        if key == "api_key" {
                            problems.push(format!("engines.{id}.api_key: give the secret by api_key_env or api_key_file"));
                        } else if !declared.contains(&key.as_str()) {
                            problems.push(format!(
                                "engines.{id}.{key}: not a key of driver {}",
                                table.kind
                            ));
                        }
                    }
                }
            }
            if table.categories.is_empty() {
                problems.push(format!(
                    "engines.{id}.categories: at least one category is required"
                ));
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(ConfigError { problems })
        }
    }

    /// The effective limits (RFC 004).
    pub fn limits(&self) -> Limits {
        Limits {
            query_max_chars: self.search.query_max_chars,
            max_results: self.search.max_results,
            max_results_ceiling: self.search.max_results_ceiling,
            results_per_engine: self.search.results_per_engine,
            search_deadline_ms: self.search.deadline_ms,
            engine_timeout_ms: self.search.engine_timeout_ms,
            search_reserve_ms: self.search.reserve_ms,
            engines_per_request: self.search.engines_per_request,
            snippet_max_chars: self.search.snippet_max_chars,
            title_max_chars: self.search.title_max_chars,
            latency_window: defaults::LATENCY_WINDOW,
            engine_connect_timeout_ms: defaults::ENGINE_CONNECT_TIMEOUT_MS,
            fetch_max_chars: self.fetch.max_chars,
            fetch_max_chars_ceiling: self.fetch.max_chars_ceiling,
            fetch_max_bytes: self.fetch.max_bytes,
            fetch_deadline_ms: self.fetch.deadline_ms,
            fetch_connect_timeout_ms: self.fetch.connect_timeout_ms,
            fetch_max_redirects: self.fetch.max_redirects,
            fetch_ports: self.fetch.ports.clone(),
            url_max_chars: defaults::URL_MAX_CHARS,
            html_max_nodes: defaults::HTML_MAX_NODES,
            html_max_depth: defaults::HTML_MAX_DEPTH,
            robots_ttl_s: defaults::ROBOTS_TTL_S,
            robots_max_bytes: defaults::ROBOTS_MAX_BYTES,
            outbound_in_flight: self.limits.outbound_in_flight,
            per_engine_in_flight: self.limits.per_engine_in_flight,
            fetches_in_flight: self.limits.fetches_in_flight,
            parsing_in_flight: self.limits.parsing_in_flight,
            web_search_per_minute: self.limits.web_search_per_minute,
            web_fetch_per_minute: self.limits.web_fetch_per_minute,
            engine_status_per_minute: self.limits.engine_status_per_minute,
            strict_mode_urls: self.limits.strict_mode_urls,
            robots_cache_entries: self.limits.robots_cache_entries,
            http_max_connections: self.service.http.max_connections,
            http_max_request_bytes: self.service.http.max_request_bytes,
            http_idle_timeout_s: self.service.http.idle_timeout_s,
            failures_to_open: self.resilience.failures_to_open,
            first_cooldown_s: self.resilience.first_cooldown_s,
            max_cooldown_s: self.resilience.max_cooldown_s,
        }
    }

    /// The default language: the first configured one.
    pub fn default_language(&self) -> Language {
        self.search
            .languages
            .first()
            .cloned()
            .unwrap_or_else(|| Language::new("en").expect("en is a language"))
    }

    /// The categories at least one enabled engine serves, in vocabulary
    /// order (REQ-SEARCH-002).
    pub fn served_categories(&self) -> Vec<Category> {
        Category::ALL
            .into_iter()
            .filter(|c| {
                self.engines
                    .values()
                    .any(|e| e.enabled && e.categories.contains(c))
            })
            .collect()
    }
}

fn range<T: Into<u64> + Copy>(problems: &mut Vec<String>, key: &str, value: T, min: T, max: T) {
    let (v, lo, hi) = (value.into(), min.into(), max.into());
    if v < lo || v > hi {
        problems.push(format!("{key}: must be between {lo} and {hi}"));
    }
}

#[cfg(test)]
mod tests;
