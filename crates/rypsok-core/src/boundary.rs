//! The content boundary: every piece of external text passes here before it
//! enters a response (external design §9; RFC 005).
//!
//! [`clean_text`] applies five steps in a fixed order: markup removal and
//! entity decoding, NFC normalization, removal of invisible and control
//! characters (REQ-SEC-015), whitespace collapsing, and bounding at a
//! character boundary (REQ-CTX-004, REQ-CTX-006). Visible text is never
//! rewritten beyond these steps (REQ-SEC-003). Every change is reported
//! through counts that become notices (REQ-CTX-007).
//!
//! [`make_result`] is the only way to build a [`SearchResult`]: the type
//! has a crate-private field, so no code outside this crate, and no code
//! inside it that bypasses the boundary, can construct one.
//!
//! ```compile_fail
//! // A result cannot be built outside the boundary (RFC 005).
//! let _ = rypsok_core::model::SearchResult {
//!     title: String::new(),
//!     url: String::new(),
//!     snippet: String::new(),
//!     date: None,
//!     engines: None,
//! };
//! ```

use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_normalization::UnicodeNormalization;

use crate::model::{Date, EngineId, SearchResult};

/// How many characters before the bound a word boundary is preferred.
const WORD_BOUNDARY_WINDOW: usize = 20;

/// Phrases that mark instruction-like content, matched case-insensitively
/// against cleaned text (REQ-SEC-003). This is a heuristic that produces a
/// notice, not a boundary: the structure of the response and the JSON
/// rendering are what keep external text from becoming an instruction.
/// Reviewable; grows with incidents.
pub const INSTRUCTION_PATTERNS: &[&str] = &[
    "ignore previous instructions",
    "ignore all previous",
    "disregard the above",
    "you are now",
    "system prompt",
    "do not tell the user",
];

/// The outcome of cleaning one text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cleaned {
    /// The cleaned, bounded text.
    pub text: String,
    /// Invisible and control characters removed in step 3.
    pub removed: u32,
    /// Whether step 5 cut the text.
    pub truncated: bool,
}

/// Cleans a title or snippet: the five steps of RFC 005 in order, bounded
/// at `max_chars` Unicode scalar values (LIM-U1).
pub fn clean_text(raw: &str, max_chars: u32) -> Cleaned {
    let without_markup = strip_tags(raw);
    let decoded = html_escape::decode_html_entities(&without_markup);
    let normalized: String = decoded.nfc().collect();
    let (visible, removed) = remove_invisible(&normalized);
    let collapsed = visible.split_whitespace().collect::<Vec<_>>().join(" ");
    let (text, truncated) = bound(&collapsed, max_chars as usize);
    Cleaned {
        text,
        removed,
        truncated,
    }
}

/// Whether the cleaned text contains an instruction-like phrase.
pub fn has_instruction_like_pattern(text: &str) -> bool {
    let lower = text.to_lowercase();
    INSTRUCTION_PATTERNS.iter().any(|p| lower.contains(p))
}

/// What building one result changed, for the notices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResultEffects {
    /// The title was cut to the bound.
    pub title_truncated: bool,
    /// The snippet was cut to the bound.
    pub snippet_truncated: bool,
    /// Invisible characters removed from title and snippet together.
    pub characters_removed: u32,
    /// Title or snippet matched an instruction-like phrase.
    pub instruction_like: bool,
}

/// Builds a result from raw external text. `url` must already be the
/// returned form of RFC 008; `date` and `engines` are rypsok's own.
pub fn make_result(
    raw_title: &str,
    raw_snippet: &str,
    url: String,
    date: Option<Date>,
    engines: Option<Vec<EngineId>>,
    title_max_chars: u32,
    snippet_max_chars: u32,
) -> (SearchResult, ResultEffects) {
    let title = clean_text(raw_title, title_max_chars);
    let snippet = clean_text(raw_snippet, snippet_max_chars);
    let effects = ResultEffects {
        title_truncated: title.truncated,
        snippet_truncated: snippet.truncated,
        characters_removed: title.removed + snippet.removed,
        instruction_like: has_instruction_like_pattern(&title.text)
            || has_instruction_like_pattern(&snippet.text),
    };
    let result = SearchResult {
        title: title.text,
        url,
        snippet: snippet.text,
        date,
        engines,
        cleaned: crate::model::Cleaned,
    };
    (result, effects)
}

/// Removes HTML tags, keeping their text. A `<` that does not start a tag
/// (not followed by a letter, `/`, `!` or `?`) is kept. An unterminated
/// tag swallows the rest, which is the safe side for a markup fragment.
fn strip_tags(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '<' {
            let starts_tag = raw[i + 1..]
                .chars()
                .next()
                .is_some_and(|n| n.is_ascii_alphabetic() || matches!(n, '/' | '!' | '?'));
            if starts_tag {
                let mut closed = false;
                for (_, n) in chars.by_ref() {
                    if n == '>' {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    break;
                }
                // A tag separates words: `<br>` or `</p>` between texts.
                out.push(' ');
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// Whether a character is removed by step 3 (REQ-SEC-015).
fn is_removed(c: char) -> bool {
    if matches!(c, '\t' | '\n' | '\r' | '\u{0B}' | '\u{0C}') {
        return false;
    }
    if ('\u{E0000}'..='\u{E007F}').contains(&c) || c == '\u{00AD}' {
        return true;
    }
    matches!(
        get_general_category(c),
        GeneralCategory::Control | GeneralCategory::Format | GeneralCategory::Unassigned
    )
}

fn remove_invisible(text: &str) -> (String, u32) {
    let mut out = String::with_capacity(text.len());
    let mut removed = 0u32;
    for c in text.chars() {
        if is_removed(c) {
            removed += 1;
        } else {
            out.push(c);
        }
    }
    (out, removed)
}

/// Cuts to `max_chars` scalar values, preferring the last word boundary
/// within the final [`WORD_BOUNDARY_WINDOW`] characters.
fn bound(text: &str, max_chars: usize) -> (String, bool) {
    let total = text.chars().count();
    if total <= max_chars {
        return (text.to_owned(), false);
    }
    let cut_at = text
        .char_indices()
        .nth(max_chars)
        .map_or(text.len(), |(i, _)| i);
    let head = &text[..cut_at];
    let window_start = head
        .char_indices()
        .nth(max_chars.saturating_sub(WORD_BOUNDARY_WINDOW))
        .map_or(0, |(i, _)| i);
    let cut = match head[window_start..].rfind(' ') {
        Some(offset) => window_start + offset,
        None => cut_at,
    };
    (head[..cut].trim_end().to_owned(), true)
}

#[cfg(test)]
mod tests;
