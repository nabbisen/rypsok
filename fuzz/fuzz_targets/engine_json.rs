//! A provider's answer, any bytes, through the Wikipedia and the Brave
//! readers (RFC 007; RFC 020, TST-14). Neither panics; a Wikipedia result
//! has a URL the pipeline admits whenever it is short enough; a URL the
//! pipeline admits, of either provider, is `http` or `https`.
//!
//! (The readers do not cut the list to the requested count: the pipeline
//! does, after it has judged each URL.)
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::engine::drivers::{brave, wikipedia};
use rypsok_core::search::normalize::returned_url;

const MAX_URL_CHARS: u32 = 2048;

fuzz_target!(|data: &[u8]| {
    let Some((&which, body)) = data.split_first() else { return };
    let mirror = url::Url::parse("http://127.0.0.1:9/").unwrap();
    let results = match which % 5 {
        0 => wikipedia::read_results(body, None, "en"),
        1 => wikipedia::read_results(body, None, "ja"),
        2 => wikipedia::read_results(body, Some(&mirror), "en"),
        3 => brave::read_results(body, brave::Endpoint::Web),
        _ => brave::read_results(body, brave::Endpoint::News),
    };
    let Ok(results) = results else { return };
    for result in &results {
        match returned_url(&result.url, MAX_URL_CHARS) {
            Ok(url) => assert!(url.starts_with("http://") || url.starts_with("https://"), "{url}"),
            Err(rejection) => {
                // A URL the reader built itself is always admitted, unless
                // it is longer than the pipeline's bound.
                assert!(
                    which % 5 >= 3 || result.url.chars().count() > MAX_URL_CHARS as usize,
                    "{} rejected: {rejection:?}",
                    result.url
                );
            }
        }
    }
});
