//! GMP Compliance Judge HTTP Server
//!
//! Standalone HTTP server that provides compliance judgment API
//! using LLM-powered analysis via Ollama.
//!
//! Usage:
//!   cargo run --bin compliance-server -- --ollama-url http://localhost:11434 --model qwen3:8b

use clap::Parser;
use sqlrustgo_gmp_api::compliance::{create_router, create_service_state};
use sqlrustgo_gmp_api::ollama_client::OllamaClientImpl;
use sqlrustgo_compliance_engine::judge::ComplianceJudge;
use std::sync::Arc;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "compliance-server")]
#[command(about = "GMP Compliance Judge HTTP Server")]
struct Args {
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    #[arg(long, default_value = "qwen3:8b-gmp")]
    model: String,

    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value = "8080")]
    port: u16,

    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&args.log_level));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    tracing::info!("GMP Compliance Judge Server");
    tracing::info!("Ollama URL: {}", args.ollama_url);
    tracing::info!("Model: {}", args.model);
    tracing::info!("Server: {}:{}", args.host, args.port);

    // Create Ollama client
    let ollama_client = OllamaClientImpl::new(&args.ollama_url);

    // Create ComplianceJudge with the Ollama client
    let judge = ComplianceJudge::new(Box::new(ollama_client), &args.model, None);

    // Create service state
    let state = create_service_state(judge);

    // Create router
    let app = create_router(Arc::new(state));

    // Start server
    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
