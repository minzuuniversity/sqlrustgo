//! GMP Server - MySQL Protocol Wrapper
//!
//! Standalone MySQL Wire Protocol server that wraps sqlrustgo-mysql-server
//! for the GMP-Platform deployment. Accepts standard MySQL client connections
//! and provides GMP data processing capabilities.
//!
//! Usage:
//!   gmp-server --host 0.0.0.0 --port 3306
//!   gmp-server --config config/default.toml

use clap::Parser;
use sqlrustgo_mysql_server::run_server;
use std::fs;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "gmp-server")]
#[command(about = "GMP MySQL Wire Protocol Server")]
struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(long, default_value = "3306")]
    port: u16,

    #[arg(
        long,
        default_value = "8080",
        help = "HTTP monitoring port (0 to disable)"
    )]
    monitoring_port: u16,

    #[arg(long, default_value = "info")]
    log_level: String,

    #[arg(long, help = "Custom config file path (overrides default)")]
    config: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Try to load custom config if provided
    if let Some(config_path) = &args.config {
        if let Ok(config_content) = fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<serde_json::Value>(&config_content) {
                println!("Loaded config from: {}", config_path);
                println!("Config: {:?}", config);
            }
        }
    }

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&args.log_level));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    tracing::info!("GMP Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("MySQL protocol server for GMP-Platform");
    tracing::info!("Accepts standard MySQL client connections");
    tracing::info!("Host: {}:{}", args.host, args.port);

    let _monitoring_port = if args.monitoring_port == 0 {
        None
    } else {
        Some(args.monitoring_port)
    };

    run_server(&args.host, args.port)?;

    Ok(())
}
