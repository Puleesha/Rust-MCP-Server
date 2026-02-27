mod request_handler;
mod request_stats;
mod tool_service;
mod repo_analyser;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use request_handler::RequestHandler;
// use tool_service::baseline_tool_process;

use metrics::{counter, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use std::env;
use std::time::Instant;

use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use hyper::header::CONTENT_TYPE;
use std::convert::Infallible;

#[tokio::main]
async fn main() -> Result<()> {

    // -----------------------------
    // Parse args
    // -----------------------------
    let args: Vec<String> = env::args().collect();
    let variant: &'static str = match args.get(6).map(|s| s.as_str()) {
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

    println!("Listening on port {}", port);
    start_metrics_server(handle.clone(), port);

    // -----------------------------
    // BENCHMARK MODE
    // -----------------------------
    if args.get(1) == Some(&"--bench".to_string()) {

        let n: usize = args[2].parse().unwrap();
        let limit: usize = args[4].parse().unwrap();

        let mut handles = Vec::new();

        for _ in 0..n {

            handles.push(tokio::spawn(async move {
                let start = Instant::now();

                let result = if variant == "baseline" {
                    tool_service::baseline_tool_process(limit).await
                } else {
                    tool_service::structured_tool_process(limit).await
                };

                counter!("requests_total", 1, "variant" => variant);

                histogram!("todos_completed_per_request", result.todo_count as f64, "variant" => variant);
                histogram!("todos_missed_per_request", (limit - result.todo_count) as f64, "variant" => variant);
                histogram!("leaked_threads", result.unfinished_tasks as f64, "variant" => variant);
                histogram!("request_duration_seconds", start.elapsed().as_secs_f64(), "variant" => variant);

                println!("Todo count = {}, leaks = {}", result.todo_count, result.unfinished_tasks);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        println!("Created benchmark with {} iterations", n);

        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        return Ok(());
    }

    // -----------------------------
    // NORMAL MCP SERVER MODE
    // -----------------------------
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

        println!("Metrics server on http://0.0.0.0:{}/metrics", port);

        if let Err(e) = server.await {
            eprintln!("metrics server error: {}", e);
        }
    });
}

