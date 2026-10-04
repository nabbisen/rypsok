//! The Model Context Protocol adapter: tool definitions, schema
//! validation, and the mapping of core responses and errors to protocol
//! results (external design §2; RFC 003). The stdio transport lives in
//! [`stdio`]; the Streamable HTTP transport arrives in milestone M4.
//!
//! The SDK `rmcp` is pinned to an exact version in the workspace manifest
//! (REQ-ARCH-008) and is used only from this crate (REQ-ARCH-005).

#[allow(dead_code, reason = "the adapter is filled in milestone M1")]
pub mod stdio;
