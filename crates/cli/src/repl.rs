//! `sqlrustgo-cli repl` — Interactive REPL shell.
//!
//! Connects to a running server and provides a readline-based
//! interactive SQL console with history.

use crate::client::Client;
use rustyline::error::ReadlineError;
use rustyline::Editor;

/// Run the interactive REPL.
pub fn run_repl(host: &str, port: u16, user: &str, password: &str) -> anyhow::Result<()> {
    let mut client = Client::connect(host, port, user, password)?;

    println!("SQLRustGo CLI REPL — connected to {host}:{port}");
    // Issue #4176 / V312-38: warn when the user accidentally connected
    // to a non-sqlrustgo MySQL on the default port (system MySQL collision).
    let diag = client.server_kind().diagnostic_for_port(port);
    if !diag.is_empty() {
        eprintln!("{diag}");
    }
    println!("Enter SQL statements. Type `exit` or Ctrl-D to quit.");
    println!();

    let mut rl = Editor::<(), rustyline::history::DefaultHistory>::new()?;
    if rl.load_history(".sqlrustgo_history").is_err() {
        // No history file yet — that's fine.
    }

    loop {
        let readline = rl.readline("sqlrustgo> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
                    break;
                }
                let _ = rl.add_history_entry(&line);

                match client.query(trimmed) {
                    Ok(result) => {
                        if !result.columns.is_empty() {
                            // Print column names
                            for (i, col) in result.columns.iter().enumerate() {
                                if i > 0 {
                                    print!(" | ");
                                }
                                print!("{col}");
                            }
                            println!();

                            // Separator
                            for (i, col) in result.columns.iter().enumerate() {
                                if i > 0 {
                                    print!("-+-");
                                }
                                print!("{}", "-".repeat(col.len()));
                            }
                            println!();
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
                    Err(e) => {
                        eprintln!("Error: {e}");
                    }
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                eprintln!("Readline error: {e}");
                break;
            }
        }
    }

    let _ = rl.save_history(".sqlrustgo_history");
    let _ = client.quit();
    println!("Bye.");
    Ok(())
}
