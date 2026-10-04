//! The command line: `serve`, `check` and `--version`, with the exit codes
//! of REQ-OPS-001: 0 on success, 1 on a configuration error, 2 on a usage
//! error.
//!
//! Until milestone M1 delivers the configuration and the stdio adapter, the
//! commands report that they are not implemented and exit with code 2.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

/// Exit code of a successful run.
pub const EXIT_SUCCESS: u8 = 0;
/// Exit code of a configuration error.
#[allow(dead_code, reason = "configuration loading arrives in milestone M1")]
pub const EXIT_CONFIGURATION: u8 = 1;
/// Exit code of a usage error, and of a command that is not implemented yet.
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
    config: Option<std::path::PathBuf>,

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
        Command::Serve { transport } => {
            not_implemented(&format!("serve --transport {transport:?}"))
        }
        Command::Check => not_implemented("check"),
    }
}

/// Reports a command that a later milestone delivers.
fn not_implemented(what: &str) -> ExitCode {
    let mut stderr = std::io::stderr().lock();
    // Diagnostics go to standard error; standard output is reserved for the
    // protocol (REQ-MCP-002). A failed write to stderr changes nothing.
    let _ = writeln!(
        stderr,
        "rypsok: `{what}` is not implemented until milestone M1"
    );
    ExitCode::from(EXIT_USAGE)
}

#[cfg(test)]
mod tests;
