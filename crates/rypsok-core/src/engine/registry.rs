//! The registry: one engine per configured table, built once, with its
//! driver, limiter and admission properties (REQ-ENG-002, REQ-ENG-003,
//! REQ-ENG-007, REQ-OPS-003).

use std::sync::Mutex;
use std::time::Duration;

use super::drivers::{self, DriverContext};
use super::limiter::EngineLimiter;
use super::{Driver, EngineProperties};
use crate::config::Config;
use crate::egress;
use crate::model::{Category, EngineId, EngineState};

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
    state: Mutex<EngineState>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("state", &self.state())
            .finish()
    }
}

impl Engine {
    /// The circuit state.
    pub fn state(&self) -> EngineState {
        *self.state.lock().expect("engine state lock")
    }

    /// Whether the engine can be selected.
    pub fn is_active(&self) -> bool {
        self.driver.is_some() && self.state() == EngineState::Closed
    }

    /// The admission properties, when the engine is active.
    pub fn properties(&self) -> Option<&EngineProperties> {
        self.driver.as_deref().map(Driver::properties)
    }
}

/// Every configured engine, ordered by preference and then identifier.
#[derive(Debug)]
pub struct Registry {
    engines: Vec<Engine>,
}

impl Registry {
    /// Builds the registry from a validated configuration. An engine whose
    /// credential cannot be read, or whose driver cannot be built, is
    /// registered as `disabled` with a log line; the process keeps running
    /// (REQ-OPS-003).
    pub fn build(config: &Config, base_override: Option<url::Url>, https_only: bool) -> Registry {
        let limits = config.limits();
        let context = DriverContext {
            user_agent: egress::standard_user_agent(&config.service.user_agent_contact),
            connect_timeout: Duration::from_millis(u64::from(limits.engine_connect_timeout_ms)),
            timeout: Duration::from_millis(u64::from(limits.engine_timeout_ms)),
            proxy: Some(config.egress.proxy.clone()).filter(|p| !p.is_empty()),
            https_only,
            base_override,
        };
        let mut engines: Vec<Engine> = config
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
                let state = if driver.is_some() { EngineState::Closed } else { EngineState::Disabled };
                Engine {
                    id,
                    kind: table.kind.clone(),
                    categories: table.categories.clone(),
                    preference: table.preference,
                    driver,
                    limiter: EngineLimiter::new(per_minute, max_in_flight),
                    state: Mutex::new(state),
                }
            })
            .collect();
        engines.sort_by(|a, b| {
            a.preference
                .cmp(&b.preference)
                .then_with(|| a.id.cmp(&b.id))
        });
        Registry { engines }
    }

    /// Every engine in preference order.
    pub fn engines(&self) -> &[Engine] {
        &self.engines
    }

    /// The active engines of a category, in preference order, at most
    /// `limit` (REQ-ENG-007, LIM-S8).
    pub fn select(&self, category: Category, limit: usize) -> Vec<&Engine> {
        self.engines
            .iter()
            .filter(|e| e.is_active() && e.categories.contains(&category))
            .take(limit)
            .collect()
    }
}

#[cfg(test)]
mod tests;
