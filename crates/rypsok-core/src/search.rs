//! The search pipeline: validation, privacy check, engine selection,
//! fan-out under one deadline, collection, normalization, deduplication,
//! rank fusion, the content boundary and the response (external design §6
//! and §7; RFC 007 and RFC 008).
//!
//! This module holds request validation (REQ-SEARCH-006); the pipeline
//! stages arrive with their RFCs.

pub mod normalize;
pub mod pipeline;
pub mod rank;

use std::collections::BTreeSet;

use crate::model::{Category, Language, SearchRequest, TimeRange, ToolError};

/// What validation checks a request against: the limits and the
/// configuration of the process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPolicy {
    /// LIM-S1.
    pub query_max_chars: u32,
    /// LIM-S2, default.
    pub max_results_default: u8,
    /// LIM-S2, upper bound.
    pub max_results_ceiling: u8,
    /// Languages the operator configured; the first is the default.
    pub languages: Vec<Language>,
    /// Categories at least one enabled engine serves.
    pub categories: BTreeSet<Category>,
}

impl SearchPolicy {
    /// The accepted names of `category`, in vocabulary order.
    pub fn category_names(&self) -> Vec<String> {
        Category::ALL
            .iter()
            .filter(|c| self.categories.contains(c))
            .map(|c| c.as_str().to_owned())
            .collect()
    }

    /// The accepted names of `language`, in configured order.
    pub fn language_names(&self) -> Vec<String> {
        self.languages
            .iter()
            .map(|l| l.as_str().to_owned())
            .collect()
    }
}

/// A request after validation: every parameter resolved to its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSearch {
    /// The query, trimmed.
    pub query: String,
    /// The category.
    pub category: Category,
    /// How many results at most.
    pub max_results: u8,
    /// The language.
    pub language: Language,
    /// The period, when one was asked for.
    pub time_range: Option<TimeRange>,
}

/// Validates a request against the policy, in parameter order
/// (REQ-SEARCH-001 to -005). The first failing parameter is reported.
pub fn validate(req: &SearchRequest, policy: &SearchPolicy) -> Result<ValidatedSearch, ToolError> {
    let query = req.query.trim();
    let length = query.chars().count();
    if length == 0 || length > policy.query_max_chars as usize {
        return Err(ToolError::query_length(policy.query_max_chars));
    }
    let category = req.category.unwrap_or(Category::General);
    if !policy.categories.contains(&category) {
        return Err(ToolError::not_allowed("category", policy.category_names()));
    }
    let max_results = req.max_results.unwrap_or(policy.max_results_default);
    if max_results == 0 || max_results > policy.max_results_ceiling {
        return Err(ToolError::out_of_range(
            "max_results",
            1,
            u32::from(policy.max_results_ceiling),
        ));
    }
    let language = match &req.language {
        None => policy
            .languages
            .first()
            .cloned()
            .ok_or_else(|| ToolError::not_allowed("language", Vec::new()))?,
        Some(l) if policy.languages.contains(l) => l.clone(),
        Some(_) => return Err(ToolError::not_allowed("language", policy.language_names())),
    };
    Ok(ValidatedSearch {
        query: query.to_owned(),
        category,
        max_results,
        language,
        time_range: req.time_range,
    })
}

#[cfg(test)]
mod tests;
