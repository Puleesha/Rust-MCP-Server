use rmcp::{
    handler::server::tool::{ToolCallContext, ToolCallResult},
    model::{ToolDefinition, ToolParams},
    ServiceExt, ServerHandler,
    transport::stdio,
};
use serde::Deserialize;
use std::sync::Arc;

// Define the request arguments schema
#[derive(Deserialize)]
struct AnalyzeArgs {
    limit: usize,
}

// Shared server state
#[derive(Clone)]
struct McpServer {
    // put shared state if needed
}

#[rmcp::tool]  // macro to generate tool definitions
impl McpServer {

    async fn baseline_analyzer(
        &self,
        ctx: ToolCallContext,
        params: ToolParams,
    ) -> ToolCallResult {

        // deserialize arguments
        let args: AnalyzeArgs = match params.parse_args() {
            Ok(v) => v,
            Err(_) => return ctx.error("invalid arguments").await,
        };

        // call your Rust tool logic
        let stats = crate::tooling::baseline_tool_process(args.limit).await;

        let result = format!(
            "TODOs={} files={} unfinished={}",
            stats.todo_count,
            stats.file_count,
            stats.unfinished_tasks
        );

        ctx.success_text(result).await
    }

    async fn structured_analyzer(
        &self,
        ctx: ToolCallContext,
        params: ToolParams,
    ) -> ToolCallResult {

        let args: AnalyzeArgs = match params.parse_args() {
            Ok(v) => v,
            Err(_) => return ctx.error("invalid arguments").await,
        };

        let stats = crate::tooling::structured_tool_process(args.limit).await;

        let result = format!(
            "TODOs={} files={} unfinished={}",
            stats.todo_count,
            stats.file_count,
            stats.unfinished_tasks
        );

        ctx.success_text(result).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // Initialize logger (stderr) so stdout remains clean for MCP JSON
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Rust MCP server starting");

    // Create the implementation instance
    let server = McpServer {};

    // stdio transport for MCP host (e.g. Claude Desktop)
    let transport = stdio();

    // Build and serve the MCP server
    let svc = server.serve(transport).await?;

    tracing::info!("Server ready; waiting for calls");

    svc.waiting().await?;

    Ok(())
}
