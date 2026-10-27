//! The content cleaner's window never panics and stays within `max_chars` (RFC 012).
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::boundary::clean_content;

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else { return };
    let Ok(text) = std::str::from_utf8(rest) else { return };
    let max = 500 + u32::from(first) * 10;
    let start = u32::from(first) * 3;
    if let Ok(window) = clean_content(text, start, max) {
        assert!(window.text.chars().count() <= max as usize);
        if let Some(next) = window.next_start_char {
            assert!(next > start);
            assert!(window.truncated);
        }
    }
});
