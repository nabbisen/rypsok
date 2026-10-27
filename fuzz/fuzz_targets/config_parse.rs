//! Configuration parsing and validation on any text (RFC 004).
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::config::Config;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    if let Ok(config) = Config::parse(text) {
        let _ = config.validate();
        let _ = config.limits();
    }
});
