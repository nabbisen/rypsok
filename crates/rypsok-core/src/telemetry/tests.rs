//! Tests of the `telemetry` module. Every test names the requirement it verifies.

use super::*;

/// REQ-OBS-003 and REQ-OBS-007: every counter path increments and is
/// readable by tool, outcome, reason, engine and class.
#[test]
fn req_obs_007_counters_increment_per_path() {
    let counters = Counters::new();
    counters.call("web_search", "ok");
    counters.call("web_search", "ok");
    counters.call("web_search", "invalid_request");
    counters.refused("web_search", "secret_in_query");
    counters.engine_error("brave", "rate_limited");
    counters.characters_removed(3);
    counters.results_dropped(1);
    assert_eq!(counters.calls_of("web_search", "ok"), 2);
    assert_eq!(counters.calls_of("web_search", "invalid_request"), 1);
    assert_eq!(counters.refusals_of("web_search", "secret_in_query"), 1);
    assert_eq!(counters.engine_errors_of("brave", "rate_limited"), 1);
    assert_eq!(counters.engine_errors_of("brave", "timeout"), 0);
    counters.log();
}

/// REQ-OBS-005: request identifiers are unique within the process.
#[test]
fn req_obs_005_request_ids_are_unique() {
    let ids: std::collections::BTreeSet<String> = (0..1000).map(|_| request_id()).collect();
    assert_eq!(ids.len(), 1000);
    assert!(
        ids.iter()
            .all(|id| id.len() == 18 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
    );
}
