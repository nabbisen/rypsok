//! Tests of the result cache store (RFC 014). Every test names the
//! requirement it verifies; the pipeline tests cover the integration.

use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::engine::RawResult;
use crate::privacy::Sensitive;

/// Poisons the lock of `cache` from a thread of its own (REQ-CACHE-006).
pub(crate) fn poison(cache: &Arc<ResultCache>) {
    let poisoner = Arc::clone(cache);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.state.lock().unwrap();
        panic!("poison the cache lock");
    })
    .join();
}

fn settings() -> CacheSettings {
    CacheSettings {
        ttl: Duration::from_secs(300),
        max_entries: 3,
        max_entry_bytes: 64,
    }
}

fn request(query: &str) -> EngineRequest {
    EngineRequest {
        query: Sensitive::new(query.to_owned()),
        max_results: 5,
        language: Language::new("en").unwrap(),
        region: "US".to_owned(),
        safe_search: SafeSearch::Moderate,
        time_range: None,
    }
}

fn key(caller: &str, query: &str) -> CacheKey {
    CacheKey::new(
        &CallerId::new(caller),
        &EngineId::new("w").unwrap(),
        &request(query),
    )
}

fn answer(text: &str) -> EngineResponse {
    EngineResponse {
        results: vec![RawResult::new(text, "https://example.org/", "")],
    }
}

/// REQ-CACHE-005: an entry is returned before its lifetime and not after;
/// an expired entry is removed on lookup.
#[tokio::test(start_paused = true)]
async fn req_cache_005_entries_expire_after_the_lifetime() {
    let cache = ResultCache::new(settings());
    cache.store(key("local", "rust"), &answer("one")).unwrap();
    tokio::time::advance(Duration::from_secs(299)).await;
    assert_eq!(cache.lookup(&key("local", "rust")), Some(answer("one")));
    tokio::time::advance(Duration::from_secs(2)).await;
    assert_eq!(cache.lookup(&key("local", "rust")), None);
    assert!(cache.is_empty(), "the expired entry was removed");
}

/// REQ-CACHE-005 (LIM-K3): when full, the least recently used entry goes,
/// and a lookup counts as a use.
#[tokio::test(start_paused = true)]
async fn req_cache_005_eviction_is_least_recently_used() {
    let cache = ResultCache::new(settings());
    for q in ["a", "b", "c"] {
        cache.store(key("local", q), &answer(q)).unwrap();
    }
    // `a` is used, so `b` is the least recently used.
    assert!(cache.lookup(&key("local", "a")).is_some());
    cache.store(key("local", "d"), &answer("d")).unwrap();
    assert_eq!(cache.len(), 3);
    assert!(cache.lookup(&key("local", "b")).is_none(), "b was evicted");
    assert!(cache.lookup(&key("local", "a")).is_some());
    assert!(cache.lookup(&key("local", "c")).is_some());
    assert!(cache.lookup(&key("local", "d")).is_some());
}

/// REQ-CACHE-005 (LIM-K3): expired entries are evicted before live ones.
#[tokio::test(start_paused = true)]
async fn req_cache_005_expired_entries_are_evicted_first() {
    let cache = ResultCache::new(settings());
    cache.store(key("local", "old"), &answer("old")).unwrap();
    tokio::time::advance(Duration::from_secs(301)).await;
    cache.store(key("local", "b"), &answer("b")).unwrap();
    cache.store(key("local", "c"), &answer("c")).unwrap();
    // With one lifetime for every entry the expired one is also the least
    // recently used; the test pins the behavior either way.
    assert!(cache.lookup(&key("local", "b")).is_some());
    assert!(cache.lookup(&key("local", "c")).is_some());
    cache.store(key("local", "d"), &answer("d")).unwrap();
    assert_eq!(cache.len(), 3);
    assert!(cache.lookup(&key("local", "old")).is_none());
    assert!(cache.lookup(&key("local", "b")).is_some());
    assert!(cache.lookup(&key("local", "c")).is_some());
    assert!(cache.lookup(&key("local", "d")).is_some());
}

