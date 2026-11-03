//! The secret check on queries and URLs (external design §10; RFC 009):
//! high-confidence credential formats only, refusing by default and naming
//! the kind and position of a finding, never its value (REQ-PRIV-002).
//! [`Sensitive`] holds query and URL text so that it cannot reach a log
//! field by accident (REQ-PRIV-004).

use std::sync::LazyLock;

use regex::Regex;

use crate::config::SecretDetection;
use crate::model::SecretKind;

/// One finding: what kind, and where, in character offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The kind of credential.
    pub kind: SecretKind,
    /// Character offset of the first character.
    pub start: u32,
    /// Character offset one past the last character.
    pub end: u32,
}

/// How a match is confirmed after the pattern has matched. A pattern says
/// what the text looks like; a check says whether it looks like a credential
/// and not like prose (RFC 020, DAT-15). The captured group `tail` is what
/// the check reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Check {
    /// The pattern decides.
    Pattern,
    /// A digit, a capital and a lower-case letter: a random base-62 run.
    Mixed,
    /// Capitals and lower-case letters, no digit needed (Hugging Face).
    MixedCase,
    /// A digit and a letter, not hyphenated words.
    DigitAndLetter,
    /// A digit (the base-32 tail of an AWS key id has digits in 2 to 7).
    Digit,
    /// A hexadecimal run with a digit and a letter.
    Hex,
    /// Not hyphenated words (`bearer authentication-and-authorization-flow`).
    NotProse,
    /// A JSON Web Token: the first two segments decode to JSON objects.
    Jwt,
    /// Userinfo with a password that is not a port number after a host name.
    Userinfo,
    /// A basic authorization header whose payload decodes to `user:password`.
    BasicPayload,
    /// A value assigned to a password name that is not an ordinary word.
    PasswordValue,
    /// A token split by one space or line break (a pasted, wrapped token).
    Split(usize),
}

/// One row of the table: the kind it names, the pattern, the confirmation.
struct Rule {
    kind: SecretKind,
    pattern: &'static str,
    check: Check,
}

