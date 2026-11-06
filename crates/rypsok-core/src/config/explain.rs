//! The words for a configuration that does not parse (RFC 020, DAT-11).
//!
//! `toml` reports a wrong type in the words of the Rust type it was
//! reading into (`expected u8`) and a place as a byte offset. The person who
//! wrote the file knows neither. This module turns the error and the text
//! into `<key path>: expected <what>, found <what> (line N, column M)`: the
//! key path is found by walking the parsed document to the span of the
//! error, the expectation comes from the range table that `validate` uses
//! too, and a string that was found is never repeated (a value may be a
//! secret put in the wrong key). It uses the `toml` crate's own spanned
//! document (`DeTable`); no dependency was added.

use toml::de::{DeTable, DeValue};

use super::integer_range;
use crate::model::Language;

/// Where in the document an offset lies.
struct Located {
    /// The key path, such as `search.languages[0]`.
    path: String,
}

/// The nearest character boundary at or before `offset` (and not past the
/// end).
fn floor_boundary(text: &str, offset: usize) -> usize {
    let mut at = offset.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// The line and the column (both from 1, in characters) of a byte offset.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let before = &text[..floor_boundary(text, offset)];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, before[line_start..].chars().count() + 1)
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}

/// The innermost key or value whose span holds `offset`, so far.
struct Best {
    located: Located,
    width: usize,
}

fn offer(
    best: &mut Option<Best>,
    path: String,
    in_key: bool,
    span: std::ops::Range<usize>,
    offset: usize,
) {
    // A zero-width span (a missing value) holds the offset it starts at.
    if span.start <= offset && offset <= span.end {
        let width = span.end - span.start;
        if best
            .as_ref()
            .is_none_or(|b| width < b.width || (width == b.width && in_key))
        {
            *best = Some(Best {
                located: Located { path },
                width,
            });
        }
    }
}

/// Walks the whole document: the parser's spans for a table do not
/// necessarily hold the spans of what is inside it, so every entry is
/// visited and the narrowest span that holds the offset wins.
fn walk(table: &DeTable<'_>, offset: usize, prefix: &str, best: &mut Option<Best>) {
    for (key, value) in table.iter() {
        let path = join(prefix, key.get_ref());
        offer(best, path.clone(), true, key.span(), offset);
        offer(best, path.clone(), false, value.span(), offset);
        inside(value.get_ref(), offset, &path, best);
    }
}

fn inside(value: &DeValue<'_>, offset: usize, path: &str, best: &mut Option<Best>) {
    match value {
        DeValue::Table(table) => walk(table, offset, path, best),
        DeValue::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let element = format!("{path}[{index}]");
                offer(best, element.clone(), false, item.span(), offset);
                inside(item.get_ref(), offset, &element, best);
            }
        }
        _ => {}
    }
}

/// The path of the table whose value span is exactly `span`: the table an
/// error of a missing field is reported on.
fn locate_table(
    table: &DeTable<'_>,
    span: &std::ops::Range<usize>,
    prefix: &str,
) -> Option<String> {
    for (key, value) in table.iter() {
        let path = join(prefix, key.get_ref());
        if value.span() == *span {
            return Some(path);
        }
        let deeper = match value.get_ref() {
            DeValue::Table(inner) => locate_table(inner, span, &path),
            DeValue::Array(items) => items.iter().enumerate().find_map(|(index, item)| {
                let element = format!("{path}[{index}]");
                if item.span() == *span {
                    return Some(element);
                }
                match item.get_ref() {
                    DeValue::Table(inner) => locate_table(inner, span, &element),
                    _ => None,
                }
            }),
            _ => None,
        };
        if deeper.is_some() {
            return deeper;
        }
    }
    None
}

fn locate(table: &DeTable<'_>, offset: usize) -> Option<Located> {
    let mut best = None;
    walk(table, offset, "", &mut best);
    best.map(|b| b.located)
}

