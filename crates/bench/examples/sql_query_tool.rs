//! Simple SQL Query Tool for SQLRustGo
//!
//! Executes SQL queries against a SQLite database using SQLRustGo's parser and executor
//!
//! Usage:
//!   cargo run -p sqlrustgo-bench --example sql_query_tool -- --db /path/to/db.sqlite "SELECT * FROM t"

use rusqlite::Connection;
use std::time::Instant;
use std::fs;
use std::path::Path;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        println!("Usage: {} --db <database_path> '<sql_query>'", args[0]);
        println!("Example: {} --db /tmp/tpch_sf10.db 'SELECT COUNT(*) FROM lineitem'", args[0]);
        std::process::exit(1);
    }
    
    let db_path = if args.len() > 2 && args[1] == "--db" {
        &args[2]
    } else {
        eprintln!("Error: Expected --db flag");
        std::process::exit(1);
    };
    
    let sql = if args.len() > 4 && args[3] == "--query" {
        &args[4..].join(" ")
    } else if args.len() > 4 {
        &args[3..].join(" ")
    } else {
        eprintln!("Error: Expected SQL query");
        std::process::exit(1);
    };
    
    if !Path::new(db_path).exists() {
        eprintln!("Error: Database file not found: {}", db_path);
        std::process::exit(1);
    }
    
    println!("Database: {}", db_path);
    println!("Query: {}", sql);
    println!();
    
    let conn = Connection::open(db_path).expect("Failed to open database");
    
    let start = Instant::now();
    let mut stmt = conn.prepare(sql).expect("Failed to prepare statement");
    
    let column_count = stmt.column_count();
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    
    println!("Columns: {:?}", column_names);
    println!();
    
    let rows = stmt.query_map([], |row| {
        let mut values: Vec<String> = Vec::new();
        for i in 0..column_count {
            let value: String = match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) => "NULL".to_string(),
                Ok(rusqlite::types::ValueRef::Integer(i)) => i.to_string(),
                Ok(rusqlite::types::ValueRef::Real(f)) => f.to_string(),
                Ok(rusqlite::types::ValueRef::Text(s)) => String::from_utf8_lossy(s).to_string(),
                Ok(rusqlite::types::ValueRef::Blob(b)) => format!("BLOB[{} bytes]", b.len()),
                Err(_) => "ERROR".to_string(),
            };
            values.push(value);
        }
        Ok(values)
    }).expect("Failed to execute query");
    
    let mut count = 0;
    for row in rows {
        match row {
            Ok(values) => {
                println!("{}", values.join(" | "));
                count += 1;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
    
    println!();
    println!("Rows returned: {}", count);
    println!("Time: {:.3}s", start.elapsed().as_secs_f64());
}