/// The pattern table: fixed prefixes or structures with the length of the
/// real format; no entropy heuristics, which misfire on hashes and
/// identifiers. The order matters: a finding that overlaps one already found
/// is dropped, so a specific pattern comes before a general one (`Bearer
/// ghp_…` is a GitHub token). Reviewable. Each row is decided by lines of the
/// corpus in `privacy/tests.rs`.
const RULES: &[Rule] = &[
    // AWS: the key id is a prefix and sixteen base-32 characters. `AKIA` is
    // the prefix of a long-term key and is not an English word: the pattern
    // decides, also for the 1 in 27 real ids that have no digit in the tail
    // (RFC 020, N2 of the 020n review).
    Rule {
        kind: SecretKind::AwsAccessKey,
        pattern: r"(?i)\bAKIA[A-Z2-7]{16}\b",
        check: Check::Pattern,
    },
    // `ASIA` (temporary) and `A3T` also begin words: `ASIA` and a 16-letter
    // word ("Asia Pacific trade zone", capitals) has no digit in its tail, so
    // a digit is asked for these two prefixes.
    Rule {
        kind: SecretKind::AwsAccessKey,
        pattern: r"(?i)\b(?:A3T[A-Z0-9]|ASIA)(?P<tail>[A-Z2-7]{16})\b",
        check: Check::Digit,
    },
    Rule {
        kind: SecretKind::GithubToken,
        pattern: r"\bgh[pousr]_[A-Za-z0-9]{36,}\b",
        check: Check::Pattern,
    },
    Rule {
        kind: SecretKind::GithubToken,
        pattern: r"\bgithub_pat_[A-Za-z0-9_]{22,}\b",
        check: Check::Pattern,
    },
    // A GitHub token wrapped by one space or line break (36 characters in all).
    Rule {
        kind: SecretKind::GithubToken,
        pattern: r"\bgh[pousr]_(?P<tail>[A-Za-z0-9]{1,60}[ \t\r\n][A-Za-z0-9]{1,60})\b",
        check: Check::Split(36),
    },
    // Slack: digits, digits, [digits,] a 24-character secret.
    Rule {
        kind: SecretKind::SlackToken,
        pattern: r"\bxox[abprs]-(?:[0-9]{8,13}-){1,3}[A-Za-z0-9]{20,64}\b",
        check: Check::Pattern,
    },
    Rule {
        kind: SecretKind::GoogleApiKey,
        pattern: r"\bAIza[0-9A-Za-z_-]{35}\b",
        check: Check::Pattern,
    },
    Rule {
        kind: SecretKind::StripeKey,
        pattern: r"\b[sr]k_(?:live|test)_(?P<tail>[0-9a-zA-Z]{24,})\b",
        check: Check::Mixed,
    },
    // OpenAI: the legacy key is 48 base-62 characters; the project, service
    // account, admin, Anthropic and OpenRouter keys have a named prefix.
    Rule {
        kind: SecretKind::OpenaiKey,
        pattern: r"\bsk-(?P<tail>[A-Za-z0-9]{40,})\b",
        check: Check::Mixed,
    },
    Rule {
        kind: SecretKind::OpenaiKey,
        pattern: r"\bsk-(?:proj|svcacct|admin|ant|or-v1)-(?P<tail>[A-Za-z0-9_-]{30,})",
        check: Check::NotProse,
    },
    Rule {
        kind: SecretKind::PrivateKey,
        pattern: r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
        check: Check::Pattern,
    },
    Rule {
        kind: SecretKind::Jwt,
        pattern: r"\b(?P<tail>eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,})\.[A-Za-z0-9_-]{10,}\b",
        check: Check::Jwt,
    },
    Rule {
        kind: SecretKind::NpmToken,
        pattern: r"\bnpm_(?P<tail>[A-Za-z0-9]{36})\b",
        check: Check::Mixed,
    },
    Rule {
        kind: SecretKind::GitlabToken,
        pattern: concat!(r"\bglpat", r"-(?P<tail>[A-Za-z0-9_-]{20,})"),
        check: Check::DigitAndLetter,
    },
    Rule {
        kind: SecretKind::HuggingfaceToken,
        pattern: r"\bhf_(?P<tail>[A-Za-z]{34})\b",
        check: Check::MixedCase,
    },
    Rule {
        kind: SecretKind::SendgridKey,
        pattern: r"\bSG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}",
        check: Check::Pattern,
    },
    Rule {
        kind: SecretKind::TelegramBotToken,
        pattern: r"\b[0-9]{8,10}:(?P<tail>[A-Za-z0-9_-]{35})\b",
        check: Check::Mixed,
    },
    Rule {
        kind: SecretKind::TwilioKey,
        pattern: r"\bSK(?P<tail>[0-9a-f]{32})\b",
        check: Check::Hex,
    },
    // The secret access key has no prefix: it is named by its context.
    Rule {
        kind: SecretKind::AwsSecretKey,
        pattern: r#"(?i)\baws_?secret_?(?:access_?)?key["']?[ \t]*[:=][ \t]*["']?[A-Za-z0-9/+=]{40}"#,
        check: Check::Pattern,
    },
    Rule {
        kind: SecretKind::AzureAccountKey,
        pattern: r"(?i)\baccountkey=[A-Za-z0-9+/]{40,}={0,2}",
        check: Check::Pattern,
    },
    Rule {
        kind: SecretKind::BasicAuthHeader,
        pattern: r#"(?i)\bauthorization["']?[ \t]*[:=][ \t]*["']?basic[ \t]+(?P<tail>[A-Za-z0-9+/_-]{8,}={0,2})"#,
        check: Check::BasicPayload,
    },
    Rule {
        kind: SecretKind::BasicAuthUrl,
        pattern: r"(?i)\b[a-z][a-z0-9+.-]*://(?P<user>[^\s/@:]+):(?P<tail>[^\s/@]+)@",
        check: Check::Userinfo,
    },
    // A value given to a password name. The span starts at the name.
    Rule {
        kind: SecretKind::PasswordAssignment,
        pattern: r#"(?i)(?:^|[^a-z])(?P<span>(?:password|passwd|secret)[ \t]*[=:][ \t]*["']?(?P<tail>[^\s"'&;,]{8,}))"#,
        check: Check::PasswordValue,
    },
    // The general rule goes last. The token follows `Bearer`, a space, `%20`
    // or `+`.
    Rule {
        kind: SecretKind::BearerHeader,
        pattern: r"(?i)\bbearer(?:[ \t]+|%20|\+)(?P<tail>[A-Za-z0-9_.~+/=-]{20,})",
        check: Check::NotProse,
    },
];

