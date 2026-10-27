//! Charset decoding, the bounded DOM, extraction and Markdown on any bytes (RFC 012).
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::fetch::{Media, render_document};
use rypsok_core::limits::Limits;

fuzz_target!(|data: &[u8]| {
    let limits = Limits {
        html_max_nodes: 4_000,
        ..Limits::default()
    };
    let document = render_document(data, Media::Html, None, &limits);
    assert!(document.title.chars().count() <= 10_000);
    let _ = render_document(data, Media::Text, Some("shift_jis"), &limits);
});
