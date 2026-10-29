//! The decoded-text cleaner never panics, keeps its bound and is fully
//! idempotent (RFC 019 D3, REQ-TEST-013): it has no markup step and no
//! entity decoding, so there is no residual to exclude.
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::boundary::clean_decoded_text;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    let max = 50 + (data.len() as u32 % 400);
    let once = clean_decoded_text(text, max);
    assert!(once.text.chars().count() <= max as usize);
    assert!(!once.text.starts_with(' ') && !once.text.ends_with(' '));
    let twice = clean_decoded_text(&once.text, max);
    assert_eq!(twice.text, once.text);
    assert_eq!(twice.removed, 0);
});
