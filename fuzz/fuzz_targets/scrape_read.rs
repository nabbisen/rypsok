//! The scraping reader on any page in the DuckDuckGo shape (RFC 015).
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::engine::drivers::duckduckgo::{DuckDuckGo, SHAPE};
use rypsok_core::engine::scrape::{Page, read};
use rypsok_core::html::dom::parse;
use rypsok_core::search::normalize::returned_url;

fuzz_target!(|data: &[u8]| {
    let Ok(html) = std::str::from_utf8(data) else { return };
    let dom = parse(html, 4_000, 256);
    match read(&dom, &SHAPE, 10, &DuckDuckGo::unwrap_link) {
        Page::Results(results) => {
            assert!(!results.is_empty() && results.len() <= 10);
            // The driver hands over the provider's text: a link it unwrapped
            // is an `http` or `https` URL once parsed, but the text may be
            // written oddly (`https:/x`, `HTTPS://x`). The pipeline makes it
            // canonical with `returned_url`; what that accepts is `http(s)`.
            for result in results {
                let parsed = url::Url::parse(&result.url).expect("the driver kept a URL that parses");
                assert!(matches!(parsed.scheme(), "http" | "https"), "{}", result.url);
                if let Ok(canonical) = returned_url(&result.url, 2048) {
                    assert!(canonical.starts_with("http://") || canonical.starts_with("https://"));
                }
            }
        }
        Page::NoResults | Page::Challenge | Page::Drifted => {}
    }
});
