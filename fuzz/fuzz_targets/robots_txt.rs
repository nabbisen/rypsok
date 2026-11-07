//! The decision a robots.txt answer gives for a URL: the status logic and
//! the matcher (RFC 012, RFC 019 D2; RFC 020, TST-14). The same answer gives
//! the same decision twice, and a body longer than the read bound is judged
//! on its prefix only.
#![no_main]
use libfuzzer_sys::fuzz_target;
use rypsok_core::fetch::robots::decision_for_answer;
use url::Url;

const URLS: [&str; 4] = [
    "https://example.org/",
    "https://example.org/private/page",
    "https://example.org/a/b?c=d",
    "http://127.0.0.1:8080/x",
];
const TYPES: [Option<&str>; 4] = [None, Some("text/plain"), Some("text/html; charset=utf-8"), Some("application/json")];

fuzz_target!(|data: &[u8]| {
    if data.len() < 6 {
        return;
    }
    let status = 100 + u16::from_le_bytes([data[0], data[1]]) % 500;
    let content_type = TYPES[usize::from(data[2]) % TYPES.len()];
    let url = Url::parse(URLS[usize::from(data[3]) % URLS.len()]).unwrap();
    // A small bound, so that the cut is reached.
    let max = 16 + u64::from(data[4]) * 16;
    let body = &data[5..];

    let first = decision_for_answer(status, content_type, body, max, &url);
    assert_eq!(first, decision_for_answer(status, content_type, body, max, &url));
    // A body past the bound is judged on its prefix only.
    if body.len() as u64 > max {
        let prefix = &body[..max as usize];
        assert_eq!(first, decision_for_answer(status, content_type, prefix, max, &url));
    }
});
