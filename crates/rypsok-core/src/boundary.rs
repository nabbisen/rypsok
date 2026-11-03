//! The content boundary: every piece of external text passes here before it
//! enters a response (external design §9; RFC 005).
//!
//! There are two cleaners for titles and snippets, chosen by where the text
//! comes from ([`Provenance`], RFC 019 D3).
//!
//! [`clean_markup_text`] is for raw provider text, which carries markup and
//! entities. It applies five steps in a fixed order: markup removal and
//! entity decoding, NFC normalization, removal of invisible and control
//! characters (REQ-SEC-015), whitespace collapsing, and bounding at a
//! character boundary (REQ-CTX-004, REQ-CTX-006).
//!
//! [`clean_decoded_text`] is for text a parser has already decoded (a
//! page's DOM): the same steps without the markup step and without entity
//! decoding, because a decoded `&lt;` is the four characters `&lt;` and a
//! decoded `<T>` is text. Handing decoded text to the markup cleaner would
//! silently eat `<T>`; handing raw text to the decoded cleaner would pass
//! markup, so a result is raw unless its producer says otherwise.
//!
//! Visible text is never rewritten beyond these steps (REQ-SEC-003). Every
//! change is reported through counts that become notices (REQ-CTX-007).
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

/// Where a piece of external text comes from, which decides its cleaner
/// (RFC 019 D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Provenance {
    /// Raw provider text: markup and entities are still in it. The default.
    #[default]
    Raw,
    /// Text a parser already decoded: no markup, no entities.
    Decoded,
}

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

/// For raw provider text: markup is stripped and entities are decoded; use
/// [`clean_decoded_text`] for text a parser already decoded.
///
/// Cleans a title or snippet: the five steps of RFC 005 in order, bounded
/// at `max_chars` Unicode scalar values (LIM-U1).
pub fn clean_markup_text(raw: &str, max_chars: u32) -> Cleaned {
    // NFC before the markup step as well as after entity decoding (RFC 005
    // amendment of 2026-10-27): normalization can map a compatibility
    // letter to ASCII or reorder combining marks after a `<`, and the tag
    // decision must see the form a second cleaning would see, so that
    // cleaning is idempotent (REQ-TEST-013).
    let normalized_raw: String = raw.nfc().collect();
    // Invisible and control characters go before the markup step too
    // (RFC 005 amendment of 2026-10-28, found by the fuzz target): a NUL
    // between `<` and a letter hid a tag from the first pass that the
    // second pass then stripped. A filter that reads `<\0y>` as text while
    // the next reader sees `<y>` is the classic bypass; both passes now
    // see the same characters.
    let (visible_raw, removed_before) = remove_invisible(&normalized_raw);
    let without_markup = strip_tags(&visible_raw);
    let decoded = html_escape::decode_html_entities(&without_markup);
    let normalized: String = decoded.nfc().collect();
    let (visible, removed_after) = remove_invisible(&normalized);
    let removed = removed_before + removed_after;
    let collapsed = visible.split_whitespace().collect::<Vec<_>>().join(" ");
    let (text, truncated) = bound(&collapsed, max_chars as usize);
    Cleaned {
        text,
        removed,
        truncated,
    }
}

/// For text a parser already decoded (a page's title, a scraped snippet):
/// there is no markup step and no entity decoding, so `<T>` and `&lt;` stay
/// as written; use [`clean_markup_text`] for raw provider text.
///
/// NFC normalization, removal of invisible and control characters
/// (REQ-SEC-015), whitespace collapsing and the bound, in the order of
/// [`clean_markup_text`]. Cleaning is idempotent: with no entity residual
/// there is nothing to exclude (RFC 019 D3).
pub fn clean_decoded_text(text: &str, max_chars: u32) -> Cleaned {
    let normalized: String = text.nfc().collect();
    let (visible, removed) = remove_invisible(&normalized);
    // Removing a character between a base and a combining mark can let
    // them compose; normalize again so that a second pass changes nothing.
    let recomposed: String = visible.nfc().collect();
    let collapsed = recomposed.split_whitespace().collect::<Vec<_>>().join(" ");
    let (text, truncated) = bound(&collapsed, max_chars as usize);
    Cleaned {
        text,
        removed,
        truncated,
    }
}

