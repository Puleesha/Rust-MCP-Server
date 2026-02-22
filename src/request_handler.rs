use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    ErrorData as McpError,
    ServerHandler
};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use std::result::Result;
use crate::tool_service::baseline_tool_process;
use crate::tool_service::structured_tool_process;

// ==========================================================
// Tool input schema
// ==========================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolSchema {
    pub limit: usize,
}

// ==========================================================
// MCP server and tool definitions
// ==========================================================

#[derive(Clone)]
pub struct RequestHandler {
    tool_router: ToolRouter<Self>
}

#[tool_router]
impl RequestHandler {

    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(name = "rust_baseline_analyzer", description = "Analyze repo using unstructured concurrency")]
    async fn rust_baseline_analyzer(&self, params: Parameters<ToolSchema>) -> Result<CallToolResult, McpError> {

        let stats = baseline_tool_process(params.0.limit).await;

        let msg = format!(
            "TODOs found = {}. Scanned {} files. Unfinished tasks = {}",
            stats.todo_count, stats.file_count, stats.unfinished_tasks
        );

        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(name = "rust_structured_analyzer", description = "Analyze repo using structured concurrency")]
    async fn rust_structured_analyzer(&self, params: Parameters<ToolSchema>) -> Result<CallToolResult, McpError> {

        let stats = structured_tool_process(params.0.limit).await;

        let msg = format!(
            "TODOs found = {}. Scanned {} files. Unfinished tasks = {}",
            stats.todo_count, stats.file_count, stats.unfinished_tasks
        );

        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }
}

#[tool_handler]
impl ServerHandler for RequestHandler {

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Rust MCP Server".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}