static COMPILED: LazyLock<Vec<(&'static Rule, Regex)>> = LazyLock::new(|| {
    RULES
        .iter()
        .map(|rule| (rule, Regex::new(rule.pattern).expect("pattern compiles")))
        .collect()
});

fn has_digit(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_digit())
}

fn has_upper(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_uppercase())
}

fn has_lower(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_lowercase())
}

/// Hyphenated (or dotted, or underscored) words: at least two of the parts
/// are three or more lower-case letters. A random token does not read like
/// that; `authentication-and-authorization-flow` does.
fn looks_like_prose(tail: &str) -> bool {
    tail.split(['-', '.', '_', '/'])
        .filter(|part| part.len() >= 3 && part.chars().all(|c| c.is_ascii_lowercase()))
        .count()
        >= 2
}

/// Decodes base64 of either alphabet, with or without padding.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for c in text.trim_end_matches('=').chars() {
        let value = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// A decoded JWT segment is a JSON object: it starts with `{` and ends with
/// `}`.
fn is_json_object(bytes: &[u8]) -> bool {
    bytes.first() == Some(&b'{') && bytes.last() == Some(&b'}')
}

/// The value after `password=`, `passwd=` or `secret=` is not an ordinary
/// word: it has a digit or a symbol, is not a placeholder (`<…>`, `${…}`,
/// `%…`, `****`) and is not one repeated character.
fn is_password_value(value: &str) -> bool {
    let first = value.chars().next().unwrap_or(' ');
    if matches!(first, '<' | '$' | '{' | '%' | '*' | '(' | '[') {
        return false;
    }
    let mut chars = value.chars();
    if let Some(head) = chars.next()
        && chars.all(|c| c == head)
    {
        return false;
    }
    has_digit(value) || value.chars().any(|c| "!@#$%^*+=~|".contains(c))
}

impl Check {
    fn accepts(self, caps: &regex::Captures<'_>) -> bool {
        let tail = caps.name("tail").map_or("", |m| m.as_str());
        match self {
            Check::Pattern => true,
            Check::Mixed => has_digit(tail) && has_upper(tail) && has_lower(tail),
            Check::MixedCase => has_upper(tail) && has_lower(tail),
            Check::DigitAndLetter => {
                has_digit(tail)
                    && tail.chars().any(|c| c.is_ascii_alphabetic())
                    && !looks_like_prose(tail)
            }
            Check::Digit => has_digit(tail),
            Check::Hex => has_digit(tail) && tail.chars().any(|c| c.is_ascii_lowercase()),
            Check::NotProse => !looks_like_prose(tail),
            Check::Jwt => {
                let mut segments = tail.split('.');
                let header = segments.next().and_then(decode_base64);
                let payload = segments.next().and_then(decode_base64);
                match (header, payload) {
                    (Some(header), Some(payload)) => {
                        let text = String::from_utf8_lossy(&header);
                        is_json_object(&header)
                            && (text.contains("\"alg\"") || text.contains("\"enc\""))
                            && is_json_object(&payload)
                    }
                    _ => false,
                }
            }
            Check::Userinfo => {
                let user = caps.name("user").map_or("", |m| m.as_str());
                // `host.example:8080@other` is a host and a port, not
                // credentials: a dotted name and at most five digits.
                let port_like = user.contains('.')
                    && tail.len() <= 5
                    && tail.chars().all(|c| c.is_ascii_digit());
                !port_like
            }
            Check::BasicPayload => decode_base64(tail)
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .is_some_and(|text| {
                    text.split_once(':')
                        .is_some_and(|(user, password)| !user.is_empty() && !password.is_empty())
                }),
            Check::PasswordValue => is_password_value(tail),
            Check::Split(total) => {
                let joined: String = tail.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
                joined.len() >= total
                    && has_digit(&joined)
                    && has_upper(&joined)
                    && has_lower(&joined)
            }
        }
    }
}

/// The text as the check reads it: invisible and control characters taken
/// out (a zero width space inside a key must not hide it), with the original
/// character offset of every character that stays.
struct View {
    text: String,
    /// `None` when nothing was taken out: the offsets are the same.
    origin: Option<Vec<u32>>,
}

