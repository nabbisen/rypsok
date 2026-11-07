//! What `rypsok check` says (REQ-CFG-005; RFC 020, DAT-1, DAT-2, D-6).

use rypsok_core::config::{Config, ConfigError, ConfigSource, LogLevel, SecretDetection};
use rypsok_core::egress::policy::is_wide_network_rule;
use rypsok_core::engine::drivers;
use rypsok_core::limits::MemoryEstimate;

use super::describe;
use crate::mcp::http::HttpSettings;

/// The lines `check` writes to standard error: the effective settings,
/// then the warnings. Nothing here is a secret: a token, a key or the text
/// of a rule is never put in a line.
pub(super) struct Report {
    /// The effective settings, one line each.
    pub lines: Vec<String>,
    /// The warnings, one line each (the exit code stays 0).
    pub warnings: Vec<String>,
}

/// Builds the report of a validated configuration, or the error `serve`
/// would stop with.
pub(super) fn build(config: &Config, source: &ConfigSource) -> Result<Report, ConfigError> {
    // What `serve --transport http` resolves, `check` resolves: a file that
    // names callers is a network configuration by intent, so a token that is
    // unset, unreadable or too short is the error `serve` stops with (RFC 020,
    // DAT-1). A file without callers and a loopback bind resolves nothing.
    let mut warnings = Vec::new();
    if !config.service.http.auth.callers.is_empty() {
        HttpSettings::from_config(config)?;
        for (i, caller) in config.service.http.auth.callers.iter().enumerate() {
            if let Some(source) = caller.secret_source()
                && let Ok((_, Some(warning))) = source.resolve()
            {
                warnings.push(format!(
                    "rypsok: warning: service.http.auth.callers[{i}]: {warning}"
                ));
            }
        }
    }
    let mut lines = Vec::new();
    let limits = config.limits();
    lines.push(format!(
        "rypsok: configuration from {} is valid",
        describe(source)
    ));
    lines.push(format!(
        "rypsok: search: query_max_chars={} max_results={} (1..={}) deadline_ms={} engine_timeout_ms={} connect_timeout_ms={} reserve_ms={} engines_per_request={} languages={} region={}",
        limits.query_max_chars,
        limits.max_results,
        limits.max_results_ceiling,
        limits.search_deadline_ms,
        limits.engine_timeout_ms,
        limits.engine_connect_timeout_ms,
        limits.search_reserve_ms,
        limits.engines_per_request,
        config
            .search
            .languages
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        config.search.region,
    ));
    lines.push(format!(
        "rypsok: tool rate limits per minute: web_search={} web_fetch={} engine_status={}; engine_status enabled={}; structured_content={}",
        limits.web_search_per_minute,
        limits.web_fetch_per_minute,
        limits.engine_status_per_minute,
        config.status.enabled,
        config.service.structured_content,
    ));
    lines.push(format!(
        "rypsok: service: shutdown_ms={}",
        config.service.shutdown_ms
    ));
    for (id, table) in &config.engines {
        let credential = match table.secret_source() {
            None => "no credential needed".to_owned(),
            Some(source) => match source.resolve() {
                Ok((_, warning)) => warning.map_or("credential found".to_owned(), |w| {
                    format!("credential found; {w}")
                }),
                Err(error) => format!("inactive: {error}"),
            },
        };
        lines.push(format!(
            "rypsok: engine {id}: type={} enabled={} categories={} preference={} {credential}",
            table.kind,
            table.enabled,
            table
                .categories
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join(","),
            table.preference,
        ));
    }
    let categories = config
        .served_categories()
        .iter()
        .map(|c| c.as_str())
        .collect::<Vec<_>>()
        .join(",");
    lines.push(format!(
        "rypsok: categories served: {}",
        if categories.is_empty() {
            "none"
        } else {
            &categories
        }
    ));
    sections(config, &mut lines);
    let memory = config.worst_case_memory();
    lines.push(format!(
        "rypsok: memory: worst case about {} MiB (base {} + bodies {} + parses {} + sessions {} + cache {} MiB; an estimate measured on the musl release build)",
        memory.total_mib(),
        MemoryEstimate::mib(memory.base),
        MemoryEstimate::mib(memory.bodies),
        MemoryEstimate::mib(memory.parses),
        MemoryEstimate::mib(memory.sessions),
        MemoryEstimate::mib(memory.cache),
    ));
    warnings.extend(risks(config, &memory));
    Ok(Report { lines, warnings })
}

fn mode(mode: SecretDetection) -> &'static str {
    match mode {
        SecretDetection::Off => "off",
        SecretDetection::Warn => "warn",
        SecretDetection::Mask => "mask",
        SecretDetection::Refuse => "refuse",
    }
}

fn level(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
    }
}

