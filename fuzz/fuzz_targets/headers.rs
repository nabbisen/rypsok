//! The header values read from a provider or a site: `Retry-After` (the
//! engine reader and the fetch reader) and the charset parameter of a
//! `Content-Type` (RFC 012; RFC 020, TST-14). None panics; a parsed
//! `Retry-After` is within its bound; a charset is a part of the value.
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::engine::drivers::parse_retry_after;
use rypsok_core::fetch::retry_after_ms;
use rypsok_core::html::charset::charset_parameter;

fuzz_target!(|data: &[u8]| {
    let Some((&which, rest)) = data.split_first() else { return };
    let Ok(text) = std::str::from_utf8(rest) else { return };
    match which % 3 {
        0 => {
            // A header value that reached the reader is visible ASCII.
            if text.bytes().all(|b| b == b'\t' || (32..127).contains(&b)) {
                if let Some(wait) = parse_retry_after(text) {
                    assert!((1..=86_400).contains(&wait.as_secs()), "{wait:?}");
                }
            }
        }
        1 => {
            if let Some(ms) = retry_after_ms(text) {
                assert!(ms % 1000 == 0 && ms <= 3_600_000, "{ms}");
                assert!(text.trim().bytes().all(|b| b.is_ascii_digit()), "{text:?}");
            }
        }
        _ => {
            if let Some(charset) = charset_parameter(text) {
                assert!(text.contains(charset), "{charset:?} is not part of {text:?}");
            }
        }
    }
});
