//! Deduplication and rank fusion (RFC 008): results with one comparison
//! key merge into one (REQ-RANK-004); reciprocal rank fusion in integer
//! arithmetic orders them (REQ-RANK-005) under a total order
//! (REQ-RANK-006); the list is cut to `max_results` (REQ-RANK-007).

use std::collections::HashMap;

use super::normalize::ComparisonKey;
use crate::engine::RawResult;
use crate::model::EngineId;

/// Observations of one key: selection index, engine, rank.
type Seen = Vec<(usize, EngineId, u32)>;

/// The standing of an observation for the choice of the best: rank, then
/// the engine's preference, then its selection index (lower is better).
type Standing = (u32, u32, usize);

/// The best-ranked `https` observation of a key: its standing and URL.
type BestHttps = Option<(Standing, String)>;

fn standing(observation: &Observation) -> Standing {
    (
        observation.rank,
        observation.preference,
        observation.engine_index,
    )
}

fn is_https(observation: &Observation) -> bool {
    observation.returned_url.starts_with("https://")
}

/// The `k` of reciprocal rank fusion.
pub const RRF_K: u64 = 60;
/// The integer scale of one contribution.
pub const RRF_SCALE: u64 = 1_000_000;

/// One result as one engine returned it, in that engine's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// Position of the engine in the selection (0 first).
    pub engine_index: usize,
    /// The engine.
    pub engine: EngineId,
    /// The engine's preference (lower first).
    pub preference: u32,
    /// 1-based rank within the engine's results.
    pub rank: u32,
    /// The raw result.
    pub raw: RawResult,
    /// The returned URL of RFC 008.
    pub returned_url: String,
    /// The comparison key.
    pub key: ComparisonKey,
}

/// One resource after merging its observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    /// The observation with the best rank; ties break by preference.
    pub best: Observation,
    /// The engines that returned it, in selection order.
    pub engines: Vec<EngineId>,
    /// Every observation as engine identifier and rank, in selection order
    /// (REQ-RANK-001).
    pub ranks: Vec<(EngineId, u32)>,
    /// The fused score.
    pub score: u64,
    /// The URL to show: the best observation's own, unless it is `http`
    /// and a merged observation is `https` (then the best-ranked `https`
    /// one). Always one engine's URL, byte for byte.
    pub url: String,
}

/// Groups observations by key in selection order and keeps the best
/// observation of each (REQ-RANK-004). Engines and ranks are listed in
/// selection order, never arrival order (REQ-ARCH-004).
pub fn merge(observations: Vec<Observation>) -> Vec<Merged> {
    let mut order: Vec<ComparisonKey> = Vec::new();
    let mut groups: HashMap<ComparisonKey, (Merged, Seen, BestHttps)> = HashMap::new();
    for observation in observations {
        let contribution = RRF_SCALE / (RRF_K + u64::from(observation.rank));
        let seen = (
            observation.engine_index,
            observation.engine.clone(),
            observation.rank,
        );
        let https = is_https(&observation)
            .then(|| (standing(&observation), observation.returned_url.clone()));
        match groups.get_mut(&observation.key) {
            None => {
                order.push(observation.key.clone());
                let merged = Merged {
                    best: observation.clone(),
                    engines: Vec::new(),
                    ranks: Vec::new(),
                    score: contribution,
                    url: observation.returned_url.clone(),
                };
                groups.insert(observation.key, (merged, vec![seen], https));
            }
            Some((merged, seen_list, best_https)) => {
                merged.score += contribution;
                seen_list.push(seen);
                if let Some(candidate) = https
                    && best_https
                        .as_ref()
                        .is_none_or(|(held, _)| candidate.0 < *held)
                {
                    *best_https = Some(candidate);
                }
                if standing(&observation) < standing(&merged.best) {
                    merged.best = observation;
                }
            }
        }
    }
    order
        .into_iter()
        .filter_map(|key| groups.remove(&key))
        .map(|(mut merged, mut seen_list, best_https)| {
            // The URL shown: the best observation's own, unless it is
            // `http` and a merged observation is `https` (RFC 020 D-8).
            if merged.best.returned_url.starts_with("http://")
                && let Some((_, url)) = best_https
            {
                merged.url = url;
            } else {
                merged.url = merged.best.returned_url.clone();
            }
            seen_list.sort();
            merged.ranks = seen_list
                .iter()
                .map(|(_, id, rank)| (id.clone(), *rank))
                .collect();
            for (_, id, _) in seen_list {
                if !merged.engines.contains(&id) {
                    merged.engines.push(id);
                }
            }
            merged
        })
        .collect()
}

/// Sorts by score descending, then best rank ascending, then the
/// preference of the engine that gave the best rank ascending, then the
/// comparison key ascending (REQ-RANK-006). The cut to `max_results`
/// (REQ-RANK-007) is the pipeline's, after the content boundary, so that a
/// dropped result never leaves a hole.
pub fn order(mut merged: Vec<Merged>) -> Vec<Merged> {
    merged.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.best.rank.cmp(&b.best.rank))
            .then_with(|| a.best.preference.cmp(&b.best.preference))
            .then_with(|| a.best.key.cmp(&b.best.key))
    });
    merged
}

#[cfg(test)]
mod tests;