/// One compact line for each section that `check` printed nothing of
/// (RFC 020, DAT-2). Counts, never the text of a rule; whether a proxy is
/// set, never its address (it may carry a password).
fn sections(config: &Config, lines: &mut Vec<String>) {
    let limits = config.limits();
    let h = &config.service.http;
    lines.push(format!(
        "rypsok: service.http: bind={} callers={} legacy_revisions={} max_connections={} max_request_bytes={} idle_timeout_s={} max_streams={} request_timeout_s={} max_sessions={}",
        h.bind,
        h.auth.callers.len(),
        h.legacy_revisions,
        h.max_connections,
        h.max_request_bytes,
        h.idle_timeout_s,
        h.max_streams,
        h.request_timeout_s,
        h.max_sessions,
    ));
    let f = &config.fetch;
    lines.push(format!(
        "rypsok: fetch: ports={} max_bytes={} max_chars={} (ceiling {}) deadline_ms={} max_redirects={} robots_txt={} follow_cross_host_redirects={}",
        f.ports
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        f.max_bytes,
        f.max_chars,
        f.max_chars_ceiling,
        f.deadline_ms,
        f.max_redirects,
        f.robots_txt,
        f.follow_cross_host_redirects,
    ));
    let e = &config.egress;
    lines.push(format!(
        "rypsok: egress: allow_rules={} deny_rules={} allow_http={} strict_mode={} proxy={}",
        e.allow.len(),
        e.deny.len(),
        e.allow_http,
        f.strict_mode,
        if e.proxy.is_empty() { "none" } else { "set" },
    ));
    lines.push(format!(
        "rypsok: privacy: secret_detection={}",
        mode(config.privacy.secret_detection)
    ));
    lines.push(format!(
        "rypsok: logging: level={} log_queries={}",
        level(config.logging.level),
        config.logging.log_queries
    ));
    let c = &config.cache;
    lines.push(format!(
        "rypsok: cache: enabled={} max_entries={} ttl_s={} max_entry_bytes={}",
        c.enabled, c.max_entries, c.ttl_s, c.max_entry_bytes
    ));
    lines.push(format!(
        "rypsok: scraping: enabled={}",
        config.scraping.enabled
    ));
    let r = &config.resilience;
    lines.push(format!(
        "rypsok: resilience: failures_to_open={} first_cooldown_s={} max_cooldown_s={}",
        r.failures_to_open, r.first_cooldown_s, r.max_cooldown_s
    ));
    lines.push(format!(
        "rypsok: limits: outbound_in_flight={} per_engine_in_flight={} fetches_in_flight={} parsing_in_flight={} strict_mode_urls={} robots_cache_entries={}",
        limits.outbound_in_flight,
        limits.per_engine_in_flight,
        limits.fetches_in_flight,
        limits.parsing_in_flight,
        limits.strict_mode_urls,
        limits.robots_cache_entries,
    ));
}

/// The warnings of RFC 020 D-6: each is one line `rypsok: warning: <key>:
/// <what happens>; <the safer setting>`. They do not change the exit code.
/// `legacy_revisions` has none, by decision.
fn risks(config: &Config, memory: &MemoryEstimate) -> Vec<String> {
    let mut out = Vec::new();
    let mut warn = |key: &str, text: &str| out.push(format!("rypsok: warning: {key}: {text}"));
    for (i, text) in config.egress.allow.iter().enumerate() {
        if is_wide_network_rule(text) {
            warn(
                &format!("egress.allow[{i}]"),
                "the rule admits a network wider than /16 (IPv4) or /32 (IPv6), which can include private networks and metadata addresses; list hosts or a narrower network",
            );
        }
    }
    if config.privacy.secret_detection == SecretDetection::Off {
        warn(
            "privacy.secret_detection",
            "\"off\" sends queries and URLs without the credential check; use \"refuse\", the default",
        );
    }
    if config
        .service
        .http
        .bind_address()
        .is_some_and(|a| !a.ip().is_loopback())
    {
        warn(
            "service.http.bind",
            "the listener speaks plain HTTP and this bind is not loopback, so tokens and traffic cross the network unencrypted; bind loopback behind a TLS-terminating proxy",
        );
    }
    if config.logging.log_queries {
        warn(
            "logging.log_queries",
            "query text is written to the debug log; set it to false",
        );
    }
    if !config.fetch.robots_txt {
        warn(
            "fetch.robots_txt",
            "false fetches pages without reading robots.txt; set it to true",
        );
    }
    if config.egress.allow_http {
        warn(
            "egress.allow_http",
            "plain http URLs are fetched as they are, without the https retry; set it to false",
        );
    }
    if config.fetch.follow_cross_host_redirects {
        warn(
            "fetch.follow_cross_host_redirects",
            "a redirect to another host is followed, so a page can lead a fetch to a host the caller did not name; set it to false",
        );
    }
    for (id, table) in &config.engines {
        if table.enabled && drivers::is_scraping(&table.kind) {
            warn(
                &format!("engines.{id}"),
                &format!(
                    "scraping engine of type \"{}\" reads a human-facing page under the provider's terms and breaks when the page changes; prefer an engine with an API",
                    table.kind
                ),
            );
        }
    }
    if config.cache.enabled
        && !config
            .engines
            .values()
            .any(|t| t.enabled && drivers::is_cacheable(&t.kind))
    {
        warn(
            "cache.enabled",
            "no enabled engine allows its results to be stored, so the cache holds nothing and only keeps queries in memory; turn the cache off or enable an engine that allows storage",
        );
    }
    if memory.exceeds_unit() {
        warn(
            "memory",
            &format!(
                "the worst case is about {} MiB, above the 256 MiB of the documented unit (an estimate measured on the musl release build); lower fetch.max_bytes, limits.fetches_in_flight, limits.parsing_in_flight or service.http.max_sessions",
                memory.total_mib()
            ),
        );
    }
    out
}

#[cfg(test)]
mod tests;
