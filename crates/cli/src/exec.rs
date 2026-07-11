//! `sqlrustgo-cli` — one-off query execution.
//!
//! Connects to a running server, executes a single SQL statement,
//! and prints the result set or affected-row count.

use crate::client::Client;

/// Run a single SQL query against the server and print the result.
pub fn run_exec(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    sql: &str,
    json_output: bool,
) -> anyhow::Result<()> {
    let mut client = Client::connect(host, port, user, password)?;
    let result = client.query(sql)?;

    if json_output {
        let rows: Vec<serde_json::Value> = result
            .rows
            .iter()
            .map(|row| {
                let obj: serde_json::Value = result
                    .columns
                    .iter()
                    .zip(row.iter())
                    .map(|(col, val)| (col.clone(), serde_json::Value::String(val.clone())))
                    .collect::<serde_json::Map<_, _>>()
                    .into();
                obj
            })
            .collect();
        let output = serde_json::json!({
            "columns": result.columns,
            "rows": rows,
            "row_count": result.row_count,
            "duration_ms": result.duration.as_secs_f64() * 1000.0,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Table output
        if !result.columns.is_empty() {
            // Print column names
            for (i, col) in result.columns.iter().enumerate() {
                if i > 0 {
                    print!(" | ");
                }
                print!("{col}");
            }
            println!();

            // Print separator
            for (i, col) in result.columns.iter().enumerate() {
                if i > 0 {
                    print!("-+-");
                }
                print!("{}", "-".repeat(col.len()));
            }
            println!();

            // Print rows
            for row in &result.rows {
                for (i, val) in row.iter().enumerate() {
                    if i > 0 {
                        print!(" | ");
                    }
                    print!("{val}");
                }
                println!();
            }
        }
        println!(
            "({} row{} in {:.1}ms)",
            result.row_count,
            if result.row_count == 1 { "" } else { "s" },
            result.duration.as_secs_f64() * 1000.0,
        );
    }

    let _ = client.quit();
    Ok(())
}