/// REQ-CACHE-004: keys are exact; two callers with the same query never
/// share an entry, and any difference in the request is a different key.
#[test]
fn req_cache_004_keys_are_exact_per_caller_and_request() {
    let cache = ResultCache::new(settings());
    cache.store(key("a", "rust"), &answer("for a")).unwrap();
    assert_eq!(cache.lookup(&key("b", "rust")), None);
    assert_eq!(cache.lookup(&key("a", "rust")), Some(answer("for a")));
    let mut other = request("rust");
    other.time_range = Some(TimeRange::Day);
    let other_key = CacheKey::new(&CallerId::new("a"), &EngineId::new("w").unwrap(), &other);
    assert_eq!(cache.lookup(&other_key), None);
    assert!(
        !format!("{:?}", key("a", "rust")).contains("rust"),
        "the query is not printed"
    );
}

/// LIM-K4: an answer above the entry bound is not stored.
#[test]
fn req_cache_005_an_answer_above_the_entry_bound_is_not_stored() {
    let cache = ResultCache::new(settings());
    let big = answer(&"x".repeat(100));
    assert_eq!(
        cache.store(key("local", "big"), &big),
        Err(NotStored::TooLarge)
    );
    assert!(cache.is_empty());
    assert_eq!(raw_bytes(&big), 100 + "https://example.org/".len());
}

/// REQ-CACHE-006: a poisoned lock disables the cache; lookups miss and
/// stores report it, nothing panics.
#[test]
fn req_cache_006_a_poisoned_lock_disables_the_cache() {
    let cache = Arc::new(ResultCache::new(settings()));
    cache.store(key("local", "rust"), &answer("one")).unwrap();
    poison(&cache);
    assert_eq!(cache.lookup(&key("local", "rust")), None);
    assert!(cache.is_disabled());
    assert_eq!(
        cache.store(key("local", "rust"), &answer("one")),
        Err(NotStored::Disabled)
    );
    cache.clear();
}

/// External design §16 I11: `clear` empties the cache.
#[test]
fn req_cache_003_clear_empties_the_cache() {
    let cache = ResultCache::new(settings());
    cache.store(key("local", "rust"), &answer("one")).unwrap();
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.lookup(&key("local", "rust")), None);
}

// --- Mutants the audit run left alive (RFC 020, TST-7) ----------------------

/// REQ-CACHE-005 (LIM-K4): an answer of exactly the entry bound is stored and
/// one byte more is not (a mutant of the comparison was missed).
#[test]
fn req_cache_005_an_entry_of_exactly_the_bound_is_stored() {
    let one = answer("one");
    let exact = raw_bytes(&one);
    let at_bound = ResultCache::new(CacheSettings {
        max_entry_bytes: exact,
        ..settings()
    });
    assert_eq!(at_bound.store(key("local", "rust"), &one), Ok(()));
    let below = ResultCache::new(CacheSettings {
        max_entry_bytes: exact - 1,
        ..settings()
    });
    assert_eq!(
        below.store(key("local", "rust"), &one),
        Err(NotStored::TooLarge)
    );
}

/// REQ-CACHE-006: the cache that fails says so once, at `warn`, with the
/// event `cache_disabled`; later calls are silent misses (a mutant that never
/// said it was missed).
#[test]
fn req_cache_006_the_failure_is_logged_once() {
    let log = crate::telemetry::tests::routed::capture();
    let cache = Arc::new(ResultCache::new(settings()));
    poison(&cache);
    assert!(
        log.lines().is_empty(),
        "nothing is said before the cache is used"
    );
    assert_eq!(cache.lookup(&key("local", "rust")), None);
    assert_eq!(cache.lookup(&key("local", "rust")), None);
    assert_eq!(
        cache.store(key("local", "rust"), &answer("one")),
        Err(NotStored::Disabled)
    );
    let lines = log.lines();
    assert_eq!(lines.len(), 1, "{lines:#?}");
    assert_eq!(lines[0]["event"], "cache_disabled");
    assert_eq!(lines[0]["level"], "WARN");
    assert_eq!(lines[0]["reason"], "poisoned");
}

