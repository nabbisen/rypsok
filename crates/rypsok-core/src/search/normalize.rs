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
/// non-default port kept. The query is the engine's, byte for byte, unless a
/// tracking parameter is removed; then the other parameters keep their own
/// text (RFC 020, DAT-8). The `url` crate also lowercases the scheme and
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

/// Whether a query segment (the text between two `&`) is a tracking
/// parameter: its name, the text before `=`, percent-decoded and lower-cased.
fn is_tracking(segment: &str) -> bool {
    url::form_urlencoded::parse(segment.as_bytes())
        .next()
        .is_some_and(|(name, _)| TRACKING_PARAMETERS.contains(&name.to_lowercase().as_str()))
}

/// Removes tracking parameters. The query is rebuilt **only** when one was
/// removed, from the original text of the segments that stay, joined by `&`;
/// a query without a tracking parameter is left exactly as it is (`?a=1&b`,
/// `?a=1;b=2`, `?q=a%20b`, `?a=%7e`, `?x=%zz`, `?a=1&&b=2`).
fn strip_tracking(url: &mut Url) {
    let Some(query) = url.query() else { return };
    if !query.split('&').any(is_tracking) {
        return;
    }
    let kept: Vec<&str> = query.split('&').filter(|s| !is_tracking(s)).collect();
    let rebuilt = kept.join("&");
    if rebuilt.is_empty() {
        url.set_query(None);
    } else {
        url.set_query(Some(&rebuilt));
    }
}

/// The comparison key of a returned URL: equal keys mean one resource. It
/// is a value for equality only; it is never returned and never fetched.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComparisonKey(String);

impl ComparisonKey {
    /// The key without its fragment: the key of the fetch target. The
    /// fragment is never sent to a server, so a URL and the same URL with any
    /// fragment are one fetch (strict mode remembers this form, RFC 020 N1).
    /// The fragment is the last part of the key and the first `#` starts it:
    /// a `#` in a path or a query is written `%23` in the key.
    pub fn without_fragment(&self) -> ComparisonKey {
        match self.0.split_once('#') {
            Some((head, _)) => ComparisonKey(head.to_owned()),
            None => self.clone(),
        }
    }

    /// The key as text, for tests only.
    #[cfg(test)]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Derives the comparison key (RFC 008, amended by RFC 020 D-8): `https`
/// (`http` and `https` are one resource), lowercase ASCII host without a
/// leading `www.`, the port when it is not the default of the URL's own
/// scheme (so `http://x:443/p` and `https://x:80/p` are not `https://x/p`),
/// normalized percent-encoding, no trailing slash except the root, the
/// query parameters sorted by name only and stably (the order of repeated
/// parameters is part of the identity), and a non-empty fragment kept.
pub fn comparison_key(returned: &str) -> ComparisonKey {
    let Ok(url) = Url::parse(returned) else {
        return ComparisonKey(returned.to_owned());
    };
    let host = url.host_str().unwrap_or_default().to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    // `Url::port` is already none for the default of the URL's own scheme.
    let port = url.port().map_or_else(String::new, |p| format!(":{p}"));
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
    // By name only, stably: the relative order of repeated parameters stays.
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
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
    let fragment = url
        .fragment()
        .filter(|f| !f.is_empty())
        .map_or_else(String::new, |f| format!("#{f}"));
    ComparisonKey(format!("https://{host}{port}{path}{query}{fragment}"))
}

/// Decodes percent-encoded unreserved characters and uppercases the hex
/// of the rest.
fn normalize_percent(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = String::with_capacity(path.len());
    let mut i = 0;
    while i < bytes.len() {
        let encoded = if bytes[i] == b'%' && i + 2 < bytes.len() {
            hex(bytes.get(i + 1)).zip(hex(bytes.get(i + 2)))
        } else {
            None
        };
        if let Some((h, l)) = encoded {
            let value = h * 16 + l;
            if value.is_ascii_alphanumeric() || matches!(value, b'-' | b'.' | b'_' | b'~') {
                out.push(value as char);
            } else {
                out.push_str(&format!("%{value:02X}"));
            }
            i += 3;
            continue;
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
