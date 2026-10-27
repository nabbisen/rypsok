//! The text cleaner never panics and keeps its bound (RFC 005, REQ-TEST-013).
//! Idempotence is asserted for inputs without `&`: entity-encoded markup
//! decodes on a second pass by the browser-faithful order, the residual
//! RFC 005 accepts and the property test reads the same way.
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::boundary::clean_text;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    let max = 50 + (data.len() as u32 % 400);
    let once = clean_text(text, max);
    assert!(once.text.chars().count() <= max as usize);
    assert!(!once.text.starts_with(' ') && !once.text.ends_with(' '));
    if !text.contains('&') {
        let twice = clean_text(&once.text, max);
        assert_eq!(twice.text, once.text);
        assert_eq!(twice.removed, 0);
    }
});