/// The outcome of cleaning fetched content (RFC 012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanedContent {
    /// The window of the cleaned content.
    pub text: String,
    /// Invisible and control characters removed from the whole content.
    pub removed: u32,
    /// Whether content remains after the window.
    pub truncated: bool,
    /// Where to continue when it was cut.
    pub next_start_char: Option<u32>,
    /// The length of the whole cleaned content, in scalar values.
    pub total_chars: u32,
}

/// Why a window could not be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartBeyondEnd {
    /// The length of the cleaned content.
    pub total_chars: u32,
}

/// Cleans rendered page content (RFC 012; REQ-CTX-005, REQ-SEC-015,
/// REQ-FETCH-003, REQ-FETCH-005): NFC, invisible characters removed and
/// counted, trailing whitespace of each line removed, three or more
/// newlines folded to two, then the window of `max_chars` scalar values
/// from `start_char` at a character boundary. Spaces inside a line are
/// kept, so code blocks keep their indentation.
pub fn clean_content(
    rendered: &str,
    start_char: u32,
    max_chars: u32,
) -> Result<CleanedContent, StartBeyondEnd> {
    let normalized: String = rendered.nfc().collect();
    let (visible, removed) = remove_invisible(&normalized);
    let mut folded = String::with_capacity(visible.len());
    let mut newlines = 0usize;
    for line in visible.split('\n') {
        let line = line.trim_end();
        if line.is_empty() {
            newlines += 1;
            continue;
        }
        if !folded.is_empty() {
            folded.push_str(if newlines > 0 { "\n\n" } else { "\n" });
        }
        folded.push_str(line);
        newlines = 0;
    }
    let total_chars = folded.chars().count();
    let start = start_char as usize;
    if start >= total_chars && !(start == 0 && total_chars == 0) {
        return Err(StartBeyondEnd {
            total_chars: total_chars as u32,
        });
    }
    let window: String = folded
        .chars()
        .skip(start)
        .take(max_chars as usize)
        .collect();
    let taken = window.chars().count();
    let truncated = start + taken < total_chars;
    Ok(CleanedContent {
        text: window,
        removed,
        truncated,
        next_start_char: truncated.then(|| (start + taken) as u32),
        total_chars: total_chars as u32,
    })
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

/// Builds a result from raw provider text (markup and entities still in
/// it): the cleaner is [`clean_markup_text`]. `url` must already be the
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
    build_result(
        clean_markup_text(raw_title, title_max_chars),
        clean_markup_text(raw_snippet, snippet_max_chars),
        url,
        date,
        engines,
    )
}

/// Builds a result from text a parser already decoded (a scraping driver's
/// results): the cleaner is [`clean_decoded_text`], so `Vec<T>` stays as
/// written. Otherwise as [`make_result`].
pub fn make_decoded_result(
    decoded_title: &str,
    decoded_snippet: &str,
    url: String,
    date: Option<Date>,
    engines: Option<Vec<EngineId>>,
    title_max_chars: u32,
    snippet_max_chars: u32,
) -> (SearchResult, ResultEffects) {
    build_result(
        clean_decoded_text(decoded_title, title_max_chars),
        clean_decoded_text(decoded_snippet, snippet_max_chars),
        url,
        date,
        engines,
    )
}

/// The one place a [`SearchResult`] is built, from cleaned text.
fn build_result(
    title: Cleaned,
    snippet: Cleaned,
    url: String,
    date: Option<Date>,
    engines: Option<Vec<EngineId>>,
) -> (SearchResult, ResultEffects) {
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
            // The HTML tokenizer's tag-open state: an ASCII letter starts
            // a tag name; the input is NFC already, so a compatibility
            // letter has become ASCII and a combining mark cannot pass.
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

/// Zero width non-joiner and zero width joiner. They are `Cf` and are
/// decided in context, not by their category (RFC 020 D-10).
const ZWNJ: char = '\u{200C}';
const ZWJ: char = '\u{200D}';

fn is_joiner(c: char) -> bool {
    c == ZWNJ || c == ZWJ
}

/// Whether the secret check should read past a character: one that step 3
/// removes or may remove (a joiner too).
pub(crate) fn is_invisible(c: char) -> bool {
    is_joiner(c) || is_removed(c)
}

/// The scripts in which a joiner carries orthography.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JoinerScript {
    Arabic,
    Indic,
}

