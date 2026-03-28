mod request_handler;
mod request_stats;
mod tool_service;
mod repo_analyser;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use request_handler::RequestHandler;
use tool_service::ToolService;

use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;

use std::env;
use std::time::{Instant, Duration};
use std::thread;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {

    // -----------------------------
    // Parse args
    // -----------------------------
    let args: Vec<String> = env::args().collect();
    // Used string lig
    let variant: &str = match args.get(4).map(|s| s.as_str()) {
        Some("baseline") => "baseline",
        Some("structured") => "structured",
        _ => "unknown"
    };
    let port = if variant == "baseline" { 9102 } else { 9103 };

    // -----------------------------
    // Initialise Prometheus metrics server
    // -----------------------------
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], port))
        .install()
        .expect("failed to initialise Prometheus");

    eprintln!("Listening on port {}", port);    // Print all logs into stderr

    // -----------------------------
    // BENCHMARK MODE
    // -----------------------------
    // This line detects if a benchmark has been requested
    if args.get(1) == Some(&"--bench".to_string()) {

        let limit: usize = args[2].parse().unwrap();

        eprintln!("Created benchmark with upto {} tasks", limit);

        let start_time = Instant::now();
        let mut start_index: usize = 0;
        let tool_service: Arc<ToolService> = Arc::new(ToolService::new());

        while start_time.elapsed() < Duration::from_millis(100_000) {

            let current_limit: usize = if start_index == limit {1} else {start_index + 1};
            start_index = current_limit;
            let service = tool_service.clone();

            let start = Instant::now();

            let result = 
                if variant == "baseline" {
                    service.baseline_tool_process(current_limit)
                } else {
                    service.structured_tool_process(current_limit)
                };
            eprintln!("Active tasks: {}", result.unfinished_tasks);

            counter!("requests_total", 1, "variant" => variant);
            histogram!("todos_completed_per_request", result.todo_count as f64, "variant" => variant);
            histogram!("leaked_threads", result.unfinished_tasks as f64, "variant" => variant);
            histogram!("request_duration_seconds", start.elapsed().as_secs_f64(), "variant" => variant);

            thread::sleep(Duration::from_millis(100));
        }

        return Ok(());
    }

    // -----------------------------
    // NORMAL MCP SERVER MODE
    // -----------------------------

    // Log all messages to stderr to prevent result contamination 
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let request_handler = RequestHandler::new().serve(stdio()).await?;
    request_handler.waiting().await?;

    // Similar to void return type in Java
    Ok(())
}
