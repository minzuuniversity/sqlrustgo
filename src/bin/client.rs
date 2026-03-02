//! SQLRustGo Client - Standalone database client
//!
//! A standalone client process that connects to the SQLRustGo server
//! and executes SQL queries interactively or in single-query mode.

use clap::Parser;
use std::io::{self, Read, Write};
use std::net::TcpStream;

/// Client command-line arguments
#[derive(Parser, Debug)]
#[command(name = "sqlrustgo-client")]
#[command(version = "1.0.0")]
#[command(about = "SQLRustGo Database Client", long_about = None)]
struct Args {
    /// Server address to connect to
    #[arg(long, default_value = "127.0.0.1:3306")]
    host: String,

    /// SQL query to execute (if not provided, enters interactive mode)
    #[arg(short, long)]
    query: Option<String>,

    /// Execute query and exit immediately
    #[arg(short, long, default_value = "false")]
    execute: bool,
}

fn main() {
    let args = Args::parse();

    // Connect to the server
    match TcpStream::connect(&args.host) {
        Ok(mut stream) => {
            println!("Connected to {}", args.host);

            // Read greeting from server
            if let Err(e) = read_greeting(&mut stream) {
                eprintln!("Failed to read server greeting: {}", e);
                std::process::exit(1);
            }

            // If query is provided via --query or --execute, run it and exit
            if let Some(query) = args.query {
                match execute_query(&mut stream, &query) {
                    Ok(response) => {
                        print_response(&response);
                        if args.execute {
                            // Send quit and exit
                            let _ = send_quit(&mut stream);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                // Interactive mode
                interactive_mode(&mut stream);
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to {}: {}", args.host, e);
            std::process::exit(1);
        }
    }
}

/// Interactive REPL mode
fn interactive_mode(stream: &mut TcpStream) {
    println!();
    println!("SQLRustGo Client v1.0.0");
    println!("Type 'exit' or 'quit' to exit.");
    println!("Type '.help' for commands.");
    println!();

    let mut input = String::new();

    loop {
        print!("sqlrustgo> ");
        if let Err(e) = io::stdout().flush() {
            eprintln!("Warning: failed to flush stdout: {}", e);
        }

        input.clear();
        match io::stdin().read_line(&mut input) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }

                // Handle special commands
                if input.starts_with('.') {
                    match handle_command(input, stream) {
                        Ok(Some(response)) => {
                            print_response(&response);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            eprintln!("Error: {}", e);
                        }
                    }
                    continue;
                }

                // Handle exit
                if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                    let _ = send_quit(stream);
                    break;
                }

                // Execute query
                match execute_query(stream, input) {
                    Ok(response) => {
                        print_response(&response);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        }
    }

    println!("Goodbye!");
}

/// Handle special commands
fn handle_command(input: &str, stream: &mut TcpStream) -> Result<Option<String>, String> {
    match input {
        ".help" => {
            println!("Available commands:");
            println!("  .help      Show this help message");
            println!("  .tables    List all tables");
            println!("  .quit      Exit the client");
            println!("  .exit      Exit the client");
            Ok(None)
        }
        ".tables" => {
            // Execute SHOW TABLES query
            match execute_query(stream, "SHOW TABLES") {
                Ok(response) => Ok(Some(response)),
                Err(e) => Err(e.to_string()),
            }
        }
        ".quit" | ".exit" => {
            let _ = send_quit(stream);
            std::process::exit(0);
        }
        _ => Err("Unknown command. Type .help for available commands.".to_string()),
    }
}

/// Read and discard server greeting
fn read_greeting(stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    use sqlrustgo::SqlError;

    // Read packet header
    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;

    let payload_length =
        u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;

    // Read payload
    let mut payload = vec![0u8; payload_length];
    stream.read_exact(&mut payload)?;

    // Verify it's a handshake packet (first byte should be 0x0a = protocol version)
    if !payload.is_empty() && payload[0] != 0x0a {
        return Err(Box::new(SqlError::ProtocolError(
            "Invalid server greeting".to_string(),
        )));
    }

    Ok(())
}

/// Send QUIT command
fn send_quit(stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    // Build MySQL packet: length (3 bytes) + sequence (1 byte) + command
    let mut packet = vec![0x01, 0x00, 0x00, 0x00, 0x01]; // length=1, seq=0, cmd=Quit(0x01)
    stream.write_all(&mut packet)?;
    stream.flush()?;
    Ok(())
}

/// Execute a query and get response
fn execute_query(stream: &mut TcpStream, query: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Build MySQL packet
    let query_bytes = query.as_bytes();
    let packet_len = query_bytes.len() + 1; // +1 for command byte

    let mut packet = Vec::with_capacity(4 + packet_len);
    // Length (3 bytes)
    packet.push((packet_len & 0xFF) as u8);
    packet.push(((packet_len >> 8) & 0xFF) as u8);
    packet.push(((packet_len >> 16) & 0xFF) as u8);
    // Sequence (1 byte)
    packet.push(0);
    // Command: Query (0x03)
    packet.push(0x03);
    // Query string
    packet.extend_from_slice(query_bytes);

    stream.write_all(&packet)?;
    stream.flush()?;

    // Read response
    read_response(stream)
}

/// Read response from server
fn read_response(stream: &mut TcpStream) -> Result<String, Box<dyn std::error::Error>> {
    // Read packet header
    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;

    let payload_length =
        u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;

    // Read payload
    let mut payload = vec![0u8; payload_length];
    stream.read_exact(&mut payload)?;

    // Check packet type
    if payload.is_empty() {
        return Ok("Empty response".to_string());
    }

    let packet_type = payload[0];

    match packet_type {
        // OK packet
        0x00 => {
            let message = parse_ok_packet(&payload);
            Ok(format!("OK - {}", message))
        }
        // Error packet
        0xff => {
            let (code, message) = parse_error_packet(&payload);
            Ok(format!("Error {}: {}", code, message))
        }
        // Result set (column count)
        _ => {
            // For now, just read and discard the rest of the result set
            let mut response = format!("Result set started (type: {})\n", packet_type);

            // Read column definitions
            // This is simplified - in a real client we'd parse the full protocol
            response.push_str(&format!("Payload: {} bytes\n", payload_length));

            Ok(response)
        }
    }
}

/// Parse OK packet
fn parse_ok_packet(payload: &[u8]) -> String {
    // Skip header byte
    if payload.len() < 9 {
        return "OK".to_string();
    }

    // Skip affected_rows (1-9 bytes, length-encoded)
    // Skip last_insert_id (1-9 bytes, length-encoded)
    // Skip status_flags (2 bytes)
    // Skip warnings (2 bytes)

    // Try to get message
    let mut offset = 9;
    while offset < payload.len() {
        let len = payload[offset] as usize;
        offset += 1;
        if offset + len > payload.len() {
            break;
        }
        if len == 0 {
            break;
        }
        let msg = String::from_utf8_lossy(&payload[offset..offset + len]).to_string();
        return msg;
    }

    "OK".to_string()
}

/// Parse error packet
fn parse_error_packet(payload: &[u8]) -> (u16, String) {
    if payload.len() < 3 {
        return (0, "Unknown error".to_string());
    }

    let error_code = u16::from_le_bytes([payload[1], payload[2]]);

    // Skip SQL state (5 bytes) if present
    let message_start = if payload.len() > 8 { 9 } else { 3 };
    let message = if message_start < payload.len() {
        String::from_utf8_lossy(&payload[message_start..]).to_string()
    } else {
        "Unknown error".to_string()
    };

    (error_code, message)
}

/// Print response in a user-friendly way
fn print_response(response: &str) {
    println!("{}", response);
}
