//! Charset decoding on any bytes with any declaration (RFC 012).
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::html::charset::{charset_parameter, decode};

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else { return };
    let declared = match first % 6 {
        0 => None,
        1 => Some("utf-8"),
        2 => Some("shift_jis"),
        3 => Some("euc-jp"),
        4 => Some("iso-8859-1"),
        _ => std::str::from_utf8(&rest[..rest.len().min(16)]).ok(),
    };
    let decoded = decode(rest, declared);
    assert!(decoded.text.len() <= rest.len() * 4 + 4);
    let _ = charset_parameter(&String::from_utf8_lossy(rest));
});
