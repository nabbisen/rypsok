//! The scraping reader on any page in the DuckDuckGo shape (RFC 015).
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::engine::drivers::duckduckgo::{DuckDuckGo, SHAPE};
use rypsok_core::engine::scrape::{Page, read};
use rypsok_core::html::dom::parse;

fuzz_target!(|data: &[u8]| {
    let Ok(html) = std::str::from_utf8(data) else { return };
    let dom = parse(html, 4_000);
    match read(&dom, &SHAPE, 10, &DuckDuckGo::unwrap_link) {
        Page::Results(results) => {
            assert!(!results.is_empty() && results.len() <= 10);
            for result in results {
                assert!(result.url.starts_with("http://") || result.url.starts_with("https://"));
            }
        }
        Page::NoResults | Page::Challenge | Page::Drifted => {}
    }
});
