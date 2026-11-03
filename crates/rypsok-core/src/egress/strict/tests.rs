//! Tests of strict mode's memory (REQ-SEC-021). Every test names the
//! requirement it verifies.

use super::*;
use crate::search::normalize::{comparison_key, returned_url};

/// REQ-SEC-021: a returned URL is remembered by its comparison key, so a
/// variant of the same resource passes; the set is bounded to its
/// capacity and forgets the oldest key first.
#[test]
fn req_sec_021_returned_urls_are_remembered_by_key_within_the_bound() {
    let set = ReturnedUrls::new(3);
    assert!(set.is_empty());
    // Tracking parameters leave in the returned URL (RFC 008); the key of
    // that URL is what is remembered.
    let tracked = returned_url("https://example.org/a?utm_source=x", 2048).unwrap();
    set.record(&comparison_key(&tracked));
    assert!(set.contains(&comparison_key("https://www.example.org/a")));
    assert!(!set.contains(&comparison_key("https://example.org/b")));
    set.record(&comparison_key("https://example.org/a"));
    assert_eq!(set.len(), 1, "a known key is not re-added");
    set.record(&comparison_key("https://example.org/b"));
    set.record(&comparison_key("https://example.org/c"));
    set.record(&comparison_key("https://example.org/d"));
    assert_eq!(set.len(), 3);
    assert!(
        !set.contains(&comparison_key("https://example.org/a")),
        "the oldest left"
    );
    assert!(set.contains(&comparison_key("https://example.org/d")));
}

/// REQ-SEC-021 (RFC 020, N1 of the 020n review): the memory of strict mode
/// does not see the fragment. A returned URL and the same URL with any
/// fragment are one fetch target (the fragment is never sent to the server);
/// two returned URLs that differ only by their fragment are one entry; a
/// different query is another URL.
#[test]
fn req_sec_021_strict_memory_ignores_the_fragment() {
    let set = ReturnedUrls::new(10);
    set.record(&comparison_key("https://x.example/p"));
    assert!(set.contains(&comparison_key("https://x.example/p#section")));
    assert!(set.contains(&comparison_key("https://x.example/p#")));
    assert!(!set.contains(&comparison_key("https://x.example/p?other=1")));
    assert!(!set.contains(&comparison_key("https://x.example/p/more#section")));
    // The other way: the returned URL has a fragment, the fetch has none.
    let other = ReturnedUrls::new(10);
    other.record(&comparison_key("https://x.example/app#/inbox"));
    assert!(other.contains(&comparison_key("https://x.example/app")));
    assert!(other.contains(&comparison_key("https://x.example/app#/settings")));
    // Two returned URLs that differ only by fragment: one strict entry.
    other.record(&comparison_key("https://x.example/app#/settings"));
    assert_eq!(other.len(), 1);
    // The deduplication key itself still tells them apart (D-8).
    assert_ne!(
        comparison_key("https://x.example/app#/inbox"),
        comparison_key("https://x.example/app#/settings")
    );
}
