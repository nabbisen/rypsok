//! Health state per engine: the circuit and the latency window (RFC 010;
//! REQ-OBS-001, REQ-OBS-002, REQ-RES-004, REQ-RES-005, REQ-RES-009).
//!
//! The circuit is driven by real requests and the clock only: the pipeline
//! consults it when it selects engines and informs it when an engine
//! request completes. There is no timer task, no synthetic probe and no
//! persistence; the table is built with the registry and dropped with the
//! process. The lock is held for nanoseconds and never across an await.

use std::collections::{BTreeMap, VecDeque};
use std::sync::RwLock;
use std::time::Duration;

use tokio::time::Instant;

use crate::engine::EngineErrorClass;
use crate::engine::registry::Registry;
use crate::limits::Limits;
use crate::model::{EngineId, EngineState, EngineStatus};

/// The circuit of one engine (external design §3.3; RFC 010).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Circuit {
    /// Disabled by configuration or inactive for lack of a credential;
    /// never changes at run time.
    Disabled,
    /// In use, with the count of consecutive counted failures.
    Closed {
        /// Counted failures since the last success.
        consecutive_failures: u8,
    },
    /// Skipped until `until`; `cooldown` is the length of this cooldown.
    Open {
        /// When the cooldown elapses.
        until: Instant,
        /// The length of this cooldown, doubled on the next failure.
        cooldown: Duration,
    },
    /// The cooldown elapsed and one call carries the probe (LIM-B5).
    HalfOpen {
        /// The cooldown that just elapsed.
        cooldown: Duration,
        /// True from the transition on: the probing call owns the probe
        /// until it reports completion or cancellation.
        probe_in_flight: bool,
    },
}

impl Circuit {
    /// The state as `engine_status` reports it.
    pub fn state(&self) -> EngineState {
        match self {
            Circuit::Disabled => EngineState::Disabled,
            Circuit::Closed { .. } => EngineState::Closed,
            Circuit::Open { .. } => EngineState::Open,
            Circuit::HalfOpen { .. } => EngineState::HalfOpen,
        }
    }
}

/// What selection decided for an engine (REQ-RES-004, REQ-RES-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// The circuit is closed: an ordinary request.
    Allowed,
    /// The cooldown elapsed: this call carries the one probe (LIM-B5).
    Probe,
    /// In cooldown, or another call holds the probe: skipped with reason
    /// `cooldown`.
    Skipped,
}

/// How an engine request ended, as the circuit sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// The provider answered.
    Success,
    /// The request failed with a class; `retry_after` when the provider
    /// said how long to wait.
    Failure {
        /// The class.
        class: EngineErrorClass,
        /// The provider's `Retry-After`.
        retry_after: Option<Duration>,
    },
    /// The call ended before the request completed (cancellation, or a
    /// probe that never started).
    Cancelled,
}

/// A circuit transition, logged by the caller after the lock is released
/// (REQ-OBS-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// The engine.
    pub engine: EngineId,
    /// The new state.
    pub state: EngineState,
    /// The cooldown that starts or ended, when one is involved.
    pub cooldown: Option<Duration>,
    /// The consecutive counted failures at the transition.
    pub consecutive_failures: u8,
}

impl Transition {
    /// Logs the transition as one `info` event: `circuit_opened`,
    /// `circuit_half_open` or `circuit_closed`.
    pub fn log(&self) {
        let cooldown_ms = self.cooldown.map(|c| c.as_millis() as u64);
        match self.state {
            EngineState::Open => tracing::info!(
                event = "circuit_opened",
                engine = %self.engine,
                state = "open",
                cooldown_ms,
                consecutive_failures = self.consecutive_failures,
                "circuit opened"
            ),
            EngineState::HalfOpen => tracing::info!(
                event = "circuit_half_open",
                engine = %self.engine,
                state = "half_open",
                cooldown_ms,
                consecutive_failures = self.consecutive_failures,
                "circuit half-open: this call probes the engine"
            ),
            EngineState::Closed => tracing::info!(
                event = "circuit_closed",
                engine = %self.engine,
                state = "closed",
                cooldown_ms,
                consecutive_failures = self.consecutive_failures,
                "circuit closed"
            ),
            EngineState::Disabled => {}
        }
    }
}

