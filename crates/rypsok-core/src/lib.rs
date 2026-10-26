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
use egress::{AgentClientSettings, EgressPolicy, ReturnedUrls};
use engine::registry::Registry;
use fetch::FetchService;
use limits::Limits;
use model::{
    CallError, FetchRequest, FetchResponse, Outcome, SearchRequest, SearchResponse, StatusResponse,
};
use search::SearchPolicy;
use search::pipeline::Pipeline;
use telemetry::{Counters, HealthTable};

/// The service: one instance per process, built from a validated
/// configuration, shared by every call.
#[derive(Debug)]
pub struct Rypsok {
    config: Config,
    limits: Limits,
    policy: SearchPolicy,
    registry: Registry,
    health: HealthTable,
    egress: Arc<EgressPolicy>,
    returned: ReturnedUrls,
    fetch_service: FetchService,
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
        let health = HealthTable::new(&registry, &limits);
        let egress_policy = EgressPolicy::from_texts(
            &config.egress.allow,
            &config.egress.deny,
            config.egress.allow_http,
            &config.fetch.ports,
        )
        .map_err(|problems| ConfigError { problems })?;
        let returned = ReturnedUrls::new(limits.strict_mode_urls);
        let egress_policy = Arc::new(egress_policy);
        let fetch_service = FetchService::new(
            &AgentClientSettings {
                user_agent: egress::standard_user_agent(&config.service.user_agent_contact),
                connect_timeout: std::time::Duration::from_millis(u64::from(
                    limits.fetch_connect_timeout_ms,
                )),
                timeout: std::time::Duration::from_millis(u64::from(limits.fetch_deadline_ms)),
                allow_http: config.egress.allow_http,
            },
            Arc::clone(&egress_policy),
            &limits,
        )
        .map_err(|error| ConfigError {
            problems: vec![format!(
                "fetch: the HTTP client could not be built: {error}"
            )],
        })?;
        Ok(Rypsok {
            config,
            limits,
            policy,
            registry,
            health,
            egress: egress_policy,
            returned,
            fetch_service,
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

    /// The health state of every engine (RFC 010).
    pub fn health(&self) -> &HealthTable {
        &self.health
    }

    /// The agent-directed egress policy (RFC 011).
    pub fn egress_policy(&self) -> &Arc<EgressPolicy> {
        &self.egress
    }

    /// The URLs `web_search` returned in this process, for strict mode
    /// (REQ-SEC-021).
    pub fn returned_urls(&self) -> &ReturnedUrls {
        &self.returned
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
            health: &self.health,
            returned: &self.returned,
            outbound: Arc::clone(&self.outbound),
            counters: &self.counters,
        };
        search::pipeline::run(&pipeline, validated, cancel).await
    }

    /// The counters.
    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    /// `web_fetch`: read the main content of one public page as Markdown
    /// (RFC 012). Content is external data.
    ///
    /// Profile (external design §3.2): no effect on local state; the URL
    /// leaves the host to the target site, plus a robots.txt read to the
    /// same site; at most LIM-F5 long; at most `max_chars` + LIM-S10 +
    /// LIM-F9 characters.
    pub async fn fetch(
        &self,
        req: FetchRequest,
        cancel: CancellationToken,
    ) -> Result<FetchResponse, CallError> {
        let pipeline = fetch::Pipeline {
            limits: &self.limits,
            policy: &self.egress,
            service: &self.fetch_service,
            returned: &self.returned,
            strict_mode: self.config.fetch.strict_mode,
            robots_txt: self.config.fetch.robots_txt,
            follow_cross_host_redirects: self.config.fetch.follow_cross_host_redirects,
            secret_detection: self.config.privacy.secret_detection,
            counters: &self.counters,
        };
        fetch::run(&pipeline, req, cancel).await
    }

    /// The fetch service: the agent client, the robots.txt cache and the
    /// permits (RFC 012).
    pub fn fetch_service(&self) -> &FetchService {
        &self.fetch_service
    }

    /// `engine_status`: report the state of each configured engine. For
    /// diagnosis, not for answering questions.
    ///
    /// Profile (external design §3.3): no effect on local state; nothing
    /// leaves the host; at most LIM-S13 long. A snapshot of the health
    /// table under a read lock: no I/O, no await (REQ-STATUS-001).
    pub fn status(&self) -> StatusResponse {
        StatusResponse {
            outcome: Outcome::Ok,
            notices: Vec::new(),
            engines: self.health.snapshot(),
        }
    }
}
