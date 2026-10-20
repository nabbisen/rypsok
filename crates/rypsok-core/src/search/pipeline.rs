//! The search pipeline (RFC 007): privacy check, engine selection, fan-out
//! under one absolute deadline, collection with early completion, outcome,
//! normalization, deduplication, rank fusion, the content boundary, and
//! the response. No retries, no detached tasks, no arrival-order effects.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::normalize::{self, UrlRejection};
use super::rank::{self, Observation};
use super::{SearchPolicy, ValidatedSearch};
use crate::CancellationToken;
use crate::boundary;
use crate::config::{SafeSearch, SecretDetection};
use crate::engine::registry::{Engine, Registry};
use crate::engine::{EngineErrorClass, EngineRequest, EngineResponse};
use crate::limits::Limits;
use crate::model::{
    Decision, EngineId, Notice, NoticeCode, Outcome, Reason, SearchResponse, ToolError, decide,
};
use crate::privacy::{self, Verdict};
use crate::telemetry::Counters;

/// What the pipeline needs from the process.
pub struct Pipeline<'a> {
    /// The engines.
    pub registry: &'a Registry,
    /// The limits.
    pub limits: &'a Limits,
    /// The validation policy (for the default language).
    pub policy: &'a SearchPolicy,
    /// The region passed to engines.
    pub region: &'a str,
    /// The safe-search level passed to engines.
    pub safe_search: SafeSearch,
    /// The secret-detection mode.
    pub secret_detection: SecretDetection,
    /// Drop results with instruction-like content.
    pub drop_flagged: bool,
    /// Always carry `results[].engines`.
    pub engine_attribution: bool,
    /// The process-wide outbound cap (LIM-C1).
    pub outbound: Arc<Semaphore>,
    /// The counters.
    pub counters: &'a Counters,
}

/// How one engine ended.
#[derive(Debug)]
enum Ended {
    Answered(EngineResponse, Duration),
    Failed(EngineErrorClass, Option<Duration>),
    Skipped(Reason),
}

