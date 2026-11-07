//! An `[egress].allow` or `deny` entry, any text (RFC 011; RFC 020,
//! TST-14). An accepted rule written back as text is read as the same rule.
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::egress::policy::parse_rule;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    if let Ok(rule) = parse_rule(text) {
        let shown = rule.to_string();
        assert_eq!(parse_rule(&shown), Ok(rule), "{text:?} shown as {shown:?}");
    }
});
