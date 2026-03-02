//! SQLRustGo Server - Standalone database server
//!
//! A standalone server process that accepts MySQL protocol connections
//! and executes SQL queries. Supports both sync and async modes.

use clap::Parser;
use sqlrustgo::network::config::{Config, ServerSection, DatabaseSection, ConnectionPoolSection, LoggingSection};
use sqlrustgo::network::async_server::ServerConfig as AsyncServerConfig;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "sqlrustgo-server")]
#[command(version = "1.0.0")]
#[command(about = "SQLRustGo Database Server", long_about = None)]
struct Args {
    /// Server bind address
    #[arg(short, long, default_value = "127.0.0.1:3306")]
    address: String,

    /// Database directory path
    #[arg(short, long, default_value = "./data")]
    database: String,

    /// Enable verbose logging
    #[arg(short, long, default_value = "false")]
    verbose: bool,

    /// Use async server mode
    #[arg(long, default_value = "false")]
    async_mode: bool,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Maximum connections
    #[arg(long)]
    max_connections: Option<usize>,
}

fn main() {
    let args = Args::parse();

    println!("╔════════════════════════════════════════════════╗");
    println!("║       SQLRustGo Server v1.0.0                 ║");
    println!("║  A Rust SQL-92 Database Server               ║");
    println!("╚════════════════════════════════════════════════╝");
    println!();

    // Load configuration from file if provided
    let config = if let Some(config_path) = &args.config {
        println!("Loading configuration from: {}", config_path);
        match Config::load(PathBuf::from(config_path).as_path()) {
            Ok(cfg) => {
                println!("Configuration loaded successfully");
                Some(cfg)
            }
            Err(e) => {
                eprintln!("Failed to load configuration: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Initialize the database system
    sqlrustgo::init();

    if args.async_mode {
        // Run async server
        run_async_server(&args, config.as_ref());
    } else {
        // Run sync server
        run_sync_server(&args, config.as_ref());
    }
}

fn run_sync_server(args: &Args, config: Option<&Config>) {
    let address = config
        .map(|c| c.server_address())
        .unwrap_or_else(|| args.address.clone());

    println!("Server address: {}", address);
    println!("Database path: {}", args.database);
    println!("Mode: Synchronous");
    println!();

    // Create execution engine
    let mut engine = sqlrustgo::ExecutionEngine::new();

    // Set up Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Start listening for connections
    match std::net::TcpListener::bind(&address) {
        Ok(listener) => {
            println!("Server listening on {}", address);
            println!("MySQL protocol compatible");
            println!("Press Ctrl-C to stop");
            println!();

            let mut connection_id: u32 = 1;

            for stream in listener.incoming() {
                if !running.load(Ordering::SeqCst) {
                    println!("Shutting down server...");
                    break;
                }

                match stream {
                    Ok(stream) => {
                        let conn_id = connection_id;
                        connection_id += 1;

                        println!("New connection #{}", conn_id);

                        if let Err(e) = handle_connection_sync(stream, conn_id, &mut engine, args.verbose) {
                            if args.verbose {
                                eprintln!("Client #{} error: {}", conn_id, e);
                            }
                        }

                        println!("Connection #{} closed", conn_id);
                    }
                    Err(e) => {
                        if args.verbose {
                            eprintln!("Connection error: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", address, e);
            std::process::exit(1);
        }
    }

    println!("Server stopped.");
}

fn run_async_server(args: &Args, config: Option<&Config>) {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let address = config
        .map(|c| c.server_address())
        .unwrap_or_else(|| args.address.clone());

    let max_connections = args.max_connections
        .or(config.map(|c| c.server.max_connections))
        .unwrap_or(100);

    println!("Server address: {}", address);
    println!("Database path: {}", args.database);
    println!("Mode: Asynchronous");
    println!("Max connections: {}", max_connections);
    println!();

    let server_config = AsyncServerConfig {
        address: address.clone(),
        database: args.database.clone(),
        max_connections,
        connection_timeout_secs: 30,
        verbose: args.verbose,
    };

    // Set up Ctrl-C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    println!("Starting async server...");
    println!("Server listening on {}", address);
    println!("Press Ctrl-C to stop");
    println!();

    runtime.block_on(async {
        use sqlrustgo::network::async_server::start_server_async;
        start_server_async(server_config).await.ok();
    });

    println!("Server stopped.");
}

/// Handle a client connection (sync)
fn handle_connection_sync(
    mut stream: std::net::TcpStream,
    connection_id: u32,
    engine: &mut sqlrustgo::ExecutionEngine,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use bytes::BufMut;

    stream.set_nonblocking(false)?;

    // Send greeting
    send_greeting_sync(&mut stream, connection_id)?;

    // Read and handle packets in a loop
    let mut sequence: u8 = 0;
    loop {
        match read_packet_sync(&mut stream) {
            Ok(Some((_seq, payload))) => {
                if payload.is_empty() {
                    continue;
                }

                let command = sqlrustgo::network::MySqlCommand::from(payload[0]);

                match command {
                    sqlrustgo::network::MySqlCommand::Quit => {
                        break;
                    }
                    sqlrustgo::network::MySqlCommand::Query => {
                        let query = String::from_utf8_lossy(&payload[1..]).to_string();
                        if verbose {
                            println!("[{}] Query: {}", connection_id, query);
                        }
                        execute_query_sync(&mut stream, &query, engine, &mut sequence)?;
                    }
                    sqlrustgo::network::MySqlCommand::Ping => {
                        send_ok_sync(&mut stream, "PONG", 0, &mut sequence)?;
                    }
                    _ => {
                        send_error_sync(&mut stream, 1047, "Command not supported", &mut sequence)?;
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}

const PACKET_HEADER_SIZE: usize = 4;

fn send_greeting_sync(
    stream: &mut std::net::TcpStream,
    connection_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use bytes::BufMut;

    let greeting = sqlrustgo::network::HandshakeV10::new(connection_id);
    let packet_data = greeting.to_bytes();

    let mut buf = bytes::BytesMut::new();
    buf.put_u8((packet_data.len() & 0xFF) as u8);
    buf.put_u8(((packet_data.len() >> 8) & 0xFF) as u8);
    buf.put_u8(((packet_data.len() >> 16) & 0xFF) as u8);
    buf.put_u8(0);
    buf.put_slice(&packet_data);

    stream.write_all(&buf)?;
    stream.flush()?;

    Ok(())
}

fn read_packet_sync(
    stream: &mut std::net::TcpStream,
) -> Result<Option<(u8, Vec<u8>)>, sqlrustgo::SqlError> {
    use sqlrustgo::SqlError;

    let mut header = [0u8; PACKET_HEADER_SIZE];

    match stream.read(&mut header) {
        Ok(0) => Ok(None),
        Ok(n) if n < PACKET_HEADER_SIZE => Err(SqlError::ProtocolError(
            "Incomplete packet header".to_string(),
        )),
        Ok(_) => {
            let payload_length =
                u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
            let sequence = header[3];

            let mut payload = vec![0u8; payload_length];
            let mut remaining = payload_length;
            let mut offset = 0;

            while remaining > 0 {
                match stream.read(&mut payload[offset..]) {
                    Ok(0) => {
                        return Err(SqlError::ProtocolError("Connection closed".to_string()));
                    }
                    Ok(n) => {
                        remaining -= n;
                        offset += n;
                    }
                    Err(e) => return Err(SqlError::IoError(e.to_string())),
                }
            }

            Ok(Some((sequence, payload)))
        }
        Err(e) => Err(SqlError::IoError(e.to_string())),
    }
}

fn execute_query_sync(
    stream: &mut std::net::TcpStream,
    query: &str,
    engine: &mut sqlrustgo::ExecutionEngine,
    sequence: &mut u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let trimmed = query.trim();

    match sqlrustgo::parse(trimmed) {
        Ok(statement) => {
            match engine.execute(statement) {
                Ok(result) => {
                    if result.rows.is_empty() {
                        let message = format!("{} row(s) affected", result.rows_affected);
                        send_ok_sync(stream, &message, result.rows_affected, sequence)?;
                    } else {
                        send_result_set_sync(stream, &result, sequence)?;
                    }
                }
                Err(e) => {
                    send_error_sync(stream, 1, &e.to_string(), sequence)?;
                }
            }
        }
        Err(e) => {
            send_error_sync(stream, 1064, &format!("Parse error: {}", e), sequence)?;
        }
    }

    Ok(())
}

fn send_ok_sync(
    stream: &mut std::net::TcpStream,
    message: &str,
    affected_rows: u64,
    sequence: &mut u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let packet = sqlrustgo::network::OkPacket::new(affected_rows, message);
    send_packet_sync(stream, &packet.to_bytes(), sequence)
}

fn send_error_sync(
    stream: &mut std::net::TcpStream,
    code: u16,
    message: &str,
    sequence: &mut u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let packet = sqlrustgo::network::ErrPacket::new(code, message);
    send_packet_sync(stream, &packet.to_bytes(), sequence)
}

fn send_result_set_sync(
    stream: &mut std::net::TcpStream,
    result: &sqlrustgo::ExecutionResult,
    sequence: &mut u8,
) -> Result<(), Box<dyn std::error::Error>> {
    use bytes::BufMut;

    let columns: Vec<&str> = if result.columns.is_empty() {
        vec!["column_0"]
    } else {
        result.columns.iter().map(|s| s.as_str()).collect()
    };

    // Column count
    {
        let mut buf = bytes::BytesMut::new();
        buf.put_u8(0x01);
        buf.put_u64_le(columns.len() as u64);
        send_packet_sync(stream, &buf, sequence)?;
    }

    // Column definitions
    for col in &columns {
        let mut col_buf = bytes::BytesMut::new();
        col_buf.put_slice(b"def\0");
        col_buf.put_slice(b"test\0");
        col_buf.put_slice(b"\0");
        col_buf.put_slice(b"test\0");
        col_buf.put_slice(b"\0");
        col_buf.put_slice(col.as_bytes());
        col_buf.put_u8(0);
        col_buf.put_u8(0x0c);
        col_buf.put_u32_le(256);
        col_buf.put_u8(0xfd);
        col_buf.put_u8(0x00);
        col_buf.put_u8(0x00);
        col_buf.put_u16_le(0x0000);
        send_packet_sync(stream, &col_buf, sequence)?;
    }

    // EOF (columns end)
    {
        let mut eof_buf = bytes::BytesMut::new();
        eof_buf.put_u8(0xfe);
        eof_buf.put_u16_le(0x0000);
        eof_buf.put_u16_le(0x0002);
        send_packet_sync(stream, &eof_buf, sequence)?;
    }

    // Row data
    for row in &result.rows {
        let mut row_buf = bytes::BytesMut::new();
        for value in row {
            match value {
                sqlrustgo::Value::Null => {
                    row_buf.put_u8(0xfb);
                }
                sqlrustgo::Value::Integer(i) => {
                    let s = i.to_string();
                    row_buf.put_u64_le(s.len() as u64);
                    row_buf.put_slice(s.as_bytes());
                }
                sqlrustgo::Value::Float(f) => {
                    let s = f.to_string();
                    row_buf.put_u64_le(s.len() as u64);
                    row_buf.put_slice(s.as_bytes());
                }
                sqlrustgo::Value::Text(s) => {
                    row_buf.put_u64_le(s.len() as u64);
                    row_buf.put_slice(s.as_bytes());
                }
                sqlrustgo::Value::Blob(b) => {
                    row_buf.put_u64_le(b.len() as u64);
                    row_buf.put_slice(b);
                }
                sqlrustgo::Value::Boolean(b) => {
                    let s = if *b { "1" } else { "0" };
                    row_buf.put_u64_le(1);
                    row_buf.put_slice(s.as_bytes());
                }
            }
        }
        send_packet_sync(stream, &row_buf, sequence)?;
    }

    // EOF (rows end)
    {
        let mut eof_buf = bytes::BytesMut::new();
        eof_buf.put_u8(0xfe);
        eof_buf.put_u16_le(0x0000);
        eof_buf.put_u16_le(0x000a);
        send_packet_sync(stream, &eof_buf, sequence)?;
    }

    Ok(())
}

fn send_packet_sync(
    stream: &mut std::net::TcpStream,
    data: &[u8],
    sequence: &mut u8,
) -> Result<(), Box<dyn std::error::Error>> {
    use bytes::BufMut;

    let mut buf = bytes::BytesMut::new();
    buf.put_u8((data.len() & 0xFF) as u8);
    buf.put_u8(((data.len() >> 8) & 0xFF) as u8);
    buf.put_u8(((data.len() >> 16) & 0xFF) as u8);
    buf.put_u8(*sequence);
    *sequence = sequence.wrapping_add(1);
    buf.put_slice(data);

    stream.write_all(&buf)?;
    stream.flush()?;

    Ok(())
}
