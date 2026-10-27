//! URL normalization and comparison keys on any text (RFC 008).
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::search::normalize::{comparison_key, returned_url};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    if let Ok(url) = returned_url(text, 2048) {
        assert!(url.chars().count() <= 2048);
        assert!(url.starts_with("http://") || url.starts_with("https://"));
        let _ = comparison_key(&url);
    }
});
