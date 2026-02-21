mod request_handler;
mod request_stats;

use request_handler::RequestHandler;
use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

// use tokio::io::{stdin, stdout};

// ==========================================================
// Main
// ==========================================================

#[tokio::main]
async fn main() -> Result<()> {

    // let transport = (stdin(), stdout());    // Create transport 

    // tracing_subscriber::fmt()
    //     .with_writer(std::io::stderr)
    //     .with_ansi(false)
    //     .init();

    let service = RequestHandler::new().serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}