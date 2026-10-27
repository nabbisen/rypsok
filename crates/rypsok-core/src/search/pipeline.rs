//! The search pipeline (RFC 007): privacy check, engine selection, fan-out
//! under one absolute deadline, collection with early completion, outcome,
//! normalization, deduplication, rank fusion, the content boundary, and
//! the response. No retries, no detached tasks, no arrival-order effects.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// Tokio's clock, so that the deadlines follow a paused test clock and
// agree with the health table; in production it is the system clock.
use tokio::time::Instant;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::normalize::{self, UrlRejection};
use super::rank::{self, Observation};
use super::{SearchPolicy, ValidatedSearch};
use crate::CancellationToken;
use crate::boundary;
use crate::cache::{CacheKey, NotStored, ResultCache};
use crate::config::{SafeSearch, SecretDetection};
use crate::egress::ReturnedUrls;
use crate::engine::registry::{Engine, Registry};
use crate::engine::{EngineErrorClass, EngineRequest, EngineResponse};
use crate::limits::Limits;
use crate::model::{
    CallError, CallerId, Decision, EngineId, Notice, NoticeCode, Outcome, Reason, SearchResponse,
    ToolError, decide,
};
use crate::privacy::{self, Sensitive, Verdict};
use crate::telemetry::Counters;
use crate::telemetry::health::{Admission, Completion, HealthTable};

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
    /// Log the query text at `debug` level (`logging.log_queries`, LIM-P3).
    pub log_queries: bool,
    /// The process-wide outbound cap (LIM-C1).
    pub outbound: Arc<Semaphore>,
    /// The counters.
    pub counters: &'a Counters,
    /// The circuits and latency windows (RFC 010).
    pub health: &'a HealthTable,
    /// The URLs returned in this process, for strict mode (REQ-SEC-021).
    pub returned: &'a ReturnedUrls,
    /// Whose call this is: the cache keys are the caller's (REQ-CACHE-004).
    pub caller: &'a CallerId,
    /// The result cache, when the operator enabled it (RFC 014).
    pub cache: Option<&'a ResultCache>,
}

/// How one engine ended.
#[derive(Debug)]
enum Ended {
    Answered(EngineResponse, Duration),
    /// The cache answered for the engine (RFC 014): no request was made.
    Cached(EngineResponse),
    Failed {
        class: EngineErrorClass,
        retry_after: Option<Duration>,
        /// The duration when a response was received; none for a timeout.
        latency: Option<Duration>,
    },
    Skipped(Reason),
}

