//! The stdio transport: standard output carries protocol messages only,
//! standard error carries logs, and the process ends on end of input
//! (REQ-MCP-002, REQ-MCP-007, REQ-OPS-002).

use rmcp::ServiceExt;
use rmcp::service::ServerInitializeError;
use tokio_util::sync::CancellationToken;

use super::Adapter;

/// Why serving ended abnormally.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The transport could not be started.
    #[error("the stdio transport could not start: {0}")]
    Start(String),
    /// The service task failed.
    #[error("the service task failed: {0}")]
    Join(String),
}

/// Serves until standard input reaches end of file or `shutdown` is
/// cancelled. Returns when the transport has closed; in-flight calls are
/// cancelled through their tokens.
pub async fn serve(adapter: Adapter, shutdown: CancellationToken) -> Result<(), ServeError> {
    let observer = adapter.clone();
    // The SDK answers a `server/discover` and stays open for either
    // lifecycle, so `serve` itself can wait for as long as the client says
    // nothing more: the shutdown is raced against it, and an end of input
    // after a discover is a normal end (a client that only looked).
    let started = tokio::select! {
        biased;
        () = shutdown.cancelled() => return Ok(()),
        started = adapter.serve(rmcp::transport::stdio()) => started,
    };
    let running = match started {
        Ok(running) => running,
        Err(ServerInitializeError::ConnectionClosed(_)) if observer.has_been_discovered() => {
            tracing::info!(
                event = "shutdown",
                reason = "input_closed",
                "transport closed"
            );
            return Ok(());
        }
        Err(error) => return Err(ServeError::Start(error.to_string())),
    };
    let stop = running.cancellation_token();
    tokio::spawn(async move {
        shutdown.cancelled().await;
        stop.cancel();
    });
    let reason = running
        .waiting()
        .await
        .map_err(|e| ServeError::Join(e.to_string()))?;
    tracing::info!(event = "shutdown", reason = ?reason, "transport closed");
    Ok(())
}

#[cfg(test)]
mod tests {
    /// REQ-MCP-002 (LIM-O3, 022a): the limits table says what the protocol
    /// library does. If a later rmcp changes its default line bound, this
    /// goes red and the table, the constant and the tests are revisited
    /// together.
    #[test]
    fn req_mcp_002_lim_o3_is_the_default_line_bound_of_the_protocol_library() {
        assert_eq!(
            rypsok_core::limits::defaults::STDIO_MAX_LINE_BYTES as usize,
            rmcp::transport::async_rw::DEFAULT_MAX_LINE_LENGTH
        );
    }
}