/// REQ-CACHE-005 (LIM-K3, RFC 020 TST-7): a lookup moves the entry to the
/// most recent place, and does it every time: after `a` is used the order of
/// the others is `b`, `c`, and after `d` and `e` arrive the oldest two are
/// gone in that order (a mutant that left the use counter where it was
/// survived the one-eviction test).
#[tokio::test(start_paused = true)]
async fn req_cache_005_every_lookup_renews_the_entry() {
    let cache = ResultCache::new(settings());
    for q in ["a", "b", "c"] {
        cache.store(key("local", q), &answer(q)).unwrap();
    }
    assert!(cache.lookup(&key("local", "a")).is_some());
    cache.store(key("local", "d"), &answer("d")).unwrap();
    cache.store(key("local", "e"), &answer("e")).unwrap();
    assert_eq!(cache.len(), 3);
    // `b` went first, `c` second; `a`, used after both arrived, is still here.
    assert!(cache.lookup(&key("local", "b")).is_none());
    assert!(cache.lookup(&key("local", "c")).is_none());
    assert!(cache.lookup(&key("local", "a")).is_some());
    assert!(cache.lookup(&key("local", "d")).is_some());
    assert!(cache.lookup(&key("local", "e")).is_some());
}

/// REQ-CACHE-006 and REQ-PRIV-004: a healthy cache is not disabled, a failed
/// one is; a key prints its type, its caller and its engine and hides the
/// query (mutants of `is_disabled` and of the key's `Debug` were missed).
#[test]
fn req_cache_006_a_cache_is_enabled_until_it_fails_and_its_key_hides_the_query() {
    let cache = Arc::new(ResultCache::new(settings()));
    assert!(!cache.is_disabled());
    poison(&cache);
    assert_eq!(cache.lookup(&key("local", "rust")), None);
    assert!(cache.is_disabled());

    let printed = format!("{:?}", key("alpha", "a-private-query"));
    assert!(printed.starts_with("CacheKey"), "{printed}");
    assert!(
        printed.contains("alpha") && printed.contains("[redacted]"),
        "{printed}"
    );
    assert!(!printed.contains("a-private-query"), "{printed}");
}

/// REQ-CACHE-003 (RFC 020, TST-7): an entry is valid only for the caller, the
/// engine and the request parameters it was produced for: a difference in any
/// one of them is a miss, and `clear` (what a reload of the configuration
/// would do) empties the cache. The configuration and the egress policy are
/// fixed for the life of the process, so they are not part of the key.
#[test]
fn req_cache_003_an_entry_is_valid_only_for_its_caller_engine_and_parameters() {
    let cache = ResultCache::new(settings());
    let caller = CallerId::new("a");
    let engine = EngineId::new("w").unwrap();
    let base = request("rust");
    cache
        .store(CacheKey::new(&caller, &engine, &base), &answer("one"))
        .unwrap();
    let hit = |caller: &CallerId, engine: &EngineId, request: &EngineRequest| {
        cache
            .lookup(&CacheKey::new(caller, engine, request))
            .is_some()
    };
    assert!(hit(&caller, &engine, &base));
    assert!(!hit(&CallerId::new("b"), &engine, &base), "caller");
    assert!(
        !hit(&caller, &EngineId::new("other").unwrap(), &base),
        "engine"
    );
    let changed = |change: &dyn Fn(&mut EngineRequest)| {
        let mut other = request("rust");
        change(&mut other);
        hit(&caller, &engine, &other)
    };
    assert!(
        !changed(&|r| r.query = Sensitive::new("rusty".to_owned())),
        "query"
    );
    assert!(
        !changed(&|r| r.language = Language::new("ja").unwrap()),
        "language"
    );
    assert!(!changed(&|r| r.region = "JP".to_owned()), "region");
    assert!(
        !changed(&|r| r.safe_search = SafeSearch::Off),
        "safe search"
    );
    assert!(
        !changed(&|r| r.time_range = Some(TimeRange::Week)),
        "time range"
    );
    assert!(!changed(&|r| r.max_results = 6), "result count");
    cache.clear();
    assert!(!hit(&caller, &engine, &base), "cleared");
}
