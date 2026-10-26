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
