//! URL normalization (RFC 008): the returned URL, changed as little as
//! possible (REQ-RANK-003), and the comparison key, normalized
//! aggressively and never returned (REQ-RANK-002).

use url::Url;

/// Query parameters removed from returned URLs: tracking only, never
/// content. Reviewable.
pub const TRACKING_PARAMETERS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "fbclid",
    "gclid",
    "dclid",
    "msclkid",
    "mc_cid",
    "mc_eid",
    "igshid",
    "yclid",
    "_hsenc",
    "_hsmi",
    "ref_src",
];

/// Why a provider's URL is dropped (REQ-CTX-006; RFC 008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlRejection {
    /// Not `http` or `https`, carries credentials, or does not parse.
    Invalid,
    /// Longer than LIM-F9 after normalization.
    TooLong,
}

/// The returned form: parsed, tracking parameters removed, host in ASCII
/// (IDNA) form; case, path, remaining query, fragment and explicit
/// non-default port kept. The `url` crate also lowercases the scheme and
/// host and drops a default port, which does not change the resource.
pub fn returned_url(raw: &str, max_chars: u32) -> Result<String, UrlRejection> {
    let mut url = Url::parse(raw.trim()).map_err(|_| UrlRejection::Invalid)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(UrlRejection::Invalid);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(UrlRejection::Invalid);
    }
    strip_tracking(&mut url);
    let text = url.to_string();
    if text.chars().count() > max_chars as usize {
        return Err(UrlRejection::TooLong);
    }
    Ok(text)
}

fn strip_tracking(url: &mut Url) {
    let Some(query) = url.query() else { return };
    let kept: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
        .filter(|(name, _)| !TRACKING_PARAMETERS.contains(&name.to_lowercase().as_str()))
        .map(|(n, v)| (n.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        url.set_query(None);
    } else {
        let rebuilt = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(kept)
            .finish();
        url.set_query(Some(&rebuilt));
    }
}

/// The comparison key of a returned URL: equal keys mean one resource. It
/// is a value for equality only; it is never returned and never fetched.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComparisonKey(String);

impl ComparisonKey {
    /// The key as text, for tests and logs of counts only.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Derives the comparison key: `https`, lowercase ASCII host without a
/// leading `www.`, no default port, normalized percent-encoding, no
/// trailing slash except the root, query parameters sorted, no fragment.
pub fn comparison_key(returned: &str) -> ComparisonKey {
    let Ok(url) = Url::parse(returned) else {
        return ComparisonKey(returned.to_owned());
    };
    let host = url.host_str().unwrap_or_default().to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    let port = match url.port() {
        Some(p) if p != 80 && p != 443 => format!(":{p}"),
        _ => String::new(),
    };
    let mut path = normalize_percent(url.path());
    if path.len() > 1 && path.ends_with('/') {
        path.pop();
    }
    let mut pairs: Vec<(String, String)> = url
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .map(|(n, v)| (n.into_owned(), v.into_owned()))
                .collect()
        })
        .unwrap_or_default();
    pairs.sort();
    let query = if pairs.is_empty() {
        String::new()
    } else {
        format!(
            "?{}",
            url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(pairs)
                .finish()
        )
    };
    ComparisonKey(format!("https://{host}{port}{path}{query}"))
}

/// Decodes percent-encoded unreserved characters and uppercases the hex
/// of the rest.
fn normalize_percent(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = String::with_capacity(path.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes.get(i + 1)), hex(bytes.get(i + 2))) {
                let value = h * 16 + l;
                if value.is_ascii_alphanumeric() || matches!(value, b'-' | b'.' | b'_' | b'~') {
                    out.push(value as char);
                } else {
                    out.push_str(&format!("%{value:02X}"));
                }
                i += 3;
                continue;
            }
        }
        let c = path[i..].chars().next().expect("in bounds");
        out.push(c);
        i += c.len_utf8();
    }
    out
}

fn hex(byte: Option<&u8>) -> Option<u8> {
    (*byte? as char).to_digit(16).map(|d| d as u8)
}

#[cfg(test)]
mod tests;
