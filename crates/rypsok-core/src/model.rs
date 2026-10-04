//! Request, result, outcome, notice and error types of the external
//! contract (external design §3 and §4; RFC 003).
//!
//! Every structure that carries external text keeps it in a field distinct
//! from every field rypsok asserts (REQ-CTX-002). Vocabularies are closed
//! enumerations (REQ-SEC-016). Filled in milestone M1.

#[cfg(test)]
mod tests;
