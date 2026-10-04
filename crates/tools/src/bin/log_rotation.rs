//! Log rotation CLI entry point.
//!
//! The rotation logic lives in `sqlrustgo_tools::log_rotation`; this
//! file only supplies `main`, parsing the command and dispatching to
//! `run_log_rotation_cmd`.

use sqlrustgo_tools::log_rotation::LogRotationCommand;
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    let cmd = LogRotationCommand::from_args();
    sqlrustgo_tools::log_rotation::run_log_rotation_cmd(cmd)
}
