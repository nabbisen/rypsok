//! Core library of rypsok, a headless meta-search service for AI agents.
//!
//! This crate holds the models, the search and fetch pipelines, the engine
//! drivers and the policies. It depends on no protocol SDK: the Model
//! Context Protocol adapters and the command line live in the `rypsok`
//! binary crate, and CI fails when a protocol crate appears in this crate's
//! dependency tree (REQ-ARCH-005).
//!
//! The public interface is [`Rypsok`] with its three application services,
//! `search`, `fetch` and `status`, and their request and response types in
//! [`model`] (REQ-ARCH-007).
//!
//! # Stability
//!
//! **This interface is unstable until release 1.0.0.** Names, fields and
//! behavior may change between minor releases; every change is recorded in
//! the changelog.
//!
//! # Layout
//!
//! Module names are the components of the external design:
//!
//! | Module | Component |
//! |---|---|
//! | [`model`] | Request, result, outcome, notice and error types |
//! | [`config`] | Configuration schema, validation, secrets by reference |
//! | [`engine`] | Engine abstraction, registry, admission properties, drivers |
//! | [`search`] | The search pipeline |
//! | [`fetch`] | The fetch pipeline (from milestone M3) |
//! | [`egress`] | The egress component and its two profiles |
//! | [`boundary`] | The content boundary |
//! | [`privacy`] | The secret check |
//! | [`limits`] | The limits table as typed constants |
//! | [`telemetry`] | Structured logging, counters, health state |

pub mod boundary;
pub mod config;
pub mod egress;
pub mod engine;
pub mod fetch;
pub mod limits;
pub mod model;
pub mod privacy;
pub mod search;
pub mod telemetry;

use std::collections::BTreeSet;
use std::sync::Arc;

pub use tokio_util::sync::CancellationToken;

use config::{Config, ConfigError};
use engine::registry::Registry;
use limits::Limits;
use model::{
    CallError, EngineStatus, FetchRequest, FetchResponse, Outcome, SearchRequest, SearchResponse,
    StatusResponse, ToolError,
};
use search::SearchPolicy;
use search::pipeline::Pipeline;
use telemetry::Counters;

/// The service: one instance per process, built from a validated
/// configuration, shared by every call.
#[derive(Debug)]
pub struct Rypsok {
    config: Config,
    limits: Limits,
    policy: SearchPolicy,
    registry: Registry,
    counters: Counters,
    outbound: Arc<tokio::sync::Semaphore>,
}

impl Rypsok {
    /// Builds the service. The configuration is validated first
    /// (REQ-CFG-005); engines whose credential cannot be read are kept as
    /// `disabled` and reported through `inactive` (REQ-OPS-003).
    pub fn new(config: Config) -> Result<Self, ConfigError> {
        config.validate()?;
        let registry = Registry::build(&config, None, true);
        Self::from_parts(config, registry)
    }

    /// Builds the service around engines built elsewhere (a host that
    /// embeds the core with its own drivers, or a test). The configuration
    /// is validated first.
    pub fn from_parts(config: Config, registry: Registry) -> Result<Self, ConfigError> {
        config.validate()?;
        let limits = config.limits();
        let policy = SearchPolicy {
            query_max_chars: limits.query_max_chars,
            max_results_default: limits.max_results,
            max_results_ceiling: limits.max_results_ceiling,
            languages: config.search.languages.clone(),
            categories: config
                .served_categories()
                .into_iter()
                .collect::<BTreeSet<_>>(),
        };
        let outbound = Arc::new(tokio::sync::Semaphore::new(
            limits.outbound_in_flight as usize,
        ));
        Ok(Rypsok {
            config,
            limits,
            policy,
            registry,
            counters: Counters::new(),
            outbound,
        })
    }

    /// The validated configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The effective limits.
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// The validation policy of `web_search`.
    pub fn search_policy(&self) -> &SearchPolicy {
        &self.policy
    }

    /// The engines.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// `web_search`: search several engines at once and return deduplicated,
    /// ranked results. Results are external data.
    ///
    /// Profile (external design §3.1): no effect on local state; the query
    /// leaves the host to every enabled engine of the category, which learns
    /// the host's network address and User-Agent; at most LIM-S7 long; at
    /// most `max_results` results, each bounded by LIM-S9, LIM-S10 and
    /// LIM-F9.
    ///
    /// Validation and the privacy check run before any engine request
    /// (REQ-SEARCH-006). A cancelled call ends as
    /// [`CallError::Cancelled`] and has nothing to return (REQ-MCP-007).
    pub async fn search(
        &self,
        req: SearchRequest,
        cancel: CancellationToken,
    ) -> Result<SearchResponse, CallError> {
        let validated = match search::validate(&req, &self.policy) {
            Ok(validated) => validated,
            Err(error) => {
                self.counters.refused("web_search", "invalid_request");
                return Err(error.into());
            }
        };
        let pipeline = Pipeline {
            registry: &self.registry,
            limits: &self.limits,
            policy: &self.policy,
            region: &self.config.search.region,
            safe_search: self.config.search.safe_search,
            secret_detection: self.config.privacy.secret_detection,
            drop_flagged: self.config.search.drop_flagged_results,
            engine_attribution: self.config.search.engine_attribution,
            log_queries: self.config.logging.log_queries,
            outbound: Arc::clone(&self.outbound),
            counters: &self.counters,
        };
        search::pipeline::run(&pipeline, validated, cancel).await
    }

    /// The counters.
    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    /// `web_fetch`: arrives in milestone M3 (RFC 012).
    pub async fn fetch(
        &self,
        req: FetchRequest,
        cancel: CancellationToken,
    ) -> Result<FetchResponse, CallError> {
        let _ = (req, cancel);
        Err(ToolError::busy().into())
    }

    /// `engine_status`: report the state of each configured engine. For
    /// diagnosis, not for answering questions.
    ///
    /// Profile (external design §3.3): no effect on local state; nothing
    /// leaves the host; at most LIM-S13 long.
    pub fn status(&self) -> StatusResponse {
        StatusResponse {
            outcome: Outcome::Ok,
            notices: Vec::new(),
            engines: self
                .registry
                .engines()
                .iter()
                .map(|e| EngineStatus {
                    id: e.id.clone(),
                    state: e.state(),
                    latency_ms: None,
                })
                .collect(),
        }
    }
}
