mod request_handler;

use request_handler::Handler;
use anyhow::Result;
use rmcp::{ transport::stdio, ServiceExt};
use tokio::io::{stdin, stdout};

// ==========================================================
// Main
// ==========================================================

#[tokio::main]
async fn main() -> Result<()> {

    // let transport = (stdin(), stdout());    // Create transport 

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let service = Handler::new().serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}