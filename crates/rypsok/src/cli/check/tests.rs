//! Tests of `rypsok check` (RFC 020, DAT-1, DAT-2, D-6, D-20, D-21). Every
//! test names the requirement it verifies.

use super::*;
use crate::mcp::http::HttpSettings;

/// A scratch directory per test, removed at the end.
struct Dir(std::path::PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rypsok-check-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Dir(path)
    }

    /// A file with mode 0600 holding `content`; its path as TOML text.
    fn file(&self, name: &str, content: &str) -> String {
        let path = self.0.join(name);
        std::fs::write(&path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        path.to_str().unwrap().replace('\\', "/")
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The report of a configuration text: parse, validate, build.
fn report_of(text: &str) -> Result<Report, ConfigError> {
    let config = Config::parse(text)?;
    config.validate()?;
    build(&config, &ConfigSource::Defaults)
}

fn all_lines(report: &Report) -> String {
    report
        .lines
        .iter()
        .chain(&report.warnings)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

const TOKEN_40: &str = "0123456789abcdefghijABCDEFGHIJ0123456789";

/// REQ-OPS-001 (RFC 020, DAT-1): `check` resolves the callers' tokens as
/// `serve --transport http` does, and fails with the same message: an
/// unset variable, an unreadable file and a token below the minimum are
/// exit 1 in `check` too; a good token passes; a file without callers
/// resolves nothing.
#[test]
fn req_ops_001_check_resolves_the_callers_like_serve() {
    let dir = Dir::new("callers");
    let short = dir.file("short.token", "12345");
    let good = dir.file("good.token", TOKEN_40);
    let missing = dir
        .0
        .join("missing.token")
        .to_str()
        .unwrap()
        .replace('\\', "/");
    let callers = |source: &str| {
        format!("config_version = 1\n[[service.http.auth.callers]]\nid = \"alpha\"\n{source}\n")
    };
    for (name, text) in [
        (
            "an unset variable",
            callers("token_env = \"RYPSOK_TEST_CHECK_NO_SUCH_VARIABLE\""),
        ),
        (
            "a token of 5 bytes",
            callers(&format!("token_file = \"{short}\"")),
        ),
        (
            "an unreadable file",
            callers(&format!("token_file = \"{missing}\"")),
        ),
    ] {
        let config = Config::parse(&text).unwrap();
        config.validate().unwrap();
        let serve = HttpSettings::from_config(&config).unwrap_err();
        let check = report_of(&text)
            .err()
            .unwrap_or_else(|| panic!("{name}: accepted"));
        assert_eq!(check.problems, serve.problems, "{name}");
        assert!(
            check.problems[0].starts_with("service.http.auth.callers[0]: "),
            "{name}: {check}"
        );
    }
    let ok = report_of(&callers(&format!("token_file = \"{good}\""))).unwrap();
    assert!(
        ok.lines.iter().any(|l| l.contains("callers=1")),
        "{}",
        all_lines(&ok)
    );
    // Without callers nothing is resolved.
    assert!(report_of("config_version = 1\n").is_ok());
}

/// REQ-OPS-001 (RFC 020, DAT-2): `check` prints a line for every section.
/// The values are the effective ones (a changed value shows), and the
/// defaults show too.
#[test]
fn req_ops_001_check_prints_every_section() {
    let dir = Dir::new("sections");
    let token = dir.file("token", TOKEN_40);
    let text = format!(
        "config_version = 1\n\
         [service.http]\nbind = \"127.0.0.1:9001\"\nlegacy_revisions = false\nmax_connections = 11\nmax_request_bytes = 8192\nidle_timeout_s = 90\nmax_streams = 7\nrequest_timeout_s = 40\nmax_sessions = 33\n\
         [[service.http.auth.callers]]\nid = \"alpha\"\ntoken_file = \"{token}\"\n\
         [[service.http.auth.callers]]\nid = \"beta\"\ntoken_file = \"{token}\"\n\
         [fetch]\nports = [80, 443]\nmax_bytes = 1048576\nmax_chars = 4000\nmax_chars_ceiling = 9000\nrobots_txt = true\nfollow_cross_host_redirects = false\n\
         [egress]\nallow = [\"198.51.100.7\", \"example.org\"]\ndeny = [\"203.0.113.0/24\"]\nallow_http = false\n\
         [privacy]\nsecret_detection = \"mask\"\n\
         [logging]\nlevel = \"debug\"\nlog_queries = false\n\
         [cache]\nenabled = true\nmax_entries = 50\nttl_s = 120\n\
         [scraping]\nenabled = false\n\
         [resilience]\nfailures_to_open = 4\nfirst_cooldown_s = 20\nmax_cooldown_s = 300\n\
         [limits]\nfetches_in_flight = 3\nparsing_in_flight = 1\n"
    );
    let report = report_of(&text).unwrap();
    let line = |start: &str| -> String {
        let found: Vec<&String> = report
            .lines
            .iter()
            .filter(|l| l.starts_with(start))
            .collect();
        assert_eq!(
            found.len(),
            1,
            "{start}: {found:?} in\n{}",
            all_lines(&report)
        );
        found[0].clone()
    };
    for (start, wanted) in [
        (
            "rypsok: service.http:",
            "bind=127.0.0.1:9001 callers=2 legacy_revisions=false max_connections=11 max_request_bytes=8192 idle_timeout_s=90 max_streams=7 request_timeout_s=40 max_sessions=33",
        ),
        (
            "rypsok: fetch:",
            "ports=80,443 max_bytes=1048576 max_chars=4000 (ceiling 9000) deadline_ms=10000 max_redirects=5 robots_txt=true follow_cross_host_redirects=false",
        ),
        (
            "rypsok: egress:",
            "allow_rules=2 deny_rules=1 allow_http=false strict_mode=false proxy=none",
        ),
        ("rypsok: privacy:", "secret_detection=mask"),
        ("rypsok: logging:", "level=debug log_queries=false"),
        (
            "rypsok: cache:",
            "enabled=true max_entries=50 ttl_s=120 max_entry_bytes=32768",
        ),
        ("rypsok: scraping:", "enabled=false"),
        (
            "rypsok: resilience:",
            "failures_to_open=4 first_cooldown_s=20 max_cooldown_s=300",
        ),
        (
            "rypsok: limits:",
            "outbound_in_flight=32 per_engine_in_flight=2 fetches_in_flight=3 parsing_in_flight=1 strict_mode_urls=10000 robots_cache_entries=1000",
        ),
    ] {
        let found = line(start);
        assert!(
            found.contains(wanted),
            "{start}\n  wanted: {wanted}\n  got:    {found}"
        );
    }
    // The defaults, with the proxy and strict mode switched on.
    let defaults = report_of(
        "config_version = 1\n[fetch]\nstrict_mode = true\n[egress]\nproxy = \"http://proxy.example:3128\"\n",
    )
    .unwrap();
    let egress = defaults
        .lines
        .iter()
        .find(|l| l.starts_with("rypsok: egress:"))
        .unwrap();
    assert!(egress.contains("strict_mode=true proxy=set"), "{egress}");
}

/// REQ-SEC-020 and REQ-PRIV-004 (RFC 020, DAT-2): nothing secret is in
/// the output: no token, no API key, no text of a rule, no proxy address
/// (it may carry a password).
#[test]
fn req_sec_020_check_never_prints_a_secret() {
    let dir = Dir::new("secrets");
    let token = dir.file("token", "TOKENVALUE-0123456789abcdefghijklmnopqrstuv");
    let key = dir.file("key", "APIKEYVALUE-zyxwvutsrqponmlkjihgfedcba9876543210");
    let text = format!(
        "config_version = 1\n\
         [[service.http.auth.callers]]\nid = \"alpha\"\ntoken_file = \"{token}\"\n\
         [egress]\nallow = [\"198.51.100.0/24\", \"secret-host.example\"]\ndeny = [\"203.0.113.77\"]\nproxy = \"http://user:PROXYPASSWORD@proxy.example:3128\"\n\
         [engines.b]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key_file = \"{key}\"\n"
    );
    let report = report_of(&text).unwrap();
    let output = all_lines(&report);
    for secret in [
        "TOKENVALUE",
        "APIKEYVALUE",
        "PROXYPASSWORD",
        "proxy.example",
        "198.51.100.0",
        "secret-host.example",
        "203.0.113.77",
    ] {
        assert!(!output.contains(secret), "`{secret}` in:\n{output}");
    }
    assert!(output.contains("callers=1"), "{output}");
    assert!(output.contains("credential found"), "{output}");
}

/// The warning lines of a configuration text.
fn warnings_of(text: &str) -> Vec<String> {
    report_of(text).unwrap().warnings
}

/// REQ-OPS-001 (RFC 020, D-6): one configuration per warning, and the exact
/// line. Each is `rypsok: warning: <key>: <what happens>; <the safer
/// setting>`; the exit code stays 0 (the report is `Ok`).
#[test]
fn req_ops_001_each_risky_setting_has_its_warning() {
    let dir = Dir::new("warnings");
    let token = dir.file("token", TOKEN_40);
    let cases: &[(&str, String, &str)] = &[
        (
            "an allow rule wider than /16",
            "config_version = 1\n[egress]\nallow = [\"10.0.0.0/8\"]\n".to_owned(),
            "rypsok: warning: egress.allow[0]: the rule admits a network wider than /16 (IPv4) or /32 (IPv6), which can include private networks and metadata addresses; list hosts or a narrower network",
        ),
        (
            "an allow rule for every address",
            "config_version = 1\n[egress]\nallow = [\"example.org\", \"0.0.0.0/0\"]\n".to_owned(),
            "rypsok: warning: egress.allow[1]: the rule admits a network wider than /16 (IPv4) or /32 (IPv6), which can include private networks and metadata addresses; list hosts or a narrower network",
        ),
        (
            "an IPv6 allow rule wider than /32",
            "config_version = 1\n[egress]\nallow = [\"fc00::/7\"]\n".to_owned(),
            "rypsok: warning: egress.allow[0]: the rule admits a network wider than /16 (IPv4) or /32 (IPv6), which can include private networks and metadata addresses; list hosts or a narrower network",
        ),
        (
            "secret detection off",
            "config_version = 1\n[privacy]\nsecret_detection = \"off\"\n".to_owned(),
            "rypsok: warning: privacy.secret_detection: \"off\" sends queries and URLs without the credential check; use \"refuse\", the default",
        ),
        (
            "a plaintext listener off loopback",
            format!("config_version = 1\n[service.http]\nbind = \"192.0.2.10:8737\"\n[[service.http.auth.callers]]\nid = \"alpha\"\ntoken_file = \"{token}\"\n"),
            "rypsok: warning: service.http.bind: the listener speaks plain HTTP and this bind is not loopback, so tokens and traffic cross the network unencrypted; bind loopback behind a TLS-terminating proxy",
        ),
        (
            "queries in the log",
            "config_version = 1\n[logging]\nlog_queries = true\n".to_owned(),
            "rypsok: warning: logging.log_queries: query text is written to the debug log; set it to false",
        ),
        (
            "robots.txt off",
            "config_version = 1\n[fetch]\nrobots_txt = false\n".to_owned(),
            "rypsok: warning: fetch.robots_txt: false fetches pages without reading robots.txt; set it to true",
        ),
        (
            "plain http",
            "config_version = 1\n[egress]\nallow_http = true\n".to_owned(),
            "rypsok: warning: egress.allow_http: plain http URLs are fetched as they are, without the https retry; set it to false",
        ),
        (
            "cross-host redirects",
            "config_version = 1\n[fetch]\nfollow_cross_host_redirects = true\n".to_owned(),
            "rypsok: warning: fetch.follow_cross_host_redirects: a redirect to another host is followed, so a page can lead a fetch to a host the caller did not name; set it to false",
        ),
        (
            "a scraping engine",
            "config_version = 1\n[scraping]\nenabled = true\n[engines.ddg]\ntype = \"duckduckgo\"\ncategories = [\"general\"]\n".to_owned(),
            "rypsok: warning: engines.ddg: scraping engine of type \"duckduckgo\" reads a human-facing page under the provider's terms and breaks when the page changes; prefer an engine with an API",
        ),
        (
            "a cache with no storable engine",
            "config_version = 1\n[cache]\nenabled = true\n[engines.d]\ntype = \"duckduckgo\"\ncategories = [\"general\"]\nenabled = true\n[scraping]\nenabled = true\n[engines.wikipedia]\ntype = \"wikipedia\"\ncategories = [\"general\"]\nenabled = false\n[engines.brave]\ntype = \"brave\"\ncategories = [\"general\"]\napi_key_env = \"X\"\nenabled = false\n".to_owned(),
            "rypsok: warning: cache.enabled: no enabled engine allows its results to be stored, so the cache holds nothing and only keeps queries in memory; turn the cache off or enable an engine that allows storage",
        ),
        (
            "more parses than the unit holds",
            "config_version = 1\n[limits]\nparsing_in_flight = 16\n".to_owned(),
            "rypsok: warning: memory: the worst case is about 616 MiB, above the 256 MiB of the documented unit (an estimate measured on the musl release build); lower fetch.max_bytes, limits.fetches_in_flight, limits.parsing_in_flight or service.http.max_sessions",
        ),
    ];
    let mut wrong = Vec::new();
    for (name, text, wanted) in cases {
        let warnings = warnings_of(text);
        if !warnings.iter().any(|w| w == wanted) {
            wrong.push(format!("{name}: wanted\n    {wanted}\n  got {warnings:#?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// REQ-OPS-001 (RFC 020, D-6): settings that are not risky give no warning:
/// the defaults, `legacy_revisions` either way, narrow allow rules and host
/// names, a loopback bind with callers, a cache with a storable engine, and
/// a worst case at the new ceilings.
#[test]
fn req_ops_001_safe_settings_give_no_warning() {
    let dir = Dir::new("quiet");
    let token = dir.file("token", TOKEN_40);
    for (name, text) in [
        ("the defaults", "config_version = 1\n".to_owned()),
        ("no file", String::new()),
        (
            "legacy revisions off",
            "config_version = 1\n[service.http]\nlegacy_revisions = false\n".to_owned(),
        ),
        (
            "legacy revisions on",
            "config_version = 1\n[service.http]\nlegacy_revisions = true\n".to_owned(),
        ),
        (
            "narrow rules",
            "config_version = 1\n[egress]\nallow = [\"10.1.0.0/16\", \"198.51.100.7\", \"docs.rs\", \"*.example.org\", \"2001:db8::/32\"]\ndeny = [\"0.0.0.0/0\"]\n".to_owned(),
        ),
        (
            "loopback with callers",
            format!("config_version = 1\n[[service.http.auth.callers]]\nid = \"alpha\"\ntoken_file = \"{token}\"\n"),
        ),
        (
            "a cache with Wikipedia",
            "config_version = 1\n[cache]\nenabled = true\n".to_owned(),
        ),
        (
            "the new max_bytes ceiling with the defaults (184 MiB)",
            "config_version = 1\n[fetch]\nmax_bytes = 10485760\n".to_owned(),
        ),
        (
            "the new max_sessions ceiling with the defaults (224 MiB)",
            "config_version = 1\n[service.http]\nmax_sessions = 2048\n".to_owned(),
        ),
    ] {
        let text = if name == "no file" { "config_version = 1\n".to_owned() } else { text };
        assert_eq!(warnings_of(&text), Vec::<String>::new(), "{name}");
    }
}

/// REQ-PERF-003 (RFC 020, D-20, D-21): each new ceiling fits the unit
/// alone, both together (289 MiB) do not, and `check` says so.
#[test]
fn req_perf_003_both_new_ceilings_together_exceed_the_unit() {
    let both = warnings_of(
        "config_version = 1\n[fetch]\nmax_bytes = 10485760\n[service.http]\nmax_sessions = 2048\n",
    );
    assert_eq!(both.len(), 1, "{both:?}");
    assert!(both[0].contains("about 289 MiB"), "{both:?}");
}

/// REQ-PERF-003 (RFC 020, D-20, D-21): `check` prints the memory estimate;
/// the defaults give 119 MiB, the new `max_bytes` ceiling stays under the
/// unit, and the line shows its parts.
#[test]
fn req_perf_003_check_prints_the_memory_estimate() {
    let line = |text: &str| -> String {
        report_of(text)
            .unwrap()
            .lines
            .into_iter()
            .find(|l| l.starts_with("rypsok: memory:"))
            .expect("a memory line")
    };
    assert_eq!(
        line("config_version = 1\n"),
        "rypsok: memory: worst case about 119 MiB (base 13 + bodies 20 + parses 71 + sessions 15 + cache 0 MiB; an estimate measured on the musl release build)"
    );
    let ceiling = line("config_version = 1\n[fetch]\nmax_bytes = 10485760\n");
    assert!(ceiling.contains("about 184 MiB"), "{ceiling}");
    let cached = line("config_version = 1\n[cache]\nenabled = true\n");
    assert!(cached.contains("cache 32 MiB"), "{cached}");
    assert!(
        report_of("config_version = 1\n[fetch]\nmax_bytes = 10485760\n")
            .unwrap()
            .warnings
            .is_empty()
    );
}
