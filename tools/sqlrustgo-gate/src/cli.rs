use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sqlrustgo-gate",
    about = "SQLRustGo Release Gate Engine — executable governance for Alpha/Beta/RC gates"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Execute a gate stage: alpha | beta | rc
    Run {
        /// Gate stage to execute: alpha, beta, or rc
        stage: String,
        /// Execution mode: preflight | partial | full | adaptive
        #[arg(long, default_value = "adaptive")]
        mode: String,
    },
    /// Export evidence report as JSON
    Export {
        /// Output path for the evidence JSON
        path: String,
    },
    /// Print a human-readable gate status report
    Report,
}
