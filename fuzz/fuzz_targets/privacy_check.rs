//! The secret check on any text in every mode (RFC 009).
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::config::SecretDetection;
use rypsok_core::privacy::{Verdict, check};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    for mode in [
        SecretDetection::Off,
        SecretDetection::Warn,
        SecretDetection::Mask,
        SecretDetection::Refuse,
    ] {
        match check(text, mode) {
            Verdict::Masked { text: masked, count } => {
                assert!(count > 0);
                assert!(masked.chars().count() <= text.chars().count() + 64 * count as usize);
            }
            Verdict::Refuse(finding) => assert!((finding.start as usize) <= text.chars().count()),
            Verdict::Clean | Verdict::Warned(_) => {}
        }
    }
});
