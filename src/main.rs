mod request_handler;
mod request_stats;
mod tool_service;
mod repo_analyser;

use request_handler::RequestHandler;
use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
// use tool_service::baseline_tool_process;

#[tokio::main]
async fn main() -> Result<()> {

    // let test = RequestHandler::new();

    // for i in 1..10 {
    //     println!("start");
    //     let stats = baseline_tool_process(10).await;

    //     let msg = format!(
    //         "TODOs found = {}. Scanned {} files. Unfinished tasks = {}",
    //         stats.todo_count, stats.file_count, stats.unfinished_tasks
    //     );

    //     dbg!(msg);
    // }

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