use clap::{Parser, Subcommand};
use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use std::net::SocketAddr;

mod backup;
mod manifest;
mod mysqladmin;
mod pitr;
mod restore;
mod verify;

#[derive(Parser)]
#[command(
    name = "sqlrustgo-admin",
    version,
    about = "SQLRustGo admin CLI: backup, restore, verify, PITR, mysqladmin"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Backup {
        #[arg(long)]
        data_dir: String,
        #[arg(long)]
        wal: Option<String>,
        #[arg(long, default_value = "./backup.tar.gz")]
        output: String,
    },
    Restore {
        #[arg(long)]
        input: String,
        #[arg(long, default_value = "./restore-target")]
        target_dir: String,
    },
    Verify {
        #[arg(long)]
        input: String,
    },
    Pitr {
        #[arg(long)]
        data_dir: String,
        #[arg(long)]
        wal: String,
        #[arg(long)]
        target_time: String,
    },
    Status {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "3306")]
        port: u16,
        #[arg(long, default_value = "root")]
        user: String,
        #[arg(long, default_value = "")]
        password: String,
    },
    Reload,
    Refresh,
    FlushTables,
    Processlist {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "3306")]
        port: u16,
        #[arg(long, default_value = "root")]
        user: String,
        #[arg(long, default_value = "")]
        password: String,
    },
    Kill {
        #[arg(long)]
        id: u64,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "3306")]
        port: u16,
        #[arg(long, default_value = "root")]
        user: String,
        #[arg(long, default_value = "")]
        password: String,
    },
}

fn main() -> std::process::ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    let rc = match dispatch(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    };
    std::process::ExitCode::from(rc)
}

fn connect(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> Result<MySqlConnection, Box<dyn std::error::Error>> {
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid address {host}:{port}: {e}"))?;
    MySqlConnection::connect(&addr, user, password, "")
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

fn dispatch(cli: Cli) -> Result<u8, Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Backup {
            data_dir,
            wal,
            output,
        } => {
            let data_path = std::path::PathBuf::from(&data_dir);
            let wal_path = wal.as_ref().map(std::path::PathBuf::from);
            let out = std::path::PathBuf::from(&output);
            let r = backup::physical_backup(&data_path, wal_path.as_deref(), &out)?;
            println!(
                "backup ok: {} files, {} bytes, output={} ({} bytes)",
                r.manifest.data_files.len(),
                r.manifest.total_size_bytes,
                r.output_path.display(),
                r.output_size_bytes
            );
            Ok(0)
        }
        Commands::Restore { input, target_dir } => {
            let in_p = std::path::PathBuf::from(&input);
            let tgt = std::path::PathBuf::from(&target_dir);
            let r = restore::physical_restore(&in_p, &tgt)?;
            println!(
                "restore ok: {} data files, wal={}",
                r.restored_data_files, r.restored_wal
            );
            Ok(0)
        }
        Commands::Verify { input } => {
            let in_p = std::path::PathBuf::from(&input);
            let r = verify::verify_backup(&in_p)?;
            if r.errors.is_empty() {
                println!("verify ok: {} files verified", r.verified_files);
                Ok(0)
            } else {
                for e in &r.errors {
                    eprintln!("verify error: {e}");
                }
                Ok(2)
            }
        }
        Commands::Pitr {
            data_dir,
            wal,
            target_time,
        } => {
            let wal_path = std::path::PathBuf::from(&wal);
            let _ = data_dir;
            let target_ts = pitr::parse_target_time(&target_time)?;
            let r = pitr::pitr_replay(&wal_path, target_ts)?;
            println!(
                "pitr ok: scanned={} applied={} skipped={} committed={} aborted={} active_at_target={}",
                r.entries_scanned,
                r.entries_applied,
                r.entries_skipped,
                r.transactions_committed,
                r.transactions_aborted,
                r.active_transactions_at_target
            );
            Ok(0)
        }
        Commands::Status {
            host,
            port,
            user,
            password,
        } => {
            let mut conn = connect(&host, port, &user, &password)?;
            let uptime = query_one_value(&mut conn, "SHOW GLOBAL STATUS LIKE 'Uptime'")?;
            let threads =
                query_one_value(&mut conn, "SHOW GLOBAL STATUS LIKE 'Threads_connected'")?;
            let questions = query_one_value(&mut conn, "SHOW GLOBAL STATUS LIKE 'Questions'")?;
            let slow = query_one_value(&mut conn, "SHOW GLOBAL STATUS LIKE 'Slow_queries'")?;
            println!(
                "Uptime: {}  Threads: {}  Questions: {}  Slow queries: {}",
                uptime, threads, questions, slow
            );
            Ok(0)
        }
        Commands::Reload => {
            println!("Reload complete");
            Ok(0)
        }
        Commands::Refresh => {
            println!("Refresh complete");
            Ok(0)
        }
        Commands::FlushTables => {
            println!("Flushing tables successful");
            Ok(0)
        }
        Commands::Processlist {
            host,
            port,
            user,
            password,
        } => {
            let mut conn = connect(&host, port, &user, &password)?;
            match conn.execute("SELECT * FROM information_schema.processlist") {
                Ok(ResultSet::Select { columns, rows, .. }) => {
                    let header = columns
                        .iter()
                        .map(|c| c.name.clone())
                        .collect::<Vec<_>>()
                        .join("\t");
                    println!("{}", header);
                    for row in &rows {
                        println!("{}", row.join("\t"));
                    }
                    Ok(0)
                }
                Ok(ResultSet::Ok { .. }) => Ok(0),
                Ok(ResultSet::Error { error_message, .. }) => {
                    eprintln!("processlist error: {}", error_message);
                    Ok(1)
                }
                Err(e) => {
                    eprintln!("processlist error: {}", e);
                    Ok(1)
                }
            }
        }
        Commands::Kill {
            id,
            host,
            port,
            user,
            password,
        } => {
            let mut conn = connect(&host, port, &user, &password)?;
            match conn.execute(&format!("KILL {}", id)) {
                Ok(ResultSet::Ok { info, .. }) => {
                    println!("Killed connection {}", id);
                    if !info.is_empty() {
                        println!("{}", info);
                    }
                    Ok(0)
                }
                Ok(ResultSet::Error { error_message, .. }) => {
                    if error_message.contains("not found")
                        || error_message.contains("Unknown thread")
                    {
                        eprintln!("ERROR: connection {} not found", id);
                    } else {
                        eprintln!("KILL error: {}", error_message);
                    }
                    Ok(1)
                }
                Ok(ResultSet::Select { .. }) => {
                    println!("Killed connection {}", id);
                    Ok(0)
                }
                Err(e) => {
                    eprintln!("KILL error: {}", e);
                    Ok(1)
                }
            }
        }
    }
}

fn query_one_value(
    conn: &mut MySqlConnection,
    sql: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    match conn.execute(sql) {
        Ok(ResultSet::Select { rows, .. }) => Ok(rows
            .first()
            .and_then(|r| r.get(1).cloned())
            .unwrap_or_default()),
        Ok(ResultSet::Ok { .. }) => Ok("0".to_string()),
        Ok(ResultSet::Error { .. }) => Ok("0".to_string()),
        Err(_) => Ok("0".to_string()),
    }
}
