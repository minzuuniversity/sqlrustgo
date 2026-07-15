use clap::Parser;

pub mod checks;
mod cli;
mod error;
pub mod evidence;
pub mod gate;
mod runner;
pub mod workspace;

use cli::{Cli, Commands};
use runner::run_gate;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { stage, mode } => {
            run_gate(&stage, &mode)?;
        }
        Commands::Export { path } => {
            evidence::export(&path)?;
        }
        Commands::Report => {
            evidence::report()?;
        }
    }

    Ok(())
}
