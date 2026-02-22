mod request_handler;
mod request_stats;
mod tool_service;
mod repo_analyser;

use request_handler::RequestHandler;
use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<()> {
    // For logging purposes
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let service = RequestHandler::new().serve(stdio()).await?;
    service.waiting().await?;

    // Similar to void return type in Java
    Ok(())
}