/// Arabic: Arabic, Arabic Supplement, Arabic Extended-A, Presentation
/// Forms-A and -B. Indic: Devanagari to Sinhala (U+0900 to U+0DFF, which
/// takes in Bengali, Gurmukhi, Gujarati, Oriya, Tamil, Telugu, Kannada and
/// Malayalam). A small private table, not a dependency.
fn joiner_script(c: char) -> Option<JoinerScript> {
    match c {
        '\u{0600}'..='\u{06FF}'
        | '\u{0750}'..='\u{077F}'
        | '\u{08A0}'..='\u{08FF}'
        | '\u{FB50}'..='\u{FDFF}'
        | '\u{FE70}'..='\u{FEFF}' => Some(JoinerScript::Arabic),
        '\u{0900}'..='\u{0DFF}' => Some(JoinerScript::Indic),
        _ => None,
    }
}

/// The viramas (halants) of the scripts in U+0900 to U+0DFF.
fn is_virama(c: char) -> bool {
    matches!(
        c,
        '\u{094D}'
            | '\u{09CD}'
            | '\u{0A4D}'
            | '\u{0ACD}'
            | '\u{0B4D}'
            | '\u{0BCD}'
            | '\u{0C4D}'
            | '\u{0CCD}'
            | '\u{0D4D}'
            | '\u{0DCA}'
    )
}

/// `Extended_Pictographic` of the Unicode emoji data (emoji-data.txt),
/// as ranges. Over-inclusive for unassigned code points inside a range,
/// which step 3 removes anyway. A test compares it with the property of the
/// `regex` crate for every scalar value.
const EXTENDED_PICTOGRAPHIC: &[(u32, u32)] = &[
    (0x00A9, 0x00A9),
    (0x00AE, 0x00AE),
    (0x203C, 0x203C),
    (0x2049, 0x2049),
    (0x2122, 0x2122),
    (0x2139, 0x2139),
    (0x2194, 0x2199),
    (0x21A9, 0x21AA),
    (0x231A, 0x231B),
    (0x2328, 0x2328),
    (0x2388, 0x2388),
    (0x23CF, 0x23CF),
    (0x23E9, 0x23F3),
    (0x23F8, 0x23FA),
    (0x24C2, 0x24C2),
    (0x25AA, 0x25AB),
    (0x25B6, 0x25B6),
    (0x25C0, 0x25C0),
    (0x25FB, 0x25FE),
    (0x2600, 0x2605),
    (0x2607, 0x2612),
    (0x2614, 0x2685),
    (0x2690, 0x2705),
    (0x2708, 0x2712),
    (0x2714, 0x2714),
    (0x2716, 0x2716),
    (0x271D, 0x271D),
    (0x2721, 0x2721),
    (0x2728, 0x2728),
    (0x2733, 0x2734),
    (0x2744, 0x2744),
    (0x2747, 0x2747),
    (0x274C, 0x274C),
    (0x274E, 0x274E),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2763, 0x2767),
    (0x2795, 0x2797),
    (0x27A1, 0x27A1),
    (0x27B0, 0x27B0),
    (0x27BF, 0x27BF),
    (0x2934, 0x2935),
    (0x2B05, 0x2B07),
    (0x2B1B, 0x2B1C),
    (0x2B50, 0x2B50),
    (0x2B55, 0x2B55),
    (0x3030, 0x3030),
    (0x303D, 0x303D),
    (0x3297, 0x3297),
    (0x3299, 0x3299),
    (0x1F000, 0x1F0FF),
    (0x1F10D, 0x1F10F),
    (0x1F12F, 0x1F12F),
    (0x1F16C, 0x1F171),
    (0x1F17E, 0x1F17F),
    (0x1F18E, 0x1F18E),
    (0x1F191, 0x1F19A),
    (0x1F1AD, 0x1F1E5),
    (0x1F201, 0x1F20F),
    (0x1F21A, 0x1F21A),
    (0x1F22F, 0x1F22F),
    (0x1F232, 0x1F23A),
    (0x1F23C, 0x1F23F),
    (0x1F249, 0x1F3FA),
    (0x1F400, 0x1F53D),
    (0x1F546, 0x1F64F),
    (0x1F680, 0x1F6FF),
    (0x1F774, 0x1F77F),
    (0x1F7D5, 0x1F7FF),
    (0x1F80C, 0x1F80F),
    (0x1F848, 0x1F84F),
    (0x1F85A, 0x1F85F),
    (0x1F888, 0x1F88F),
    (0x1F8AE, 0x1F8FF),
    (0x1F90C, 0x1F93A),
    (0x1F93C, 0x1F945),
    (0x1F947, 0x1FAFF),
    (0x1FC00, 0x1FFFD),
];