/// What a completion asks the caller to do besides counting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Completed {
    /// A transition to log.
    pub transition: Option<Transition>,
    /// First `auth_failure` of this engine in this process: log a warning
    /// (the circuit never opens on it).
    pub warn_auth: bool,
}

/// Whether a failure class counts toward opening the circuit
/// (REQ-RES-004).
pub fn counted(class: EngineErrorClass) -> bool {
    matches!(
        class,
        EngineErrorClass::Timeout
            | EngineErrorClass::RateLimited
            | EngineErrorClass::Blocked
            | EngineErrorClass::ParseFailure
    )
}

#[derive(Debug)]
struct Health {
    circuit: Circuit,
    latencies: VecDeque<u32>,
    auth_warned: bool,
}

/// The circuit parameters (LIM-B1 to LIM-B3, LIM-S11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Policy {
    failures_to_open: u8,
    first_cooldown: Duration,
    max_cooldown: Duration,
    latency_window: usize,
}

/// The health state of every configured engine.
#[derive(Debug)]
pub struct HealthTable {
    policy: Policy,
    entries: RwLock<BTreeMap<EngineId, Health>>,
}

impl HealthTable {
    /// One entry per engine of the registry: `Disabled` for an engine
    /// without a driver, `Closed` otherwise.
    pub fn new(registry: &Registry, limits: &Limits) -> Self {
        Self::from_ids(
            registry
                .engines()
                .iter()
                .map(|e| (e.id.clone(), e.is_active())),
            limits,
        )
    }

    /// One entry per `(identifier, active)` pair.
    pub fn from_ids(ids: impl IntoIterator<Item = (EngineId, bool)>, limits: &Limits) -> Self {
        let entries = ids
            .into_iter()
            .map(|(id, active)| {
                let circuit = if active {
                    Circuit::Closed {
                        consecutive_failures: 0,
                    }
                } else {
                    Circuit::Disabled
                };
                (
                    id,
                    Health {
                        circuit,
                        latencies: VecDeque::new(),
                        auth_warned: false,
                    },
                )
            })
            .collect();
        HealthTable {
            policy: Policy {
                failures_to_open: limits.failures_to_open.max(1),
                first_cooldown: Duration::from_secs(u64::from(limits.first_cooldown_s)),
                max_cooldown: Duration::from_secs(u64::from(limits.max_cooldown_s)),
                latency_window: usize::from(limits.latency_window.max(1)),
            },
            entries: RwLock::new(entries),
        }
    }

    /// Consults the circuit at selection. An elapsed cooldown turns the
    /// circuit half-open under the lock, so that exactly one call carries
    /// the probe (LIM-B5).
    pub fn admit(&self, id: &EngineId) -> (Admission, Option<Transition>) {
        let mut entries = self.entries.write().expect("health lock");
        let Some(health) = entries.get_mut(id) else {
            return (Admission::Skipped, None);
        };
        match health.circuit {
            Circuit::Disabled | Circuit::HalfOpen { .. } => (Admission::Skipped, None),
            Circuit::Closed { .. } => (Admission::Allowed, None),
            Circuit::Open { until, cooldown } => {
                if Instant::now() < until {
                    return (Admission::Skipped, None);
                }
                health.circuit = Circuit::HalfOpen {
                    cooldown,
                    probe_in_flight: true,
                };
                (
                    Admission::Probe,
                    Some(Transition {
                        engine: id.clone(),
                        state: EngineState::HalfOpen,
                        cooldown: Some(cooldown),
                        consecutive_failures: self.policy.failures_to_open,
                    }),
                )
            }
        }
    }

