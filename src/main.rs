//! SQLRustGo Database System
//!
//! Usage:
//!   sqlrustgo "SELECT ..."

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    sqlrustgo::init();
    println!("SQLRustGo v1.6.0");
    
    if args.len() < 2 {
        println!("\nUsage: sqlrustgo '<sql_query>'");
        println!("Example: sqlrustgo 'SELECT 1'");
        return;
    }
    
    let sql = args[1..].join(" ");
    
    if sql.trim().is_empty() {
        println!("Error: Empty SQL query");
        return;
    }
    
    // Create in-memory storage and execution engine
    let storage = std::sync::Arc::new(
        std::sync::RwLock::new(sqlrustgo::MemoryStorage::new())
    );
    let mut engine = sqlrustgo::ExecutionEngine::new(storage);
    
    println!("\nQuery: {}", sql);
    println!("---");
    
    match sqlrustgo::parse(&sql) {
        Ok(statement) => {
            match engine.execute(statement) {
                Ok(result) => {
                    if result.rows.is_empty() {
                        println!("No rows returned.");
                    } else {
                        for row in &result.rows {
                            let values: Vec<String> = row.iter()
                                .map(|v| format_val(v))
                                .collect();
                            println!("{}", values.join(" | "));
                        }
                    }
                    println!("\nRows: {}", result.rows.len());
                }
                Err(e) => println!("Error: {}", e),
            }
        }
        Err(e) => println!("Parse error: {}", e),
    }
}

fn format_val(v: &sqlrustgo::Value) -> String {
    match v {
        sqlrustgo::Value::Null => "NULL".into(),
        sqlrustgo::Value::Boolean(b) => b.to_string(),
        sqlrustgo::Value::Integer(i) => i.to_string(),
        sqlrustgo::Value::Float(f) => f.to_string(),
        sqlrustgo::Value::Decimal(d) => format!("{}", d),
        sqlrustgo::Value::Text(s) => s.clone(),
        sqlrustgo::Value::Blob(b) => format!("BLOB[{}]", b.len()),
        sqlrustgo::Value::Date(d) => d.to_string(),
        sqlrustgo::Value::Timestamp(ts) => ts.to_string(),
        sqlrustgo::Value::Uuid(u) => u.to_string(),
        sqlrustgo::Value::Array(a) => format!("ARRAY[{}]", a.len()),
        sqlrustgo::Value::Enum(i, n) => format!("{}({})", n, i),
    }
}