impl View {
    fn new(original: &str) -> Self {
        if !original.chars().any(crate::boundary::is_invisible) {
            return View {
                text: original.to_owned(),
                origin: None,
            };
        }
        let mut text = String::with_capacity(original.len());
        let mut origin = Vec::with_capacity(original.len());
        for (index, c) in original.chars().enumerate() {
            if !crate::boundary::is_invisible(c) {
                text.push(c);
                origin.push(index as u32);
            }
        }
        View {
            text,
            origin: Some(origin),
        }
    }

    /// The range `start..end` in characters of the viewed text, as
    /// characters of the original: from the first character that stays to one
    /// past the last, so that an invisible character inside is covered.
    fn original(&self, start: u32, end: u32) -> (u32, u32) {
        match &self.origin {
            None => (start, end),
            Some(origin) => (origin[start as usize], origin[end as usize - 1] + 1),
        }
    }
}

/// Whether a finding and the range `start..end` (character offsets, `end`
/// exclusive) share a character. Ranges that only touch do not overlap.
fn overlaps(found: &Finding, start: u32, end: u32) -> bool {
    found.start < end && start < found.end
}

/// Scans a text and returns every finding, by position in characters of the
/// text as given.
pub fn scan(text: &str) -> Vec<Finding> {
    let view = View::new(text);
    let mut findings: Vec<Finding> = Vec::new();
    for (rule, regex) in COMPILED.iter() {
        for caps in regex.captures_iter(&view.text) {
            if !rule.check.accepts(&caps) {
                continue;
            }
            let m = caps
                .name("span")
                .or_else(|| caps.get(0))
                .expect("a match has a span");
            let first = view.text[..m.start()].chars().count() as u32;
            let last = first + m.as_str().chars().count() as u32;
            let (start, end) = view.original(first, last);
            if !findings.iter().any(|f| overlaps(f, start, end)) {
                findings.push(Finding {
                    kind: rule.kind,
                    start,
                    end,
                });
            }
        }
    }
    findings.sort_by_key(|f| (f.start, f.end));
    findings
}

/// What the check decided for a text under a mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing found, or the scan is off: the text is unchanged.
    Clean,
    /// Refuse the call; the first finding says what and where.
    Refuse(Finding),
    /// Each finding replaced by `[redacted]`; the count for the notice.
    Masked {
        /// The text with every finding replaced.
        text: String,
        /// How many findings were replaced.
        count: u32,
    },
    /// Findings exist but the call proceeds unchanged; the count for the log.
    Warned(u32),
}

/// Applies the configured mode (LIM-P1).
pub fn check(text: &str, mode: SecretDetection) -> Verdict {
    if mode == SecretDetection::Off {
        return Verdict::Clean;
    }
    let findings = scan(text);
    let Some(first) = findings.first() else {
        return Verdict::Clean;
    };
    match mode {
        SecretDetection::Off => Verdict::Clean,
        SecretDetection::Refuse => Verdict::Refuse(first.clone()),
        SecretDetection::Warn => Verdict::Warned(findings.len() as u32),
        SecretDetection::Mask => {
            let mut out = String::with_capacity(text.len());
            let mut cursor = 0u32;
            let chars: Vec<char> = text.chars().collect();
            for finding in &findings {
                out.extend(chars[cursor as usize..finding.start as usize].iter());
                out.push_str("[redacted]");
                cursor = finding.end;
            }
            out.extend(chars[cursor as usize..].iter());
            Verdict::Masked {
                text: out,
                count: findings.len() as u32,
            }
        }
    }
}

/// Text that must never reach a log field: the query and the URL. There is
/// no `Display`; `Debug` prints the length only.
#[derive(Clone, PartialEq, Eq)]
pub struct Sensitive(String);

impl Sensitive {
    /// Wraps a text.
    pub fn new(text: impl Into<String>) -> Self {
        Sensitive(text.into())
    }

    /// Reads the text. The name makes every use visible in review.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// The length in characters, which may be logged.
    pub fn chars(&self) -> usize {
        self.0.chars().count()
    }
}

impl From<&str> for Sensitive {
    fn from(text: &str) -> Self {
        Sensitive(text.to_owned())
    }
}

impl From<String> for Sensitive {
    fn from(text: String) -> Self {
        Sensitive(text)
    }
}

impl std::fmt::Debug for Sensitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sensitive({} chars)", self.chars())
    }
}

#[cfg(test)]
mod tests;
