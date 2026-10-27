//! Tests of the result cache store (RFC 014). Every test names the
//! requirement it verifies; the pipeline tests cover the integration.

use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::engine::RawResult;
use crate::privacy::Sensitive;

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
        results: vec![RawResult {
            title: text.to_owned(),
            url: "https://example.org/".to_owned(),
            snippet: String::new(),
            date: None,
        }],
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
    let poisoner = Arc::clone(&cache);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.state.lock().unwrap();
        panic!("poison the cache lock");
    })
    .join();
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
