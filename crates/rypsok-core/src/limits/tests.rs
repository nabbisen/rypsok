//! Tests of the `limits` module. Every test names the requirement it verifies.

use super::*;

/// REQ-CFG-003: the default limits are the values of the table.
#[test]
fn req_cfg_003_defaults_are_the_table() {
    let limits = Limits::default();
    assert_eq!(limits.query_max_chars, 400);
    assert_eq!(limits.max_results, 5);
    assert_eq!(limits.max_results_ceiling, 20);
    assert_eq!(limits.search_deadline_ms, 2500);
    assert_eq!(limits.engine_timeout_ms, 2000);
    assert_eq!(limits.search_reserve_ms, 200);
    assert_eq!(limits.search_call_max_ms(), 2800);
    assert_eq!(limits.fetch_call_max_ms(), 11000);
    assert_eq!(limits.fetch_ports, vec![80, 443]);
    assert!(limits.engine_timeout_ms + limits.search_reserve_ms <= limits.search_deadline_ms);
}
