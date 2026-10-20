//! The stdio transport: standard output carries protocol messages only,
//! standard error carries logs, and the process ends on end of input
//! (REQ-MCP-002, REQ-MCP-007, REQ-OPS-002).

use rmcp::ServiceExt;
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
    let running = adapter
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| ServeError::Start(e.to_string()))?;
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
