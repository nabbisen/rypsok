//! Charset handling (RFC 012; REQ-PAGE-007): the byte-order mark, then the
//! `Content-Type` charset, then a `<meta>` declaration in the first 1024
//! bytes, then detection, then UTF-8; malformed sequences are replaced,
//! never fatal.

use encoding_rs::Encoding;

/// How far a `<meta>` declaration is looked for, as the HTML standard's
/// prescan does.
const PRESCAN_BYTES: usize = 1024;

/// What decided the encoding, for the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A byte-order mark.
    Bom,
    /// The `Content-Type` header's charset parameter.
    Header,
    /// A `<meta charset>` or `http-equiv` declaration.
    Meta,
    /// Statistical detection.
    Detected,
    /// Nothing said; UTF-8 assumed.
    Default,
}

/// A decoded document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The text, with replacement characters where bytes were malformed.
    pub text: String,
    /// The encoding's name as the WHATWG Encoding Standard spells it.
    pub encoding: &'static str,
    /// What decided it.
    pub source: Source,
    /// Whether any sequence was replaced.
    pub had_errors: bool,
}

/// Decodes a body. `declared` is the charset parameter of the
/// `Content-Type` header, when present.
pub fn decode(bytes: &[u8], declared: Option<&str>) -> Decoded {
    let (encoding, source) = choose(bytes, declared);
    let (text, had_errors) = encoding.decode_with_bom_removal(bytes);
    Decoded {
        text: text.into_owned(),
        encoding: encoding.name(),
        source,
        had_errors,
    }
}

fn choose(bytes: &[u8], declared: Option<&str>) -> (&'static Encoding, Source) {
    if let Some((encoding, _)) = Encoding::for_bom(bytes) {
        return (encoding, Source::Bom);
    }
    if let Some(encoding) = declared.and_then(|label| Encoding::for_label(label.trim().as_bytes()))
    {
        return (encoding, Source::Header);
    }
    if let Some(encoding) = meta_charset(&bytes[..bytes.len().min(PRESCAN_BYTES)]) {
        return (encoding, Source::Meta);
    }
    let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    detector.feed(bytes, true);
    let guess = detector.guess(None, chardetng::Utf8Detection::Allow);
    if guess == encoding_rs::UTF_8 {
        (encoding_rs::UTF_8, Source::Default)
    } else {
        (guess, Source::Detected)
    }
}

/// A `charset=` inside a `<meta` tag of the prescan window, case-insensitive,
/// either `<meta charset="x">` or `<meta http-equiv="Content-Type"
/// content="text/html; charset=x">`.
fn meta_charset(head: &[u8]) -> Option<&'static Encoding> {
    let lower: Vec<u8> = head.iter().map(u8::to_ascii_lowercase).collect();
    let mut from = 0;
    while let Some(at) = find(&lower[from..], b"<meta") {
        let start = from + at;
        let end = find(&lower[start..], b">").map_or(lower.len(), |e| start + e);
        let tag = &lower[start..end];
        if let Some(c) = find(tag, b"charset=") {
            let value = &tag[c + "charset=".len()..];
            let value = value
                .strip_prefix(b"\"")
                .or_else(|| value.strip_prefix(b"'"))
                .unwrap_or(value);
            let stop = value
                .iter()
                .position(|b| matches!(b, b'"' | b'\'' | b';' | b' ' | b'/' | b'>'))
                .unwrap_or(value.len());
            if let Some(encoding) = Encoding::for_label(&value[..stop]) {
                return Some(encoding);
            }
        }
        from = end;
    }
    None
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// The charset parameter of a `Content-Type` value, when present.
pub fn charset_parameter(content_type: &str) -> Option<&str> {
    content_type
        .split(';')
        .skip(1)
        .map(str::trim)
        .find_map(|p| {
            let (key, value) = p.split_once('=')?;
            key.trim()
                .eq_ignore_ascii_case("charset")
                .then(|| value.trim().trim_matches('"'))
        })
}

#[cfg(test)]
mod tests;