/// The key path of a syntax error, read from the lines of the text: the
/// table header the offset is in, or the latest header above it, and the
/// key at the offset when it is a plain word. The parser gives no document
/// for a duplicate, so the text is read instead.
fn path_from_text(text: &str, offset: usize, span_end: usize) -> Option<String> {
    let offset = floor_boundary(text, offset);
    let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[offset..].find('\n').map_or(text.len(), |i| offset + i);
    let line = text[line_start..line_end].trim();
    let header = |line: &str| -> Option<String> {
        let inner = line
            .strip_prefix("[[")
            .and_then(|l| l.split("]]").next())
            .or_else(|| line.strip_prefix('[').and_then(|l| l.split(']').next()))?;
        Some(
            inner
                .split('.')
                .map(|part| part.trim().trim_matches('"'))
                .collect::<Vec<_>>()
                .join("."),
        )
    };
    if line.starts_with('[') {
        return header(line);
    }
    let table = text[..line_start]
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| l.starts_with('['))
        .and_then(header)
        .unwrap_or_default();
    let key = text.get(offset..span_end.max(offset))?.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some(join(&table, key))
}

/// A list of the words between backticks in a serde message.
fn backticked(message: &str) -> Vec<&str> {
    message.split('`').skip(1).step_by(2).collect()
}

/// What was found, in a few words. A string is never repeated.
fn found(what: &str) -> String {
    let what = what.split(" as ").next().unwrap_or(what);
    if what.starts_with("string") {
        "text".to_owned()
    } else if let Some(rest) = what
        .strip_prefix("integer `")
        .or_else(|| what.strip_prefix("floating point `"))
        .or_else(|| what.strip_prefix("boolean `"))
    {
        rest.trim_end_matches('`').chars().take(40).collect()
    } else if what.starts_with("sequence") {
        "a list".to_owned()
    } else if what.starts_with("map") {
        "a table".to_owned()
    } else {
        "a value of another kind".to_owned()
    }
}

/// The key path with its list indexes and engine identifiers made general,
/// to look it up in a table of what a key holds.
fn general(path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        out.push_str("[]");
        match rest[open..].find(']') {
            Some(close) => rest = &rest[open + close + 1..],
            None => return out,
        }
    }
    out.push_str(rest);
    match out.strip_prefix("engines.") {
        Some(after) => match after.split_once('.') {
            Some((_, key)) => format!("engines.*.{key}"),
            None => out,
        },
        None => out,
    }
}

/// What a key was expected to hold, for a message.
fn expected(what: &str, path: &str) -> String {
    let key = general(path);
    match what {
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "i128" => {
            if key == "config_version" {
                return "the number 1".to_owned();
            }
            let (low, high) = integer_range(&key).unwrap_or(match what {
                "u8" => (0, u64::from(u8::MAX)),
                "u16" => (0, u64::from(u16::MAX)),
                "u32" => (0, u64::from(u32::MAX)),
                _ => (0, u64::MAX),
            });
            format!("an integer from {low} to {high}")
        }
        "a boolean" => "true or false".to_owned(),
        "a string" | "a borrowed string" | "string" => "text".to_owned(),
        "a sequence" => match key.as_str() {
            "fetch.ports" => "a list of port numbers, such as [80, 443]".to_owned(),
            "egress.allow" | "egress.deny" => {
                "a list of text rules, such as [\"10.0.0.0/8\"]".to_owned()
            }
            "search.languages" => "a list of language tags, such as [\"en\", \"ja\"]".to_owned(),
            "engines.*.categories" => "a list of categories".to_owned(),
            _ => "a list of text values".to_owned(),
        },
        other if other.starts_with("struct ") || other == "a map" => "a table".to_owned(),
        _ => "a different kind of value".to_owned(),
    }
}

