//! The command line: `serve`, `check` and `--version`, with the exit codes
//! of REQ-OPS-001: 0 on success, 1 on a configuration error, 2 on a usage
//! error (RFC 004). Nothing is ever written to standard output by the
//! command line (REQ-MCP-002); diagnostics go to standard error.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use rypsok_core::config::{Config, ConfigError, ConfigSource};
use rypsok_core::limits::defaults::SHUTDOWN_MAX_MS;
use rypsok_core::{CancellationToken, Rypsok, telemetry};

use crate::mcp::{Adapter, stdio};

/// Exit code of a successful run.
pub const EXIT_SUCCESS: u8 = 0;
/// Exit code of a configuration error.
pub const EXIT_CONFIGURATION: u8 = 1;
/// Exit code of a usage error.
pub const EXIT_USAGE: u8 = 2;

/// Top-level command line.
#[derive(Debug, Parser)]
#[command(
    name = "rypsok",
    version,
    about = "Headless meta-search service for AI agents"
)]
struct Cli {
    /// Path of the configuration file; `RYPSOK_CONFIG` is used when absent.
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

/// The subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Start the service on a transport.
    Serve {
        /// The transport to serve on.
        #[arg(long, value_enum, default_value_t = Transport::Stdio)]
        transport: Transport,
    },
    /// Validate the configuration and exit.
    Check,
}

/// Transports that `serve` accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Transport {
    /// The Model Context Protocol over standard input and output.
    Stdio,
    /// The Model Context Protocol over Streamable HTTP (milestone M4).
    Http,
}

/// Parses the arguments and runs the chosen command.
///
/// Returns the exit code instead of exiting, so that the function is
/// testable and the process ends through `main` alone.
pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => {
            // `--version` and `--help` are clap "errors" that exit with 0.
            let code = if error.use_stderr() {
                EXIT_USAGE
            } else {
                EXIT_SUCCESS
            };
            // clap writes help, version and usage errors itself.
            let _ = error.print();
            return ExitCode::from(code);
        }
    };
    match cli.command {
        Command::Serve {
            transport: Transport::Http,
        } => {
            diagnostic("rypsok: `serve --transport http` is not available in this release");
            ExitCode::from(EXIT_USAGE)
        }
        Command::Serve {
            transport: Transport::Stdio,
        } => serve(cli.config.as_deref()),
        Command::Check => check(cli.config.as_deref()),
    }
}

/// Writes one line to standard error. A failed write changes nothing.
fn diagnostic(line: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{line}");
}

fn report(error: &ConfigError) -> ExitCode {
    for problem in &error.problems {
        diagnostic(&format!("rypsok: configuration: {problem}"));
    }
    ExitCode::from(EXIT_CONFIGURATION)
}

fn describe(source: &ConfigSource) -> String {
    match source {
        ConfigSource::Flag(p) => format!("--config {}", p.display()),
        ConfigSource::Env(p) => format!("RYPSOK_CONFIG {}", p.display()),
        ConfigSource::Defaults => "defaults (no file)".to_owned(),
    }
}

/// `rypsok check`: validate the configuration and report the effective
/// settings on standard error (REQ-CFG-005).
fn check(flag: Option<&std::path::Path>) -> ExitCode {
    let (config, source) = match Config::locate(flag) {
        Ok(found) => found,
        Err(error) => return report(&error),
    };
    if let Err(error) = config.validate() {
        return report(&error);
    }
    let limits = config.limits();
    diagnostic(&format!(
        "rypsok: configuration from {} is valid",
        describe(&source)
    ));
    diagnostic(&format!(
        "rypsok: search: query_max_chars={} max_results={} (1..={}) deadline_ms={} engine_timeout_ms={} reserve_ms={} engines_per_request={} languages={} region={}",
        limits.query_max_chars,
        limits.max_results,
        limits.max_results_ceiling,
        limits.search_deadline_ms,
        limits.engine_timeout_ms,
        limits.search_reserve_ms,
        limits.engines_per_request,
        config
            .search
            .languages
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        config.search.region,
    ));
    diagnostic(&format!(
        "rypsok: tool rate limits per minute: web_search={} web_fetch={} engine_status={}; engine_status enabled={}; structured_content={}",
        limits.web_search_per_minute,
        limits.web_fetch_per_minute,
        limits.engine_status_per_minute,
        config.status.enabled,
        config.service.structured_content,
    ));
    for (id, table) in &config.engines {
        let credential = match table.secret_source() {
            None => "no credential needed".to_owned(),
            Some(source) => match source.resolve() {
                Ok((_, warning)) => warning.map_or("credential found".to_owned(), |w| {
                    format!("credential found; {w}")
                }),
                Err(error) => format!("inactive: {error}"),
            },
        };
        diagnostic(&format!(
            "rypsok: engine {id}: type={} enabled={} categories={} preference={} {credential}",
            table.kind,
            table.enabled,
            table
                .categories
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join(","),
            table.preference,
        ));
    }
    let categories = config
        .served_categories()
        .iter()
        .map(|c| c.as_str())
        .collect::<Vec<_>>()
        .join(",");
    diagnostic(&format!(
        "rypsok: categories served: {}",
        if categories.is_empty() {
            "none"
        } else {
            &categories
        }
    ));
    ExitCode::from(EXIT_SUCCESS)
}

/// `rypsok serve --transport stdio`: validate, build the service, serve
/// until end of input or a signal, exit within LIM-O1 (REQ-OPS-002).
fn serve(flag: Option<&std::path::Path>) -> ExitCode {
    let (config, source) = match Config::locate(flag) {
        Ok(found) => found,
        Err(error) => return report(&error),
    };
    if let Err(error) = config.validate() {
        return report(&error);
    }
    telemetry::init(config.logging.level);
    tracing::info!(event = "start", version = env!("CARGO_PKG_VERSION"), configuration = %describe(&source), "rypsok starting");
    if config.logging.log_queries {
        tracing::warn!(
            event = "log_queries",
            "logging.log_queries is on: query text appears in debug events"
        );
    }
    let service = match Rypsok::new(config) {
        Ok(service) => Arc::new(service),
        Err(error) => return report(&error),
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            diagnostic(&format!("rypsok: cannot start the runtime: {error}"));
            return ExitCode::from(EXIT_CONFIGURATION);
        }
    };
    let shutdown = CancellationToken::new();
    let outcome = runtime.block_on(async {
        let signals = shutdown.clone();
        tokio::spawn(async move {
            wait_for_signal().await;
            tracing::info!(event = "signal", "shutdown requested");
            signals.cancel();
        });
        stdio::serve(Adapter::new(service), shutdown).await
    });
    // Everything still running was cancelled through its token; the runtime
    // is dropped without waiting for the blocking stdin reader, so that the
    // process exits well within LIM-O1.
    runtime.shutdown_timeout(Duration::from_millis(u64::from(SHUTDOWN_MAX_MS) / 10));
    match outcome {
        Ok(()) => ExitCode::from(EXIT_SUCCESS),
        Err(error) => {
            tracing::error!(event = "serve_failed", error = %error, "serve ended with an error");
            ExitCode::from(EXIT_CONFIGURATION)
        }
    }
}

#[cfg(unix)]
async fn wait_for_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = match signal(SignalKind::terminate()) {
        Ok(term) => term,
        Err(_) => {
            let _ = tokio::signal::ctrl_c().await;
            return;
        }
    };
    tokio::select! {
        _ = term.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests;
