//! Validated identifiers and small value types of the contract.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The identifier of a configured engine: lowercase letters, digits and
/// underscores, starting with a letter, at most 32 characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct EngineId(String);

/// Why a string is not an engine identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "an engine identifier is 1 to 32 lowercase letters, digits or underscores, starting with a letter"
)]
pub struct InvalidEngineId;

impl EngineId {
    /// Validates and wraps an identifier.
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidEngineId> {
        let id = id.into();
        let mut chars = id.chars();
        let valid_first = chars.next().is_some_and(|c| c.is_ascii_lowercase());
        let valid_rest = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if valid_first && valid_rest && id.len() <= 32 {
            Ok(EngineId(id))
        } else {
            Err(InvalidEngineId)
        }
    }

    /// The identifier as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for EngineId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        EngineId::new(raw).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for EngineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A language tag: two or three lowercase letters, optionally followed by a
/// hyphen and a two-letter region in uppercase, as in `en` or `pt-BR`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct Language(String);

/// Why a string is not a language tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "a language is two or three lowercase letters, optionally followed by a hyphen and a two-letter uppercase region"
)]
pub struct InvalidLanguage;

impl Language {
    /// Validates and wraps a language tag.
    pub fn new(tag: impl Into<String>) -> Result<Self, InvalidLanguage> {
        let tag = tag.into();
        let (primary, region) = match tag.split_once('-') {
            Some((p, r)) => (p, Some(r)),
            None => (tag.as_str(), None),
        };
        let primary_ok =
            (2..=3).contains(&primary.len()) && primary.chars().all(|c| c.is_ascii_lowercase());
        let region_ok =
            region.is_none_or(|r| r.len() == 2 && r.chars().all(|c| c.is_ascii_uppercase()));
        if primary_ok && region_ok {
            Ok(Language(tag))
        } else {
            Err(InvalidLanguage)
        }
    }

    /// The tag as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The primary language subtag, without the region.
    pub fn primary(&self) -> &str {
        self.0.split('-').next().unwrap_or(&self.0)
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Language::new(raw).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A calendar date in RFC 3339 form, `YYYY-MM-DD` (LIM-U4).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct Date(String);

/// Why a string is not a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a date is YYYY-MM-DD")]
pub struct InvalidDate;

impl Date {
    /// Validates and wraps a date.
    pub fn new(text: impl Into<String>) -> Result<Self, InvalidDate> {
        let text = text.into();
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(InvalidDate);
        }
        let digits = |range: std::ops::Range<usize>| -> Result<u32, InvalidDate> {
            text[range].parse::<u32>().map_err(|_| InvalidDate)
        };
        if !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        {
            return Err(InvalidDate);
        }
        let (year, month, day) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => return Err(InvalidDate),
        };
        if day == 0 || day > days_in_month {
            return Err(InvalidDate);
        }
        Ok(Date(text))
    }

    /// The date as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Date::new(raw).map_err(serde::de::Error::custom)
    }
}
