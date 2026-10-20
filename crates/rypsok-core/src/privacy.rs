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

/// The pattern table: fixed prefixes or structures only; no entropy
/// heuristics, which misfire on hashes and identifiers. Reviewable.
const PATTERNS: &[(SecretKind, &str)] = &[
    (
        SecretKind::AwsAccessKey,
        r"\b(?:A3T[A-Z0-9]|AKIA|ASIA)[A-Z0-9]{16}\b",
    ),
    (SecretKind::GithubToken, r"\bgh[pousr]_[A-Za-z0-9]{36,}\b"),
    (SecretKind::GithubToken, r"\bgithub_pat_[A-Za-z0-9_]{22,}\b"),
    (SecretKind::SlackToken, r"\bxox[abpr]-[A-Za-z0-9-]{10,}\b"),
    (SecretKind::GoogleApiKey, r"\bAIza[0-9A-Za-z_-]{35}\b"),
    (
        SecretKind::StripeKey,
        r"\b[sr]k_(?:live|test)_[0-9a-zA-Z]{24,}\b",
    ),
    (SecretKind::OpenaiKey, r"\bsk-[A-Za-z0-9_-]{20,}\b"),
    (
        SecretKind::PrivateKey,
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
    ),
    (
        SecretKind::Jwt,
        r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b",
    ),
    (
        SecretKind::BearerHeader,
        r"(?i)\bbearer [A-Za-z0-9_.-]{20,}",
    ),
    (
        SecretKind::BasicAuthUrl,
        r"(?i)\b[a-z][a-z0-9+.-]*://[^\s/@:]+:[^\s/@]*@",
    ),
];

static COMPILED: LazyLock<Vec<(SecretKind, Regex)>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .map(|(kind, pattern)| (*kind, Regex::new(pattern).expect("pattern compiles")))
        .collect()
});

/// Scans a text and returns every finding, by position.
pub fn scan(text: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    for (kind, regex) in COMPILED.iter() {
        for m in regex.find_iter(text) {
            let start = text[..m.start()].chars().count() as u32;
            let end = start + m.as_str().chars().count() as u32;
            if !findings.iter().any(|f| f.start < end && start < f.end) {
                findings.push(Finding {
                    kind: *kind,
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

impl std::fmt::Debug for Sensitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sensitive({} chars)", self.chars())
    }
}

#[cfg(test)]
mod tests;
