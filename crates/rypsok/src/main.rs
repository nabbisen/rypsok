//! rypsok: a headless meta-search service for AI agents, served over the
//! Model Context Protocol.
//!
//! The binary parses the command line, loads the configuration and starts a
//! transport. All logic lives in the `rypsok-core` crate; this crate holds
//! only the command line and the protocol adapters.

mod cli;
mod mcp;

fn main() -> std::process::ExitCode {
    // First of all: a panic is one JSON line without its message (F-1).
    rypsok_core::telemetry::install_panic_hook();
    cli::run(std::env::args_os())
}
