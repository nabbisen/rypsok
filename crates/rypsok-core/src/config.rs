//! Configuration: the TOML schema, its validation and secrets by reference
//! (external design §13.2; RFC 004).
//!
//! One file, located by `--config` or `RYPSOK_CONFIG`, with documented
//! defaults for every key (REQ-CFG-001). Unknown keys are refused
//! (REQ-CFG-002). Filled in milestone M1.

#[cfg(test)]
mod tests;
