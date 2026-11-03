//! Strict fetch mode (RFC 011; REQ-SEC-021, D-09): the URLs `web_search`
//! returned in this process, remembered by comparison key **without its
//! fragment** (the fragment is not sent to the server, so it names no other
//! fetch; RFC 020, N1 of the 020n review), bounded to LIM-F14 entries with
//! oldest-first eviction. Nothing persists across a restart (REQ-RES-009).

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

use crate::search::normalize::ComparisonKey;
use crate::sync;

/// The comparison keys of returned URLs.
#[derive(Debug)]
pub struct ReturnedUrls {
    capacity: usize,
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    order: VecDeque<ComparisonKey>,
    keys: HashSet<ComparisonKey>,
}

impl ReturnedUrls {
    /// A set remembering at most `capacity` keys (LIM-F14).
    pub fn new(capacity: u32) -> Self {
        ReturnedUrls {
            capacity: usize::try_from(capacity).unwrap_or(usize::MAX).max(1),
            inner: Mutex::new(Inner::default()),
        }
    }

    /// Remembers a returned URL (given by its comparison key; the fragment
    /// is dropped here, so that no caller can forget to); the oldest key
    /// leaves when the set is full. A key already present is not re-added.
    pub fn record(&self, key: &ComparisonKey) {
        let key = &key.without_fragment();
        let mut inner = sync::lock(&self.inner);
        if inner.keys.contains(key) {
            return;
        }
        if inner.order.len() >= self.capacity
            && let Some(oldest) = inner.order.pop_front()
        {
            inner.keys.remove(&oldest);
        }
        inner.order.push_back(key.clone());
        inner.keys.insert(key.clone());
    }

    /// Whether a URL with this key, whatever its fragment, was returned
    /// earlier.
    pub fn contains(&self, key: &ComparisonKey) -> bool {
        sync::lock(&self.inner)
            .keys
            .contains(&key.without_fragment())
    }

    /// How many keys are remembered.
    pub fn len(&self) -> usize {
        sync::lock(&self.inner).order.len()
    }

    /// Whether nothing is remembered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests;
