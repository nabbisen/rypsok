//! The engine abstraction, the registry and the admission properties
//! (external design §11; RFC 006).
//!
//! A driver turns an engine request into one provider request and parses
//! the answer into raw results and classified errors (REQ-ENG-001,
//! REQ-ENG-006). The registry is built once from the configuration. Drivers
//! live in [`drivers`] behind Cargo features. Filled in milestone M1.

pub mod drivers;

#[cfg(test)]
mod tests;