/// An emoji, or a code point that belongs to an emoji sequence: an
/// `Extended_Pictographic` character, the variation selector 16 and the skin
/// tone modifiers U+1F3FB to U+1F3FF.
fn is_emoji_neighbour(c: char) -> bool {
    let v = c as u32;
    v == 0xFE0F
        || (0x1F3FB..=0x1F3FF).contains(&v)
        || EXTENDED_PICTOGRAPHIC
            .iter()
            .any(|&(lo, hi)| (lo..=hi).contains(&v))
}

/// Whether the joiner `j` between `previous` and `next` (neither is a
/// joiner) carries orthography (RFC 020 D-10):
/// - ZWNJ between two characters of Arabic script, or between two of an
///   Indic script (Persian `می‌خواهم`);
/// - ZWJ in an Indic conjunct: between two Indic characters, next to a
///   virama (Hindi `क्‍ष`, Bengali `র‍্য`);
/// - ZWJ between two emoji (`👨‍👩‍👧`, `🏳️‍🌈`).
fn joiner_is_orthography(j: char, previous: Option<char>, next: Option<char>) -> bool {
    let (Some(before), Some(after)) = (previous, next) else {
        return false;
    };
    if j == ZWNJ {
        return match (joiner_script(before), joiner_script(after)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        };
    }
    let indic = |c| joiner_script(c) == Some(JoinerScript::Indic);
    (indic(before) && indic(after) && (is_virama(before) || is_virama(after)))
        || (is_emoji_neighbour(before) && is_emoji_neighbour(after))
}

/// Whether a character is removed by step 3 (REQ-SEC-015). The joiners
/// ZWNJ and ZWJ are not decided here but by `remove_invisible`, in context.
fn is_removed(c: char) -> bool {
    if is_joiner(c) {
        return false;
    }
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

/// Removes invisible and control characters and counts them. Every `Cc`,
/// `Cf` and unassigned character goes, except that a joiner stays where it
/// carries orthography (RFC 020 D-10): the decision is made on the text
/// after every other removal, with the run of joiners as a unit, so that
/// cleaning the result again changes nothing. At most one joiner is kept
/// from a run, and none at the start or the end of the text.
fn remove_invisible(text: &str) -> (String, u32) {
    let mut removed = 0u32;
    let chars: Vec<char> = text
        .chars()
        .filter(|&c| {
            let drop = is_removed(c);
            removed += u32::from(drop);
            !drop
        })
        .collect();
    let mut out = String::with_capacity(text.len());
    let mut previous: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !is_joiner(c) {
            out.push(c);
            previous = Some(c);
            i += 1;
            continue;
        }
        let end = chars[i..]
            .iter()
            .position(|&n| !is_joiner(n))
            .map_or(chars.len(), |offset| i + offset);
        let next = chars.get(end).copied();
        let kept = chars[i..end]
            .iter()
            .copied()
            .find(|&j| joiner_is_orthography(j, previous, next));
        if let Some(j) = kept {
            out.push(j);
        }
        removed += (end - i) as u32 - u32::from(kept.is_some());
        i = end;
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
    // A cut can leave a joiner at the end, where no joiner is kept (RFC 020
    // D-10): drop it, and the space before it, so that cleaning the result
    // again changes nothing.
    let cut_text = head[..cut]
        .trim_end_matches(|c: char| c.is_whitespace() || is_joiner(c))
        .to_owned();
    (cut_text, true)
}

#[cfg(test)]
mod tests;
