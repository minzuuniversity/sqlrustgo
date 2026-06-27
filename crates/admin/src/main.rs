use clap::{Parser, Subcommand};

mod backup;
mod manifest;
mod pitr;
mod restore;
mod verify;

#[derive(Parser)]
#[command(
    name = "sqlrustgo-admin",
    version,
    about = "SQLRustGo admin CLI: backup, restore, verify, PITR"
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
    }
}
