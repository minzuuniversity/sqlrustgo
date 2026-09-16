//! sqlrustgo-shard-router — entry point binary.

use clap::Parser;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use sqlrustgo_shard_router::{run, ShardRouterConfig};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> std::process::ExitCode {
    let cfg = ShardRouterConfig::parse();

    let filter = EnvFilter::try_new(&cfg.log)
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    let cancel = CancellationToken::new();
    let cancel_signal = cancel.clone();

    tokio::spawn(async move {
        // SIGINT / SIGTERM -> graceful shutdown
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).expect("signal handler");
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal handler");
        tokio::select! {
            _ = sigint.recv() => tracing::info!(target: "shard_router", "SIGINT received, shutting down"),
            _ = sigterm.recv() => tracing::info!(target: "shard_router", "SIGTERM received, shutting down"),
        }
        cancel_signal.cancel();
    });

    match run(cfg, cancel).await {
        Ok(local) => {
            tracing::info!(target: "shard_router", listen = %local, "router exited cleanly");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!(target: "shard_router", error = %e, "router failed");
            std::process::ExitCode::FAILURE
        }
    }
}