/// The first element of the list that starts at `start` that is not a
/// language tag: its key path and its offset. A custom error of an element
/// is reported on the whole list; this finds the element.
fn bad_language(table: &DeTable<'_>, start: usize, prefix: &str) -> Option<(String, usize)> {
    for (key, value) in table.iter() {
        let path = join(prefix, key.get_ref());
        match value.get_ref() {
            DeValue::Array(items) if value.span().start == start => {
                for (index, item) in items.iter().enumerate() {
                    let bad = item
                        .get_ref()
                        .as_str()
                        .is_none_or(|s| Language::new(s).is_err());
                    if bad {
                        return Some((format!("{path}[{index}]"), item.span().start));
                    }
                }
            }
            DeValue::Table(inner) => {
                if let Some(found) = bad_language(inner, start, &path) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// The message for an error of `toml` on `text`.
pub(super) fn explain(text: &str, error: &toml::de::Error) -> String {
    let message = error.message();
    let span = error.span().unwrap_or(0..0);
    let (document, syntax) = match DeTable::parse(text) {
        Ok(document) => (document, false),
        Err(_) => (DeTable::parse_recoverable(text).0, true),
    };
    let located = locate(document.get_ref(), span.start);
    // A language tag is checked by its own type, whose error is reported
    // on the whole list: the element is the place.
    let language = message
        .starts_with("a language is")
        .then(|| bad_language(document.get_ref(), span.start, ""))
        .flatten();
    let offset = language.as_ref().map_or(span.start, |(_, start)| *start);
    let (line, column) = position(text, offset);
    let at = format!("(line {line}, column {column})");

    if syntax {
        if message == "duplicate key" {
            return match path_from_text(text, span.start, span.end) {
                Some(path) => format!("{path}: defined more than once {at}"),
                None => format!("a key is defined more than once {at}"),
            };
        }
        return format!("TOML syntax error: {message} {at}");
    }
    // A missing field is reported on the table that lacks it: that table
    // is found by its exact span (the root has none).
    let path = if message.starts_with("missing field") {
        locate_table(document.get_ref(), &span, "").unwrap_or_default()
    } else {
        language
            .as_ref()
            .map(|(path, _)| path.clone())
            .or_else(|| located.map(|l| l.path))
            .unwrap_or_default()
    };
    let named = |what: String| -> String {
        if path.is_empty() {
            format!("{what} {at}")
        } else {
            format!("{path}: {what} {at}")
        }
    };
    if message.starts_with("unknown field") {
        let list = backticked(message)
            .into_iter()
            .skip(1)
            .collect::<Vec<_>>()
            .join(", ");
        return named(format!("unknown key; expected one of {list}"));
    }
    if message.starts_with("unknown variant") {
        // The word is not repeated: it may be a secret put in the wrong
        // key, and the message reaches the log (L1 of the 020l review).
        let list = backticked(message)
            .into_iter()
            .skip(1)
            .collect::<Vec<_>>()
            .join(", ");
        return named(format!("expected one of {list}, found an unknown word"));
    }
    if let Some(field) = message.strip_prefix("missing field `") {
        let field = field.trim_end_matches('`');
        if field == "config_version" && path.is_empty() {
            return "config_version: required; this release supports 1".to_owned();
        }
        return if path.is_empty() {
            format!("{field}: required {at}")
        } else {
            format!("{path}.{field}: required {at}")
        };
    }
    if message.starts_with("a language is") {
        return named(
            "expected a language tag (two or three lowercase letters, optionally a hyphen and a two-letter uppercase region, such as en or pt-BR), found text"
                .to_owned(),
        );
    }
    for lead in ["invalid type: ", "invalid value: "] {
        if let Some(rest) = message.strip_prefix(lead)
            && let Some((what, wanted)) = rest.rsplit_once(", expected ")
        {
            return named(format!(
                "expected {}, found {}",
                expected(wanted, &path),
                found(what)
            ));
        }
    }
    named(message.to_owned())
}

#[cfg(test)]
mod tests;