    /// Informs the circuit that a request (or a probe) completed.
    pub fn complete(&self, id: &EngineId, completion: Completion) -> Completed {
        let mut entries = self.entries.write().expect("health lock");
        let Some(health) = entries.get_mut(id) else {
            return Completed::default();
        };
        let mut completed = Completed::default();
        if let Completion::Failure {
            class: EngineErrorClass::AuthFailure,
            ..
        } = completion
            && !health.auth_warned
        {
            health.auth_warned = true;
            completed.warn_auth = true;
        }
        let policy = self.policy;
        let open = |cooldown: Duration, retry_after: Option<Duration>, failures: u8| {
            let cooldown = retry_after
                .map_or(cooldown, |r| r.max(cooldown))
                .min(policy.max_cooldown);
            (
                Circuit::Open {
                    until: Instant::now() + cooldown,
                    cooldown,
                },
                Transition {
                    engine: id.clone(),
                    state: EngineState::Open,
                    cooldown: Some(cooldown),
                    consecutive_failures: failures,
                },
            )
        };
        let next = match (health.circuit, completion) {
            (Circuit::Disabled, _) | (Circuit::Open { .. }, _) => None,
            (Circuit::Closed { .. }, Completion::Success) => Some((
                Circuit::Closed {
                    consecutive_failures: 0,
                },
                None,
            )),
            (Circuit::Closed { .. }, Completion::Cancelled) => None,
            (
                Circuit::Closed {
                    consecutive_failures,
                },
                Completion::Failure { class, retry_after },
            ) => {
                if !counted(class) {
                    None
                } else {
                    let failures = consecutive_failures.saturating_add(1);
                    if failures >= policy.failures_to_open {
                        let (circuit, transition) =
                            open(policy.first_cooldown, retry_after, failures);
                        Some((circuit, Some(transition)))
                    } else {
                        Some((
                            Circuit::Closed {
                                consecutive_failures: failures,
                            },
                            None,
                        ))
                    }
                }
            }
            (Circuit::HalfOpen { .. }, Completion::Success) => Some((
                Circuit::Closed {
                    consecutive_failures: 0,
                },
                Some(Transition {
                    engine: id.clone(),
                    state: EngineState::Closed,
                    cooldown: None,
                    consecutive_failures: 0,
                }),
            )),
            (Circuit::HalfOpen { cooldown, .. }, Completion::Failure { class, retry_after })
                if counted(class) =>
            {
                let doubled = cooldown.saturating_mul(2).min(policy.max_cooldown);
                let (circuit, transition) = open(
                    doubled,
                    retry_after,
                    policy.failures_to_open.saturating_add(1),
                );
                Some((circuit, Some(transition)))
            }
            (Circuit::HalfOpen { cooldown, .. }, _) => {
                // An uncounted failure or a cancelled probe: open again with
                // the same cooldown; the probe gave no verdict.
                let (circuit, transition) = open(cooldown, None, policy.failures_to_open);
                Some((circuit, Some(transition)))
            }
        };
        if let Some((circuit, transition)) = next {
            health.circuit = circuit;
            completed.transition = transition;
        }
        completed
    }

    /// Records the duration of a request that received a response,
    /// successful or classified (REQ-OBS-002). Timeouts and skips are not
    /// recorded.
    pub fn record_latency(&self, id: &EngineId, latency: Duration) {
        let mut entries = self.entries.write().expect("health lock");
        if let Some(health) = entries.get_mut(id) {
            if health.latencies.len() >= self.policy.latency_window {
                health.latencies.pop_front();
            }
            health
                .latencies
                .push_back(u32::try_from(latency.as_millis()).unwrap_or(u32::MAX));
        }
    }

    /// The circuit of an engine, for tests and hosts.
    pub fn circuit(&self, id: &EngineId) -> Option<Circuit> {
        self.entries
            .read()
            .expect("health lock")
            .get(id)
            .map(|h| h.circuit)
    }

    /// The snapshot `engine_status` returns: identifier, state and median
    /// latency per engine, in configuration order (REQ-STATUS-001). No
    /// I/O, no await.
    pub fn snapshot(&self) -> Vec<EngineStatus> {
        self.entries
            .read()
            .expect("health lock")
            .iter()
            .map(|(id, health)| EngineStatus {
                id: id.clone(),
                state: health.circuit.state(),
                latency_ms: median(&health.latencies),
            })
            .collect()
    }
}

/// The integer median: the middle value, or the mean of the two middle
/// values rounded down; `None` of an empty window.
fn median(latencies: &VecDeque<u32>) -> Option<u32> {
    if latencies.is_empty() {
        return None;
    }
    let mut sorted: Vec<u32> = latencies.iter().copied().collect();
    sorted.sort_unstable();
    let n = sorted.len();
    Some(if n % 2 == 1 {
        sorted[n / 2]
    } else {
        ((u64::from(sorted[n / 2 - 1]) + u64::from(sorted[n / 2])) / 2) as u32
    })
}

#[cfg(test)]
mod tests;
