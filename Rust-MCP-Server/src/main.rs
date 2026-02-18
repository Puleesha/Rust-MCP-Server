async fn main() -> anyhow::Result<()> {
    // Initialize tracing to stderr (stdout is reserved for MCP protocol)
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    
    tracing::info!("Starting Weather MCP Server");
    
    let api_key = std::env::var("OPENWEATHER_API_KEY")
        .expect("OPENWEATHER_API_KEY environment variable required");
    
    let server = WeatherServer {
        api_key,
        cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
    };
    
    // Serve via stdio transport
    let service = server
        .serve(stdio())
        .await?;
    
    tracing::info!("Server running, waiting for requests");
    service.waiting().await?;
    
    Ok(())
}
