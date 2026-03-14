mod request_handler;
mod request_stats;
mod tool_service;
mod repo_analyser;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use request_handler::RequestHandler;
use tool_service::ToolService;

use metrics::{counter, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use std::env;
use std::time::{Instant, Duration};
use std::thread;
use std::sync::Arc;
use std::convert::Infallible;

use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use hyper::header::CONTENT_TYPE;

#[tokio::main]
async fn main() -> Result<()> {

    // -----------------------------
    // Parse args
    // -----------------------------
    let args: Vec<String> = env::args().collect();
    let variant: &'static str = match args.get(4).map(|s| s.as_str()) {
        Some("baseline") => "baseline",
        Some("structured") => "structured",
        _ => "unknown",
    };
    let port = if variant == "baseline" { 9102 } else { 9103 };

    // -----------------------------
    // Install Prometheus exporter ONCE
    // -----------------------------
    let handle: PrometheusHandle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install recorder");

    eprintln!("Listening on port {}", port);    // Print all logs into stderr
    start_metrics_server(handle.clone(), port);

    // -----------------------------
    // BENCHMARK MODE
    // -----------------------------
    if args.get(1) == Some(&"--bench".to_string()) {

        let limit: usize = args[2].parse().unwrap();

        eprintln!("Created benchmark with upto {} tasks", limit);

        let start_time = Instant::now();
        let mut start_index: usize = 0;
        let service: Arc<ToolService> = Arc::new(ToolService::new());

        while start_time.elapsed() < Duration::from_millis(100_000) {

            let current_limit: usize = if start_index == limit {1} else {start_index + 1};
            start_index = current_limit;
            let service = service.clone();

            let start = Instant::now();

            let result = 
            if variant == "baseline" {
                service.baseline_tool_process(current_limit)
            } else {
                service.structured_tool_process(current_limit)
            };

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

// -----------------------------
// Make metrics response headers work with Prometheus.
// -----------------------------
async fn metrics_handler(
    req: Request<Body>,
    handle: PrometheusHandle,
) -> Result<Response<Body>, Infallible> {

    if req.uri().path() != "/metrics" {
        return Ok(Response::builder()
            .status(404)
            .body(Body::from("Not Found"))
            .unwrap());
    }

    let body = handle.render();

    Ok(Response::builder()
        .status(200)
        .header(
            CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8"
        )
        .body(Body::from(body))
        .unwrap())
}

fn start_metrics_server(handle: PrometheusHandle, port: u16) {
    tokio::spawn(async move {
        let addr = ([0, 0, 0, 0], port).into();

        let make_svc = make_service_fn(move |_conn| {
            let h = handle.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    metrics_handler(req, h.clone())
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        eprintln!("Metrics server on http://0.0.0.0:{}/metrics", port);

        if let Err(e) = server.await {
            eprintln!("metrics server error: {}", e);
        }
    });
}

