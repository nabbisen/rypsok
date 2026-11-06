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

const MIB: u64 = 1024 * 1024;

fn limits_with(f: impl FnOnce(&mut Limits)) -> Limits {
    let mut limits = Limits::default();
    f(&mut limits);
    limits
}

/// REQ-PERF-003 (RFC 020 D-20, D-21): the worst-case memory is the measured
/// formula of audit 020e: base 13 MiB, bodies `fetches_in_flight x
/// max_bytes`, parses `parsing_in_flight x (4.5 x max_bytes + 13 MiB)`,
/// sessions `max_sessions x 60 KiB`, plus the cache bound when it is on.
#[test]
fn req_perf_003_the_memory_formula_of_the_audit() {
    let estimate = Limits::default().worst_case_memory(0);
    assert_eq!(estimate.base, 13 * MIB);
    assert_eq!(estimate.bodies, 4 * 5 * MIB);
    // 4.5 x 5 MiB = 22.5 MiB, plus 13 MiB, twice.
    assert_eq!(estimate.parses, 2 * (45 * 5 * MIB / 10 + 13 * MIB));
    assert_eq!(estimate.sessions, 256 * 60 * 1024);
    assert_eq!(estimate.cache, 0);
    let total = 13 * MIB + 20 * MIB + 71 * MIB + 15 * MIB;
    assert_eq!(estimate.total_bytes(), total);
    assert_eq!(estimate.total_mib(), 119);
    assert!(!estimate.exceeds_unit());
    // The defaults, without the sessions, are the 104 MiB the measurement
    // compares with (90 to 98 MiB at four 5 MiB pages, 102 to 106 in the
    // test with a larger `max_bytes`).
    assert_eq!(estimate.total_bytes() - estimate.sessions, 104 * MIB);
}

/// REQ-PERF-003 (RFC 020 D-20): `fetch.max_bytes` at its new ceiling with
/// the defaults is under the unit's 256 MiB; the old ceiling was not.
#[test]
fn req_perf_003_the_new_ceiling_fits_the_unit_and_the_old_one_did_not() {
    let at_ceiling = limits_with(|l| l.fetch_max_bytes = 10 * MIB).worst_case_memory(0);
    assert_eq!(at_ceiling.total_mib(), 184, "{at_ceiling:?}");
    assert!(!at_ceiling.exceeds_unit());
    let old_ceiling = limits_with(|l| l.fetch_max_bytes = 20 * MIB).worst_case_memory(0);
    assert!(old_ceiling.exceeds_unit(), "{old_ceiling:?}");
    // The audit measured 265 to 282 MiB at that setting; the formula says
    // about 325 MiB, on the safe side of the measurement.
    assert!(old_ceiling.total_mib() >= 282);
}

/// REQ-PERF-003: more parses than the defaults is a warning; so are many
/// sessions; and the cache bound counts when the cache is on.
#[test]
fn req_perf_003_the_formula_follows_each_setting() {
    let sixteen = limits_with(|l| l.parsing_in_flight = 16).worst_case_memory(0);
    assert!(sixteen.exceeds_unit(), "{sixteen:?}");
    let sessions = limits_with(|l| l.http_max_sessions = 2048).worst_case_memory(0);
    assert_eq!(sessions.sessions, 2048 * 60 * 1024);
    assert_eq!(sessions.total_mib(), 119 - 15 + 120);
    let cache = Limits::default().worst_case_memory(1000 * 32768);
    assert_eq!(cache.cache, 1000 * 32768);
    assert_eq!(cache.total_bytes(), 119 * MIB + 1000 * 32768);
    // The estimate rounds up to a whole MiB.
    assert_eq!(MemoryEstimate::mib(1), 1);
    assert_eq!(MemoryEstimate::mib(MIB), 1);
    assert_eq!(MemoryEstimate::mib(MIB + 1), 2);
    assert_eq!(MemoryEstimate::mib(0), 0);
}

/// REQ-PERF-003: the constants of the formula are the rows of the measured
/// costs block of the limits table (`tools/check-limits` compares them too).
#[test]
fn req_perf_003_the_measured_costs_are_the_table_rows() {
    assert_eq!(defaults::MEASURED_BASE_BYTES, 13 * MIB);
    assert_eq!(defaults::MEASURED_PARSE_FIXED_BYTES, 13 * MIB);
    assert_eq!(defaults::MEASURED_PARSE_TENTHS_PER_BYTE, 45);
    assert_eq!(defaults::MEASURED_SESSION_BYTES, 60 * 1024);
    assert_eq!(defaults::MEMORY_UNIT_BYTES, 256 * MIB);
}
