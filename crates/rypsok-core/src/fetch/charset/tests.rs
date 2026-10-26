//! Tests of charset handling (REQ-PAGE-007). Every test names the
//! requirement it verifies.

use super::*;

/// REQ-PAGE-007: the declaration order is byte-order mark, header, meta,
/// detection, UTF-8; each source is reported.
#[test]
fn req_page_007_declaration_order_and_sources() {
    let bom = [0xEF, 0xBB, 0xBF, b'h', b'i'];
    let d = decode(&bom, Some("shift_jis"));
    assert_eq!(
        (d.encoding, d.source, d.text.as_str()),
        ("UTF-8", Source::Bom, "hi")
    );

    let sjis = [0x82, 0xB1, 0x82, 0xF1, 0x82, 0xC9, 0x82, 0xBF, 0x82, 0xCD]; // こんにちは
    let d = decode(&sjis, Some("Shift_JIS"));
    assert_eq!(
        (d.encoding, d.source, d.text.as_str()),
        ("Shift_JIS", Source::Header, "こんにちは")
    );

    let mut meta = b"<html><head><meta charset=\"shift_jis\"></head><body>".to_vec();
    meta.extend_from_slice(&sjis);
    let d = decode(&meta, None);
    assert_eq!((d.encoding, d.source), ("Shift_JIS", Source::Meta));
    assert!(d.text.contains("こんにちは"));

    let mut equiv =
        b"<meta http-equiv='Content-Type' content='text/html; charset=EUC-JP'>".to_vec();
    equiv.extend_from_slice(&[0xA4, 0xB3, 0xA4, 0xF3]); // こん
    let d = decode(&equiv, None);
    assert_eq!((d.encoding, d.source), ("EUC-JP", Source::Meta));
    assert!(d.text.contains("こん"));

    let d = decode("plain ascii and ünïcode".as_bytes(), None);
    assert_eq!((d.encoding, d.source), ("UTF-8", Source::Default));
    assert!(!d.had_errors);
}

/// REQ-PAGE-007: a legacy Japanese document without any declaration is
/// detected; malformed bytes are replaced, never fatal.
#[test]
fn req_page_007_detection_and_replacement() {
    // Shift_JIS text long enough for detection: 日本語のテキストです。
    let sjis: Vec<u8> = vec![
        0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA, 0x82, 0xCC, 0x83, 0x65, 0x83, 0x4C, 0x83, 0x58, 0x83,
        0x67, 0x82, 0xC5, 0x82, 0xB7, 0x81, 0x42,
    ];
    let mut body = b"<p>".to_vec();
    for _ in 0..8 {
        body.extend_from_slice(&sjis);
    }
    let d = decode(&body, None);
    assert_eq!(d.source, Source::Detected);
    assert!(d.text.contains("日本語のテキストです。"), "{}", d.text);

    let broken = [b'a', 0xFF, 0xFE, b'b'];
    let d = decode(&broken, Some("utf-8"));
    assert!(d.had_errors);
    assert_eq!(d.text, "a\u{FFFD}\u{FFFD}b");
}

/// REQ-PAGE-005: the charset parameter of a `Content-Type` value is read
/// case-insensitively and unquoted.
#[test]
fn req_page_005_charset_parameter_is_read() {
    assert_eq!(charset_parameter("text/html; charset=UTF-8"), Some("UTF-8"));
    assert_eq!(
        charset_parameter("text/html;Charset=\"iso-8859-1\""),
        Some("iso-8859-1")
    );
    assert_eq!(charset_parameter("text/html"), None);
    assert_eq!(charset_parameter("text/plain; format=flowed"), None);
}
