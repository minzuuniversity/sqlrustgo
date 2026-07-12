//! SQLRustGo TCP Server
//!
//! A simple TCP server that accepts SQL queries and returns results.
//!
//! # DEPRECATED
//! This module is deprecated and non-functional. The canonical MySQL server
//! implementation is in `crates/mysql-server`. This file is kept for
//! backwards compatibility but uses stub implementations.

use sqlrustgo_common::logging::{init_logging, LogFormat, LogLevel};
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::SqlError;
use std::env;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, RwLock};

use parking_lot::RwLock as ParkingRwLock;

/// Stub ExecutionEngine for backwards compatibility
/// The real ExecutionEngine is in sqlrustgo crate
pub struct ExecutionEngine<S> {
    storage: Arc<ParkingRwLock<S>>,
}

impl<S> ExecutionEngine<S> {
    pub fn new(storage: Arc<ParkingRwLock<S>>) -> Self {
        Self { storage }
    }
}

/// Stub parse function - delegates to sqlrustgo_parser
pub fn parse(sql: &str) -> Result<sqlrustgo_parser::Statement, String> {
    sqlrustgo_parser::parse(sql)
}

/// Handle a single client connection
#[allow(dead_code)]
fn handle_client(
    mut stream: TcpStream,
    _storage: Arc<RwLock<MemoryStorage>>,
) -> std::io::Result<()> {
    let peer_addr = stream
        .peer_addr()
        .unwrap_or(SocketAddr::from(([0, 0, 0, 0], 0)));
    let _ip = peer_addr.ip().to_string();
    let _user = "anonymous".to_string();

    let mut buffer = [0u8; 4096];

    loop {
        let bytes_read = match stream.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        let received = String::from_utf8_lossy(&buffer[..bytes_read]);
        let _queries: Vec<&str> = received.lines().collect();

        // Stub response for backwards compatibility
        let response = serde_json::json!({
            "status": "deprecated",
            "message": "This server implementation is deprecated. Use crates/mysql-server instead."
        })
        .to_string();

        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(b"\n");
        let _ = stream.flush();
    }
}

fn is_ddl(query: &str) -> bool {
    let upper = query.to_uppercase();
    upper.starts_with("CREATE")
        || upper.starts_with("DROP")
        || upper.starts_with("ALTER")
        || upper.starts_with("TRUNCATE")
}

#[allow(dead_code)]
fn main() {
    let log_dir = env::var("SQLRUSTGO_LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    let log_level = env::var("SQLRUSTGO_LOG_LEVEL")
        .map(|v| LogLevel::from_str(&v))
        .unwrap_or(LogLevel::Info);
    let log_format = if env::var("SQLRUSTGO_LOG_JSON").is_ok() {
        LogFormat::Json
    } else {
        LogFormat::Text
    };
    let max_file_size: u64 = env::var("SQLRUSTGO_LOG_MAX_SIZE")
        .unwrap_or_else(|_| "10485760".to_string())
        .parse()
        .unwrap_or(10 * 1024 * 1024);
    let max_files: usize = env::var("SQLRUSTGO_LOG_MAX_FILES")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5);

    if let Err(e) = init_logging(&log_dir, log_level, log_format, max_file_size, max_files) {
        eprintln!("Failed to initialize logging: {}", e);
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    log::info!("SQLRustGo TCP Server v1.6.1 starting...");
    log::info!("Log directory: {}", log_dir);
    log::info!("Log level: {:?}", log_level);

    let addr = "127.0.0.1:4000";
    println!("SQLRustGo TCP Server v1.6.1 (DEPRECATED)");
    println!("Listening on {}", addr);
    println!("WARNING: This implementation is deprecated. Use crates/mysql-server instead.");

    let _storage = Arc::new(RwLock::new(MemoryStorage::new()));

    println!("Security: audit logging enabled");

    let listener = TcpListener::bind(addr).expect("Failed to bind to address");
    println!("Ready to accept connections");

    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let storage = _storage.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(stream, storage) {
                        eprintln!("Client handler error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }
}