/// Runs a validated search. The caller has cancelled when `cancel` fires;
/// the result is then not sent and an `unavailable` error is returned so
/// that the call ends with one outcome.
pub async fn run(
    p: &Pipeline<'_>,
    mut search: ValidatedSearch,
    cancel: CancellationToken,
) -> Result<SearchResponse, ToolError> {
    let start = Instant::now();
    let deadline = start + Duration::from_millis(u64::from(p.limits.search_deadline_ms));
    let fan_out_deadline = deadline - Duration::from_millis(u64::from(p.limits.search_reserve_ms));
    let mut notices: Vec<Notice> = Vec::new();

    // Privacy check before any engine request (REQ-SEARCH-006, REQ-PRIV-002).
    match privacy::check(&search.query, p.secret_detection) {
        Verdict::Clean => {}
        Verdict::Refuse(finding) => {
            p.counters.refused("web_search", "secret_in_query");
            tracing::warn!(event = "refused", tool = "web_search", reason = "secret_in_query", kind = ?finding.kind, "call refused");
            return Err(ToolError::secret_in_query(finding.kind, finding.start));
        }
        Verdict::Masked { text, count } => {
            notices.push(Notice::counted(NoticeCode::CredentialMasked, count));
            search.query = text;
        }
        Verdict::Warned(count) => {
            p.counters.refused("web_search", "secret_in_query_warned");
            tracing::warn!(
                event = "secret_in_query",
                tool = "web_search",
                count,
                "query carries what looks like a credential; proceeding (warn mode)"
            );
        }
    }

    // Selection (REQ-ENG-007).
    let default_language = p
        .policy
        .languages
        .first()
        .cloned()
        .unwrap_or_else(|| search.language.clone());
    let mut queried: Vec<Arc<Engine>> = Vec::new();
    for engine in p
        .registry
        .select(search.category, usize::from(p.limits.engines_per_request))
    {
        let Some(properties) = engine.properties() else {
            continue;
        };
        match properties.skip_reason(
            &search.query,
            &search.language,
            &default_language,
            search.time_range,
        ) {
            Some(reason) => notices.push(Notice::engine(
                NoticeCode::EngineSkipped,
                engine.id.clone(),
                reason,
            )),
            None if engine.limiter.is_blocked() => {
                notices.push(Notice::engine(
                    NoticeCode::EngineSkipped,
                    engine.id.clone(),
                    Reason::RateLimited,
                ));
            }
            None => queried.push(engine),
        }
    }
    if queried.is_empty() {
        p.counters.call("web_search", "unavailable");
        return Err(ToolError::no_engine(search.category));
    }

    // Fan-out (REQ-AGG-001 to -003, REQ-AGG-006, REQ-SEC-014).
    let request = EngineRequest {
        query: search.query.clone(),
        max_results: p.limits.results_per_engine,
        language: search.language.clone(),
        region: p.region.to_owned(),
        safe_search: p.safe_search,
        time_range: search.time_range,
    };
    let engine_timeout = Duration::from_millis(u64::from(p.limits.engine_timeout_ms));
    let mut tasks: JoinSet<(usize, Ended)> = JoinSet::new();
    for (index, engine) in queried.iter().enumerate() {
        let engine = Arc::clone(engine);
        let request = request.clone();
        let outbound = Arc::clone(&p.outbound);
        let child = cancel.child_token();
        tasks.spawn(async move {
            let Ok(_global) =
                tokio::time::timeout_at(fan_out_deadline.into(), outbound.acquire_owned()).await
            else {
                return (index, Ended::Skipped(Reason::RateLimited));
            };
            let Ok(_permit) = engine.limiter.acquire(fan_out_deadline).await else {
                return (index, Ended::Skipped(Reason::RateLimited));
            };
            let Some(driver) = engine.driver.as_deref() else {
                return (index, Ended::Skipped(Reason::Cooldown));
            };
            let started = Instant::now();
            let ended =
                match tokio::time::timeout(engine_timeout, driver.search(&request, child)).await {
                    Ok(Ok(response)) => Ended::Answered(response, started.elapsed()),
                    Ok(Err(error)) => Ended::Failed(error.class, error.retry_after),
                    Err(_) => Ended::Failed(EngineErrorClass::Timeout, None),
                };
            (index, ended)
        });
    }

    // Collection with early completion (REQ-AGG-009). Dropping the set
    // aborts what is still running.
    let mut ended: Vec<Option<Ended>> = (0..queried.len()).map(|_| None).collect();
    loop {
        let next = tokio::select! {
            _ = cancel.cancelled() => { drop(tasks); return Err(ToolError::deadline()); }
            _ = tokio::time::sleep_until(fan_out_deadline.into()) => None,
            joined = tasks.join_next() => joined,
        };
        match next {
            Some(Ok((index, result))) => ended[index] = Some(result),
            Some(Err(_join_error)) => {
                // A driver panicked: isolated, classified, logged (REQ-SEC-014).
                tracing::error!(
                    event = "driver_panic",
                    "an engine task panicked; classified as request_failure"
                );
            }
            None => break,
        }
        if ended.iter().all(Option::is_some) {
            break;
        }
    }
    drop(tasks);

    // Outcome (REQ-ERR-001, REQ-ERR-005; external design §7).
    let max_cooldown = Duration::from_secs(u64::from(p.limits.max_cooldown_s));
    let mut answered: Vec<(usize, EngineResponse)> = Vec::new();
    let mut failed = 0u32;
    let mut all_timeouts = true;
    let mut retry_after: Option<Duration> = None;
    for (index, engine) in queried.iter().enumerate() {
        let outcome = ended[index]
            .take()
            .unwrap_or(Ended::Failed(EngineErrorClass::Timeout, None));
        match outcome {
            Ended::Answered(response, latency) => {
                tracing::debug!(event = "engine_answered", engine = %engine.id, results = response.results.len(), latency_ms = latency.as_millis() as u64, "engine answered");
                answered.push((index, response));
            }
            Ended::Failed(class, wait) => {
                failed += 1;
                all_timeouts &= class == EngineErrorClass::Timeout;
                if let Some(wait) = wait {
                    engine.limiter.block_for(wait, max_cooldown);
                    retry_after = Some(retry_after.map_or(wait, |r| r.min(wait)));
                }
                p.counters.engine_error(engine.id.as_str(), class.as_str());
                tracing::info!(event = "engine_failed", engine = %engine.id, class = class.as_str(), "engine failed");
                notices.push(Notice::engine(
                    NoticeCode::EngineUnavailable,
                    engine.id.clone(),
                    class.reason(),
                ));
            }
            Ended::Skipped(reason) => notices.push(Notice::engine(
                NoticeCode::EngineSkipped,
                engine.id.clone(),
                reason,
            )),
        }
    }
    let n = queried.len() as u32;
    let a = answered.len() as u32;
    if let Decision::AllFailed = decide(n, a, failed, 0) {
        p.counters.call("web_search", "unavailable");
        tracing::info!(
            event = "call",
            tool = "web_search",
            outcome = "unavailable",
            latency_ms = start.elapsed().as_millis() as u64,
            engines = n,
            "call ended"
        );
        return Err(if all_timeouts {
            ToolError::deadline()
        } else {
            ToolError::all_failed(retry_after.map(|d| d.as_millis() as u64))
        });
    }

    // Normalization (RFC 008) in selection order, never arrival order.
    let mut observations: Vec<Observation> = Vec::new();
    let mut dropped_invalid = 0u32;
    let mut dropped_long = 0u32;
    for (index, response) in &answered {
        let engine = &queried[*index];
        for (position, raw) in response.results.iter().enumerate() {
            match normalize::returned_url(&raw.url, p.limits.url_max_chars) {
                Ok(returned_url) => {
                    let key = normalize::comparison_key(&returned_url);
                    observations.push(Observation {
                        engine_index: *index,
                        engine: engine.id.clone(),
                        preference: engine.preference,
                        rank: position as u32 + 1,
                        raw: raw.clone(),
                        returned_url,
                        key,
                    });
                }
                Err(UrlRejection::Invalid) => dropped_invalid += 1,
                Err(UrlRejection::TooLong) => dropped_long += 1,
            }
        }
    }
    if dropped_invalid > 0 {
        notices.push(Notice::counted_with_reason(
            NoticeCode::ResultsDropped,
            Reason::UrlInvalid,
            dropped_invalid,
        ));
    }
    if dropped_long > 0 {
        notices.push(Notice::counted_with_reason(
            NoticeCode::ResultsDropped,
            Reason::UrlTooLong,
            dropped_long,
        ));
    }
    let merged = rank::order_and_cut(
        rank::merge(observations),
        usize::from(search.max_results) + 8,
    );

    // The content boundary (RFC 005) and attribution (REQ-CTX-001).
    let mut results = Vec::new();
    let (mut titles_cut, mut snippets_cut, mut removed, mut flagged, mut dropped_flagged) =
        (0u32, 0u32, 0u32, 0u32, 0u32);
    for item in merged {
        if results.len() >= usize::from(search.max_results) {
            break;
        }
        let attribution_required = item.engines.iter().any(|id| {
            queried
                .iter()
                .any(|e| e.id == *id && e.properties().is_some_and(|pr| pr.attribution_required))
        });
        let engines: Option<Vec<EngineId>> =
            (p.engine_attribution || attribution_required).then(|| item.engines.clone());
        let (result, effects) = boundary::make_result(
            &item.best.raw.title,
            &item.best.raw.snippet,
            item.best.returned_url.clone(),
            item.best.raw.date.clone(),
            engines,
            p.limits.title_max_chars,
            p.limits.snippet_max_chars,
        );
        titles_cut += u32::from(effects.title_truncated);
        snippets_cut += u32::from(effects.snippet_truncated);
        removed += effects.characters_removed;
        if effects.instruction_like {
            flagged += 1;
            if p.drop_flagged {
                dropped_flagged += 1;
                continue;
            }
        }
        results.push(result);
    }
    if titles_cut > 0 {
        notices.push(Notice::counted(NoticeCode::TitleTruncated, titles_cut));
    }
    if snippets_cut > 0 {
        notices.push(Notice::counted(NoticeCode::SnippetTruncated, snippets_cut));
    }
    if removed > 0 {
        notices.push(Notice::counted(NoticeCode::CharactersRemoved, removed));
        p.counters.characters_removed(u64::from(removed));
    }
    if flagged > 0 {
        notices.push(Notice::counted(NoticeCode::InstructionLikeContent, flagged));
    }
    if dropped_flagged > 0 {
        notices.push(Notice::counted_with_reason(
            NoticeCode::ResultsDropped,
            Reason::InstructionLikeContent,
            dropped_flagged,
        ));
    }
    p.counters
        .results_dropped(u64::from(dropped_invalid + dropped_long + dropped_flagged));

    let outcome = match decide(n, a, failed, results.len() as u32) {
        Decision::Outcome(outcome) => outcome,
        Decision::NoEngine | Decision::AllFailed => Outcome::Empty,
    };
    let outcome_name = match outcome {
        Outcome::Ok => "ok",
        Outcome::Partial => "partial",
        Outcome::Empty => "empty",
    };
    p.counters.call("web_search", outcome_name);
    tracing::info!(
        event = "call",
        tool = "web_search",
        outcome = outcome_name,
        latency_ms = start.elapsed().as_millis() as u64,
        engines = n,
        answered = a,
        failed,
        results = results.len(),
        "call ended"
    );
    Ok(SearchResponse {
        outcome,
        notices,
        results,
    })
}

#[cfg(test)]
mod tests;