/// Runs a validated search. The caller has cancelled when `cancel` fires;
/// nothing is returned: the call ends as [`CallError::Cancelled`]
/// (REQ-MCP-007).
pub async fn run(
    p: &Pipeline<'_>,
    mut search: ValidatedSearch,
    cancel: CancellationToken,
) -> Result<SearchResponse, CallError> {
    let start = Instant::now();
    let deadline = start + Duration::from_millis(u64::from(p.limits.search_deadline_ms));
    let fan_out_deadline = deadline - Duration::from_millis(u64::from(p.limits.search_reserve_ms));
    let mut notices: Vec<Notice> = Vec::new();
    // A query the privacy check warned about is never retained (RFC 014).
    let mut cache_bypassed = false;

    // Privacy check before any engine request (REQ-SEARCH-006, REQ-PRIV-002).
    match privacy::check(search.query.expose(), p.secret_detection) {
        Verdict::Clean => {}
        Verdict::Refuse(finding) => {
            p.counters.refused("web_search", "secret_in_query");
            tracing::warn!(event = "refused", tool = "web_search", reason = "secret_in_query", kind = ?finding.kind, "call refused");
            return Err(ToolError::secret_in_query(finding.kind, finding.start).into());
        }
        Verdict::Masked { text, count } => {
            notices.push(Notice::counted(NoticeCode::CredentialMasked, count));
            search.query = Sensitive::new(text);
        }
        Verdict::Warned(count) => {
            cache_bypassed = true;
            if p.cache.is_some() {
                p.counters.cache("bypassed");
            }
            p.counters.warned("web_search", "secret_in_query");
            tracing::warn!(
                event = "secret_in_query",
                tool = "web_search",
                count,
                "query carries what looks like a credential; proceeding (warn mode)"
            );
        }
    }
    if p.log_queries {
        // The one place the query text may be logged: unlocked by the
        // operator, at debug level only (RFC 009).
        tracing::debug!(
            event = "query",
            tool = "web_search",
            query = search.query.expose(),
            "query text (logging.log_queries)"
        );
    }

    // Selection (REQ-ENG-007).
    let default_language = p
        .policy
        .languages
        .first()
        .cloned()
        .unwrap_or_else(|| search.language.clone());
    let mut queried: Vec<Arc<Engine>> = Vec::new();
    // Whether each queried engine carries the probe of a half-open circuit.
    let mut probes: Vec<bool> = Vec::new();
    for engine in p
        .registry
        .select(search.category, usize::from(p.limits.engines_per_request))
    {
        let Some(properties) = engine.properties() else {
            continue;
        };
        match properties.skip_reason(
            search.query.expose(),
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
            None => {
                // The circuit (RFC 010): an elapsed cooldown makes this call
                // the one probe; another call's probe means a skip.
                let (admission, transition) = p.health.admit(&engine.id);
                if let Some(transition) = transition {
                    transition.log();
                }
                match admission {
                    Admission::Skipped => notices.push(Notice::engine(
                        NoticeCode::EngineSkipped,
                        engine.id.clone(),
                        Reason::Cooldown,
                    )),
                    Admission::Allowed | Admission::Probe => {
                        queried.push(engine);
                        probes.push(admission == Admission::Probe);
                    }
                }
            }
        }
    }
    if queried.is_empty() {
        p.counters.call("web_search", "unavailable");
        return Err(ToolError::no_engine(search.category).into());
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
    // Task identifier → engine index, so that a panicked task is attributed
    // to its engine (REQ-AGG-007).
    let mut task_engines: HashMap<tokio::task::Id, usize> = HashMap::new();
    let mut ended: Vec<Option<Ended>> = (0..queried.len()).map(|_| None).collect();
    for (index, engine) in queried.iter().enumerate() {
        // The cache (RFC 014): an admitted, cacheable engine is looked up
        // before any permit is taken; a probe reaches the network.
        if let Some(cache) = cache_for(p, engine, probes[index], cache_bypassed) {
            let key = CacheKey::new(p.caller, &engine.id, &request);
            if let Some(response) = cache.lookup(&key) {
                ended[index] = Some(Ended::Cached(response));
                continue;
            }
            p.counters.cache("miss");
        }
        let engine = Arc::clone(engine);
        let request = request.clone();
        let outbound = Arc::clone(&p.outbound);
        let child = cancel.child_token();
        let handle = tasks.spawn(async move {
            let Ok(_global) =
                tokio::time::timeout_at(fan_out_deadline, outbound.acquire_owned()).await
            else {
                return (index, Ended::Skipped(Reason::RateLimited));
            };
            let Ok(_permit) = engine.limiter.acquire(fan_out_deadline.into_std()).await else {
                return (index, Ended::Skipped(Reason::RateLimited));
            };
            let Some(driver) = engine.driver.as_deref() else {
                return (index, Ended::Skipped(Reason::Cooldown));
            };
            let started = Instant::now();
            let ended =
                match tokio::time::timeout(engine_timeout, driver.search(&request, child)).await {
                    Ok(Ok(response)) => Ended::Answered(response, started.elapsed()),
                    Ok(Err(error)) => Ended::Failed {
                        class: error.class,
                        retry_after: error.retry_after,
                        latency: Some(started.elapsed()),
                    },
                    Err(_) => Ended::Failed {
                        class: EngineErrorClass::Timeout,
                        retry_after: None,
                        latency: None,
                    },
                };
            (index, ended)
        });
        task_engines.insert(handle.id(), index);
    }

    // Collection with early completion (REQ-AGG-009). Dropping the set
    // aborts what is still running.
    loop {
        if ended.iter().all(Option::is_some) {
            break;
        }
        let next = tokio::select! {
            _ = cancel.cancelled() => {
                drop(tasks);
                settle_cancelled(p, &queried, &probes, &ended);
                return Err(cancelled(p, start));
            }
            _ = tokio::time::sleep_until(fan_out_deadline) => None,
            joined = tasks.join_next() => joined,
        };
        match next {
            Some(Ok((index, result))) => ended[index] = Some(result),
            Some(Err(join_error)) => {
                // A driver panicked: isolated, attributed to its engine,
                // classified as a request failure, logged (REQ-SEC-014,
                // REQ-AGG-007).
                if let Some(&index) = task_engines.get(&join_error.id()) {
                    tracing::error!(
                        event = "driver_panic",
                        engine = %queried[index].id,
                        "an engine task panicked; classified as request_failure"
                    );
                    ended[index] = Some(Ended::Failed {
                        class: EngineErrorClass::RequestFailure,
                        retry_after: None,
                        latency: None,
                    });
                }
            }
            None => break,
        }
        if ended.iter().all(Option::is_some) {
            break;
        }
    }
    drop(tasks);
    // Observed between stages as well (RFC 007): an engine that ends on the
    // same tick as the cancellation must not turn the call into an outcome.
    if cancel.is_cancelled() {
        settle_cancelled(p, &queried, &probes, &ended);
        return Err(cancelled(p, start));
    }

    // Outcome (REQ-ERR-001, REQ-ERR-005; external design §7).
    let max_cooldown = Duration::from_secs(u64::from(p.limits.max_cooldown_s));
    let mut answered: Vec<(usize, EngineResponse)> = Vec::new();
    let mut failed = 0u32;
    let mut all_timeouts = true;
    let mut retry_after: Option<Duration> = None;
    for (index, engine) in queried.iter().enumerate() {
        let outcome = ended[index].take().unwrap_or(Ended::Failed {
            class: EngineErrorClass::Timeout,
            retry_after: None,
            latency: None,
        });
        inform_health(p, engine, probes[index], Some(&outcome));
        match outcome {
            Ended::Answered(response, latency) => {
                tracing::debug!(event = "engine_answered", engine = %engine.id, results = response.results.len(), latency_ms = latency.as_millis() as u64, cached = false, "engine answered");
                // A fresh answer is stored for the next call (RFC 014); a
                // probe's answer too, it is as fresh as any.
                if let Some(cache) = cache_for(p, engine, false, cache_bypassed) {
                    let key = CacheKey::new(p.caller, &engine.id, &request);
                    p.counters.cache(match cache.store(key, &response) {
                        Ok(()) => "stored",
                        Err(NotStored::TooLarge) => "too_large",
                        Err(NotStored::Disabled) => "disabled",
                    });
                }
                answered.push((index, response));
            }
            Ended::Cached(response) => {
                p.counters.cache("hit");
                tracing::debug!(event = "engine_answered", engine = %engine.id, results = response.results.len(), cached = true, "engine answered from the cache");
                notices.push(Notice::cached(engine.id.clone()));
                answered.push((index, response));
            }
            Ended::Failed {
                class,
                retry_after: wait,
                ..
            } => {
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
            ToolError::deadline().into()
        } else {
            ToolError::all_failed(retry_after.map(|d| d.as_millis() as u64)).into()
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
    let merged = rank::order(rank::merge(observations));

    // The content boundary (RFC 005), attribution (REQ-CTX-001) and the cut
    // to `max_results` (REQ-RANK-007): every candidate is available, so a
    // dropped result never leaves a hole, and the notices describe only
    // what is returned.
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
        // What is returned may be fetched in strict mode (RFC 011).
        p.returned.record(&item.best.key);
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

/// Informs the health table how an engine ended (REQ-OBS-002, REQ-RES-004,
/// REQ-RES-005) and logs what it asks for. `None` is an engine whose
/// request never completed: nothing to count, except that a probe without
/// a verdict reopens its circuit.
fn inform_health(p: &Pipeline<'_>, engine: &Engine, probe: bool, ended: Option<&Ended>) {
    let completion = match ended {
        Some(Ended::Answered(_, latency)) => {
            p.health.record_latency(&engine.id, *latency);
            Some(Completion::Success)
        }
        Some(Ended::Failed {
            class,
            retry_after,
            latency,
        }) => {
            // A classified failure received a response and counts toward the
            // latency window; a timeout, whether the driver's or the
            // pipeline's, did not (REQ-OBS-002).
            if let Some(latency) = latency
                && *class != EngineErrorClass::Timeout
            {
                p.health.record_latency(&engine.id, *latency);
            }
            Some(Completion::Failure {
                class: *class,
                retry_after: *retry_after,
            })
        }
        Some(Ended::Skipped(_)) | None => probe.then_some(Completion::Cancelled),
        // No request was made; the health table has nothing to learn, and a
        // probe never comes from the cache.
        Some(Ended::Cached(_)) => None,
    };
    let Some(completion) = completion else {
        return;
    };
    let completed = p.health.complete(&engine.id, completion);
    if completed.warn_auth {
        tracing::warn!(
            event = "auth_failure",
            engine = %engine.id,
            "the provider refused the credential; the engine stays selected so that a corrected key takes effect at once"
        );
    }
    if let Some(transition) = completed.transition {
        transition.log();
    }
}

/// The cache that applies to `engine` on this call, if any (RFC 014): the
/// operator enabled it, the engine's record allows storage, the query was
/// not warned about, and the engine is not the probe of a circuit.
fn cache_for<'a>(
    p: &Pipeline<'a>,
    engine: &Engine,
    probe: bool,
    bypassed: bool,
) -> Option<&'a ResultCache> {
    p.cache
        .filter(|_| !bypassed && !probe && engine.properties().is_some_and(|pr| pr.cacheable))
}

/// On cancellation, what did complete is still an observation; a probe that
/// did not complete reopens its circuit (RFC 010).
fn settle_cancelled(
    p: &Pipeline<'_>,
    queried: &[Arc<Engine>],
    probes: &[bool],
    ended: &[Option<Ended>],
) {
    for (index, engine) in queried.iter().enumerate() {
        inform_health(p, engine, probes[index], ended[index].as_ref());
    }
}

/// The end of a cancelled call: counted, logged, nothing returned
/// (REQ-MCP-007).
fn cancelled(p: &Pipeline<'_>, start: Instant) -> CallError {
    p.counters.call("web_search", "cancelled");
    tracing::info!(
        event = "call",
        tool = "web_search",
        outcome = "cancelled",
        latency_ms = start.elapsed().as_millis() as u64,
        "call cancelled by the caller"
    );
    CallError::Cancelled
}

#[cfg(test)]
mod tests;
