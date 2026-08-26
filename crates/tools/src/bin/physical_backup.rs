//! `physical-backup` — physical backup and restore based on storage
//! snapshots.
//!
//! CLI subcommands (delegated to the corresponding pub functions in
//! `sqlrustgo_tools::physical_backup`):
//!
//!   physical-backup backup   -d <dir> [-D <data>] [-w <wal>] [-c] [--parent P]
//!   physical-backup list     -d <dir>
//!   physical-backup verify   -d <dir>
//!   physical-backup restore  -d <dir> -t <target> [-w <wal-target>]
//!   physical-backup prune    -d <dir> [-k N] [-D days] [-n] [-f]
//!   physical-backup help
//!
//! See `docs/releases/v3.12.0/evidence/v312-14-CRASH-RECOVERY-RECHECK.md`
//! §3 (Backup/Restore) — required by RC3 `promotion_to_RC_requires`
//! so that operators can run physical backup outside the test harness.

use anyhow::{bail, Result};
use sqlrustgo_tools::physical_backup::{
    create_physical_backup, list_physical_backups, prune_physical_backups,
    restore_physical_backup, verify_physical_backup, PhysicalBackupCommand,
};
use structopt::StructOpt;

fn main() -> Result<()> {
    let cmd = PhysicalBackupCommand::from_args();
    match cmd {
        PhysicalBackupCommand::Backup {
            dir,
            data_dir,
            wal_dir,
            compress,
            parent,
        } => create_physical_backup(&dir, &data_dir, &wal_dir, compress, parent.as_deref())
            .map_err(|e| anyhow::anyhow!("backup failed: {e}")),
        PhysicalBackupCommand::List { dir } => list_physical_backups(&dir)
            .map_err(|e| anyhow::anyhow!("list failed: {e}")),
        PhysicalBackupCommand::Verify { dir } => {
            verify_physical_backup(&dir).map_err(|e| anyhow::anyhow!("verify failed: {e}"))?;
            println!("Physical backup OK: {}", dir.display());
            Ok(())
        }
        PhysicalBackupCommand::Restore {
            dir,
            target,
            wal_target,
        } => restore_physical_backup(&dir, &target, wal_target.as_deref())
            .map_err(|e| anyhow::anyhow!("restore failed: {e}")),
        PhysicalBackupCommand::Prune {
            dir,
            keep,
            keep_days,
            dry_run,
            force,
        } => {
            if !force && !dry_run && (keep.is_some() || keep_days.is_some()) {
                eprintln!(
                    "About to prune backups under {} (keep={:?}, keep_days={:?}).",
                    dir.display(),
                    keep,
                    keep_days
                );
                eprintln!("Re-run with --force to confirm, or use --dry-run to preview.");
                bail!("refusing to prune without --force or --dry-run");
            }
            prune_physical_backups(&dir, keep, keep_days, dry_run, force)
                .map_err(|e| anyhow::anyhow!("prune failed: {e}"))
        }
    }
}
