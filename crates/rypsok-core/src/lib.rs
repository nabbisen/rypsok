//! Core library of rypsok, a headless meta-search service for AI agents.
//!
//! This crate holds the models, the search and fetch pipelines, the engine
//! drivers and the policies. It depends on no protocol SDK: the Model
//! Context Protocol adapters and the command line live in the `rypsok`
//! binary crate, and CI fails when a protocol crate appears in this crate's
//! dependency tree (REQ-ARCH-005).
//!
//! The public interface is the three application services, `search`,
//! `fetch` and `status`, and their request and response types
//! (REQ-ARCH-007). Everything else is private to the crate.
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
