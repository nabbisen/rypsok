//! The Model Context Protocol adapter: tool definitions, argument
//! validation, the mapping of core responses and errors to protocol
//! results, per-tool rate limits and cancellation (external design §2;
//! RFC 003). The stdio transport lives in [`stdio`]; the Streamable HTTP
//! transport arrives in milestone M4.
//!
//! The SDK `rmcp` is pinned to an exact version in the workspace manifest
//! (REQ-ARCH-008) and is used only from this crate (REQ-ARCH-005).

pub mod rate;
pub mod stdio;
pub mod tools;

use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use rypsok_core::Rypsok;
use rypsok_core::model::{CallError, ToolError};
use serde::Serialize;

use rate::ToolLimiter;

/// The adapter: one per process, holding the service and the tool list.
#[derive(Debug)]
pub struct Adapter {
    service: Arc<Rypsok>,
    tools: Vec<Tool>,
    structured_content: bool,
    status_enabled: bool,
    search_limiter: ToolLimiter,
    status_limiter: ToolLimiter,
}

impl Adapter {
    /// Builds the adapter from a service; the tool list is fixed here and
    /// never varies per connection (REQ-MCP-009).
    pub fn new(service: Arc<Rypsok>) -> Self {
        let config = service.config();
        let limits = service.limits();
        let status_enabled = config.status.enabled;
        let tool_list = tools::tool_list(service.search_policy(), status_enabled);
        tracing::info!(
            event = "tools",
            count = tool_list.len(),
            definition_chars = tools::definitions_size(&tool_list),
            "tool definitions built"
        );
        Adapter {
            tools: tool_list,
            structured_content: config.service.structured_content,
            status_enabled,
            search_limiter: ToolLimiter::per_minute(limits.web_search_per_minute),
            status_limiter: ToolLimiter::per_minute(limits.engine_status_per_minute),
            service,
        }
    }

    /// A successful response: structured content plus the same JSON as
    /// text, or the text alone when the operator switched structured
    /// content off (REQ-MCP-011).
    fn render_ok<T: Serialize>(&self, response: &T) -> CallToolResult {
        let value = serde_json::to_value(response).expect("responses serialize");
        if self.structured_content {
            CallToolResult::structured(value)
        } else {
            CallToolResult::success(vec![ContentBlock::text(value.to_string())])
        }
    }

    /// A tool execution error with the error object (REQ-ERR-003).
    fn render_error(&self, error: ToolError) -> CallToolResult {
        let value = serde_json::to_value(error.envelope()).expect("errors serialize");
        if self.structured_content {
            CallToolResult::structured_error(value)
        } else {
            CallToolResult::error(vec![ContentBlock::text(value.to_string())])
        }
    }
}

impl ServerHandler for Adapter {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("rypsok", env!("CARGO_PKG_VERSION")))
            .with_instructions(tools::INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(self.tools.clone()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let result = match request.name.as_ref() {
            tools::WEB_SEARCH => {
                if let Err(retry_after_ms) = self.search_limiter.admit() {
                    tracing::warn!(
                        tool = tools::WEB_SEARCH,
                        reason = "rate_limited",
                        "call refused"
                    );
                    self.render_error(ToolError::rate_limited(retry_after_ms))
                } else {
                    match tools::parse_search_arguments(request.arguments.as_ref()) {
                        Err(error) => self.render_error(error),
                        Ok(req) => match self.service.search(req, context.ct.clone()).await {
                            Ok(response) => self.render_ok(&response),
                            Err(CallError::Tool(error)) => self.render_error(error),
                            // The SDK drops the response of a cancelled
                            // request; nothing is sent (REQ-MCP-007).
                            Err(CallError::Cancelled) => {
                                return Err(McpError::new(
                                    rmcp::model::ErrorCode(-32800),
                                    "request cancelled",
                                    None,
                                ));
                            }
                        },
                    }
                }
            }
            tools::ENGINE_STATUS if self.status_enabled => {
                if let Err(retry_after_ms) = self.status_limiter.admit() {
                    self.render_error(ToolError::rate_limited(retry_after_ms))
                } else if request.arguments.as_ref().is_some_and(|a| !a.is_empty()) {
                    self.render_error(ToolError::unknown_parameter(&[]))
                } else {
                    self.render_ok(&self.service.status())
                }
            }
            // An unknown tool is the one case that is a protocol error
            // (REQ-ERR-003). The name is caller text and is not repeated.
            _ => return Err(McpError::invalid_params("unknown tool", None)),
        };
        Ok(result.into())
    }
}
