//! The result cache (RFC 014; REQ-CACHE-001 to REQ-CACHE-007): one
//! engine's raw answer to one engine request for one caller, in memory,
//! for at most LIM-K2 seconds, in at most LIM-K3 entries of at most LIM-K4
//! bytes. Keys are exact tuples compared by equality, so two callers can
//! never share an entry. A failure of the cache disables it for the rest
//! of the process and costs nothing but latency (REQ-CACHE-006).
//!
//! Only the search side holds a handle: `web_fetch` cannot reach the cache
//! (REQ-CACHE-002).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

// Tokio's clock, as in the pipeline, so that tests pause it.
use tokio::time::Instant;

use crate::config::SafeSearch;
use crate::engine::{EngineRequest, EngineResponse};
use crate::model::{CallerId, EngineId, Language, TimeRange};

/// What the operator configured (`[cache]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheSettings {
    /// LIM-K2.
    pub ttl: Duration,
    /// LIM-K3.
    pub max_entries: usize,
    /// LIM-K4.
    pub max_entry_bytes: usize,
}

/// The exact key of an entry (RFC 014): the caller, the engine and the
/// engine request. The query is the text after the privacy check; it is
/// never printed.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    caller: CallerId,
    engine: EngineId,
    query: String,
    language: Language,
    region: String,
    safe_search: SafeSearch,
    time_range: Option<TimeRange>,
    max_results: u8,
}

impl CacheKey {
    /// The key of `engine`'s answer to `request` for `caller`.
    pub fn new(caller: &CallerId, engine: &EngineId, request: &EngineRequest) -> Self {
        CacheKey {
            caller: caller.clone(),
            engine: engine.clone(),
            query: request.query.expose().to_owned(),
            language: request.language.clone(),
            region: request.region.clone(),
            safe_search: request.safe_search,
            time_range: request.time_range,
            max_results: request.max_results,
        }
    }
}

impl fmt::Debug for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheKey")
            .field("caller", &self.caller)
            .field("engine", &self.engine)
            .field("query", &"[redacted]")
            .finish_non_exhaustive()
    }
}

/// Why an answer was not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotStored {
    /// The raw bytes of the answer exceed LIM-K4.
    TooLarge,
    /// The cache is disabled after a failure (REQ-CACHE-006).
    Disabled,
}

#[derive(Debug)]
struct Entry {
    response: EngineResponse,
    expires: Instant,
    /// The position in the use order; larger is more recent.
    used: u64,
}

#[derive(Debug, Default)]
struct State {
    entries: HashMap<CacheKey, Entry>,
    /// Least recently used first.
    order: BTreeMap<u64, CacheKey>,
    tick: u64,
}

/// The cache of one process.
#[derive(Debug)]
pub struct ResultCache {
    settings: CacheSettings,
    state: Mutex<State>,
    disabled: AtomicBool,
}

impl ResultCache {
    /// An empty cache.
    pub fn new(settings: CacheSettings) -> Self {
        ResultCache {
            settings,
            state: Mutex::new(State::default()),
            disabled: AtomicBool::new(false),
        }
    }

    /// The settings.
    pub fn settings(&self) -> CacheSettings {
        self.settings
    }

    /// Whether a failure disabled the cache (REQ-CACHE-006).
    pub fn is_disabled(&self) -> bool {
        self.disabled.load(Ordering::SeqCst)
    }

    /// Entries held now, expired ones included until they are swept.
    pub fn len(&self) -> usize {
        self.state.lock().map(|s| s.entries.len()).unwrap_or(0)
    }

    /// Whether nothing is held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The answer stored under `key`, when one is held and not expired.
    /// Never fails: a disabled cache misses.
    pub fn lookup(&self, key: &CacheKey) -> Option<EngineResponse> {
        let mut state = self.lock()?;
        let now = Instant::now();
        let expired = state.entries.get(key).is_some_and(|e| e.expires <= now);
        if expired {
            if let Some(entry) = state.entries.remove(key) {
                state.order.remove(&entry.used);
            }
            return None;
        }
        state.tick += 1;
        let tick = state.tick;
        let entry = state.entries.get_mut(key)?;
        let previous = std::mem::replace(&mut entry.used, tick);
        let response = entry.response.clone();
        state.order.remove(&previous);
        state.order.insert(tick, key.clone());
        Some(response)
    }

    /// Stores `response` under `key` for LIM-K2, evicting expired entries
    /// first and then the least recently used when the cache is full.
    /// Never fails: the reason an answer is not stored is returned for
    /// the counters.
    pub fn store(&self, key: CacheKey, response: &EngineResponse) -> Result<(), NotStored> {
        if raw_bytes(response) > self.settings.max_entry_bytes {
            return Err(NotStored::TooLarge);
        }
        let mut state = self.lock().ok_or(NotStored::Disabled)?;
        let now = Instant::now();
        if let Some(previous) = state.entries.remove(&key) {
            state.order.remove(&previous.used);
        }
        while state.entries.len() >= self.settings.max_entries.max(1) {
            let evict = state
                .order
                .iter()
                .find(|(_, k)| state.entries.get(k).is_some_and(|e| e.expires <= now))
                .or_else(|| state.order.iter().next())
                .map(|(used, k)| (*used, k.clone()));
            let Some((used, evicted)) = evict else {
                break;
            };
            state.order.remove(&used);
            state.entries.remove(&evicted);
        }
        state.tick += 1;
        let tick = state.tick;
        state.order.insert(tick, key.clone());
        state.entries.insert(
            key,
            Entry {
                response: response.clone(),
                expires: now + self.settings.ttl,
                used: tick,
            },
        );
        Ok(())
    }

    /// Poisons the lock from the calling thread, for tests of
    /// REQ-CACHE-006: call it on a spawned thread and join.
    #[cfg(test)]
    pub(crate) fn poison_for_test(&self) {
        let _guard = self.state.lock().expect("not yet poisoned");
        panic!("poisoning the cache lock for a test");
    }

    /// Removes everything (a future configuration reload; external design
    /// §16 I11).
    pub fn clear(&self) {
        if let Some(mut state) = self.lock() {
            *state = State::default();
        }
    }

    /// The lock, or `None` once the cache is disabled. A poisoned lock
    /// disables the cache with one `warn` event (REQ-CACHE-006).
    fn lock(&self) -> Option<std::sync::MutexGuard<'_, State>> {
        if self.disabled.load(Ordering::SeqCst) {
            return None;
        }
        match self.state.lock() {
            Ok(guard) => Some(guard),
            Err(_) => {
                if !self.disabled.swap(true, Ordering::SeqCst) {
                    tracing::warn!(
                        event = "cache_disabled",
                        reason = "poisoned",
                        "the result cache failed and is disabled for the rest of the process"
                    );
                }
                None
            }
        }
    }
}

/// The raw bytes of an answer: titles, snippets and URLs (LIM-K4).
pub fn raw_bytes(response: &EngineResponse) -> usize {
    response
        .results
        .iter()
        .map(|r| r.title.len() + r.snippet.len() + r.url.len())
        .sum()
}

#[cfg(test)]
mod tests;
