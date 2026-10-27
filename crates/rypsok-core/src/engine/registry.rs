//! The registry: one engine per configured table, built once, with its
//! driver, limiter and admission properties (REQ-ENG-002, REQ-ENG-003,
//! REQ-ENG-007, REQ-OPS-003).

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use super::drivers::{self, DriverContext};
use super::limiter::EngineLimiter;
use super::{Driver, EngineProperties};
use crate::config::Config;
use crate::egress;
use crate::model::{Category, EngineId};

/// One configured engine.
pub struct Engine {
    /// The identifier.
    pub id: EngineId,
    /// The driver type.
    pub kind: String,
    /// The categories it serves.
    pub categories: Vec<Category>,
    /// Lower first.
    pub preference: u32,
    /// The driver, when the engine is active.
    pub driver: Option<Box<dyn Driver>>,
    /// The limiter.
    pub limiter: EngineLimiter,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("active", &self.is_active())
            .finish()
    }
}

impl Engine {
    /// An engine with a driver, for hosts and tests that build engines
    /// themselves; its limiter follows the driver's properties.
    pub fn new(
        id: EngineId,
        categories: Vec<Category>,
        preference: u32,
        driver: Box<dyn Driver>,
    ) -> Self {
        let properties = driver.properties();
        let limiter = EngineLimiter::new(properties.requests_per_minute, properties.max_in_flight);
        Engine {
            id,
            kind: driver.kind().to_owned(),
            categories,
            preference,
            driver: Some(driver),
            limiter,
        }
    }

    /// Whether the engine has a driver and can be selected. The circuit
    /// (cooldown, probe) is the health table's, consulted by the pipeline.
    pub fn is_active(&self) -> bool {
        self.driver.is_some()
    }

    /// The admission properties, when the engine is active.
    pub fn properties(&self) -> Option<&EngineProperties> {
        self.driver.as_deref().map(Driver::properties)
    }
}

/// Every configured engine, ordered by preference and then identifier.
#[derive(Debug)]
pub struct Registry {
    engines: Vec<Arc<Engine>>,
    /// The parsing permits (LIM-C5) shared by the scraping drivers and
    /// the fetch pipeline (RFC 015).
    parsing: Arc<Semaphore>,
}

impl Registry {
    /// Builds the registry from a validated configuration. An engine whose
    /// credential cannot be read, or whose driver cannot be built, is
    /// registered as `disabled` with a log line; the process keeps running
    /// (REQ-OPS-003).
    pub fn build(config: &Config, base_override: Option<url::Url>, https_only: bool) -> Registry {
        let limits = config.limits();
        let parsing = Arc::new(Semaphore::new(limits.parsing_in_flight as usize));
        let context = DriverContext {
            user_agent: egress::standard_user_agent(&config.service.user_agent_contact),
            connect_timeout: Duration::from_millis(u64::from(limits.engine_connect_timeout_ms)),
            timeout: Duration::from_millis(u64::from(limits.engine_timeout_ms)),
            proxy: Some(config.egress.proxy.clone()).filter(|p| !p.is_empty()),
            https_only,
            base_override,
            parsing: Arc::clone(&parsing),
            html_max_nodes: limits.html_max_nodes,
        };
        let engines: Vec<Engine> = config
            .engines
            .iter()
            .map(|(id, table)| {
                let id = EngineId::new(id.as_str()).expect("validated identifier");
                let driver = if !table.enabled {
                    tracing::info!(event = "engine_disabled", engine = %id, "engine disabled by configuration");
                    None
                } else {
                    let secret = match table.secret_source().map(|s| s.resolve()) {
                        None if drivers::needs_credential(&table.kind) => {
                            tracing::warn!(event = "engine_inactive", engine = %id, reason = "no credential source", "engine inactive: credential not available");
                            Err(())
                        }
                        None => Ok(None),
                        Some(Ok((secret, warning))) => {
                            if let Some(warning) = warning {
                                tracing::warn!(event = "secret_file_permissions", engine = %id, "{warning}");
                            }
                            Ok(Some(secret))
                        }
                        Some(Err(error)) => {
                            tracing::warn!(event = "engine_inactive", engine = %id, reason = %error, "engine inactive: credential not available");
                            Err(())
                        }
                    };
                    secret.ok().and_then(|secret| match drivers::build(table, secret.as_ref(), &context) {
                        Ok(driver) => Some(driver),
                        Err(error) => {
                            tracing::warn!(event = "engine_inactive", engine = %id, reason = %error, "engine inactive: driver not built");
                            None
                        }
                    })
                };
                let (per_minute, max_in_flight) = driver
                    .as_deref()
                    .map(|d| (d.properties().requests_per_minute, d.properties().max_in_flight.min(limits.per_engine_in_flight)))
                    .unwrap_or((1, 1));
                Engine {
                    id,
                    kind: table.kind.clone(),
                    categories: table.categories.clone(),
                    preference: table.preference,
                    driver,
                    limiter: EngineLimiter::new(per_minute, max_in_flight),
                }
            })
            .collect();
        let mut registry = Registry::from_engines(engines);
        registry.parsing = parsing;
        registry
    }

    /// A registry of these engines, ordered by preference and identifier.
    pub fn from_engines(mut engines: Vec<Engine>) -> Registry {
        engines.sort_by(|a, b| {
            a.preference
                .cmp(&b.preference)
                .then_with(|| a.id.cmp(&b.id))
        });
        Registry {
            engines: engines.into_iter().map(Arc::new).collect(),
            parsing: Arc::new(Semaphore::new(
                crate::limits::defaults::PARSING_IN_FLIGHT as usize,
            )),
        }
    }

    /// The parsing permits (LIM-C5) the scraping drivers were built with;
    /// the fetch pipeline shares them (RFC 015).
    pub fn parsing(&self) -> &Arc<Semaphore> {
        &self.parsing
    }

    /// Every engine in preference order.
    pub fn engines(&self) -> &[Arc<Engine>] {
        &self.engines
    }

    /// The active engines of a category, in preference order, at most
    /// `limit` (REQ-ENG-007, LIM-S8).
    pub fn select(&self, category: Category, limit: usize) -> Vec<Arc<Engine>> {
        self.engines
            .iter()
            .filter(|e| e.is_active() && e.categories.contains(&category))
            .take(limit)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests;
