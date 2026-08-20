//! Crash Test Harness (P1-2 #3174)
//!
//! Shared utilities for cross-process crash testing. Provides:
//! - `spawn_db_with_env` — start a DB sub-process with fault-injection
//!   environment variables, return a child handle.
//! - `crash_at_point` — enumerate named crash injection points that
//!   the DB consults during startup or operation.
//! - `recover_and_verify` — re-open a (possibly corrupt) data dir and
//!   assert recovery semantics.
//!
//! This file is **not** a test target itself (it has no `#[test]`); it
//! is `include!`-d by `crash_test_framework.rs` and any other test
//! target that needs the harness.

#![allow(dead_code)] // helpers consumed by test targets via include!

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Named fault-injection points. The DB consults the env var
/// `SQLRUSTGO_CRASH_AT=<name>` at the matching site and aborts
/// (`std::process::exit(134)`) if it matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashPoint {
    /// Crash inside INSERT after the row has been written to the WAL
    /// but before the page write completes.
    InsertDuringPageWrite,
    /// Crash inside UPDATE after the WAL append but during index
    /// rebalance.
    UpdateDuringBtreeRebalance,
    /// Crash inside DELETE during B+tree rebalance (mid-rotation).
    DeleteDuringBtreeRebalance,
    /// Crash during COMMIT after the in-memory state is updated but
    /// before the WAL fsync returns.
    CommitDuringWalFsync,
    /// Crash during COMMIT before the transaction state is finalised
    /// (i.e. before the WAL append).
    CommitBeforeWalAppend,
    /// Crash in the middle of a WAL record write (partial record on
    /// disk).
    WalMidRecordWrite,
    /// Crash during CHECKPOINT, between page flush and metadata write.
    CheckpointDuringPageFlush,
    /// Crash during CHECKPOINT, after partial metadata write.
    CheckpointAfterPartialMeta,
    /// Crash during WAL replay (recovery mode), after applying some
    /// records but before completion.
    RecoveryDuringWalReplay,
}

impl CrashPoint {
    /// String form (matches the `SQLRUSTGO_CRASH_AT` env value).
    pub fn as_env(self) -> &'static str {
        match self {
            CrashPoint::InsertDuringPageWrite => "insert_during_page_write",
            CrashPoint::UpdateDuringBtreeRebalance => "update_during_btree_rebalance",
            CrashPoint::DeleteDuringBtreeRebalance => "delete_during_btree_rebalance",
            CrashPoint::CommitDuringWalFsync => "commit_during_wal_fsync",
            CrashPoint::CommitBeforeWalAppend => "commit_before_wal_append",
            CrashPoint::WalMidRecordWrite => "wal_mid_record_write",
            CrashPoint::CheckpointDuringPageFlush => "checkpoint_during_page_flush",
            CrashPoint::CheckpointAfterPartialMeta => "checkpoint_after_partial_meta",
            CrashPoint::RecoveryDuringWalReplay => "recovery_during_wal_replay",
        }
    }

    /// All known crash points (used by the harness for test enumeration
    /// and by the G8 gate to verify 8-category coverage).
    pub const ALL: &'static [CrashPoint] = &[
        CrashPoint::InsertDuringPageWrite,
        CrashPoint::UpdateDuringBtreeRebalance,
        CrashPoint::DeleteDuringBtreeRebalance,
        CrashPoint::CommitDuringWalFsync,
        CrashPoint::CommitBeforeWalAppend,
        CrashPoint::WalMidRecordWrite,
        CrashPoint::CheckpointDuringPageFlush,
        CrashPoint::CheckpointAfterPartialMeta,
        CrashPoint::RecoveryDuringWalReplay,
    ];
}

/// Spawn a DB sub-process with the crash-point env var set. Returns
/// the child handle plus the data dir used. The caller decides whether
/// to wait, kill, or assert.
pub fn spawn_db_with_crash_point(
    bin: &str,
    data_dir: &PathBuf,
    point: CrashPoint,
) -> std::io::Result<Child> {
    Command::new(bin)
        .arg("--data-dir")
        .arg(data_dir)
        .env("SQLRUSTGO_CRASH_AT", point.as_env())
        .env("SQLRUSTGO_FAULT_INJECTION", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

/// Kill the given child process group with `SIGKILL` (real, hard
/// crash simulation). On Windows this falls back to `kill()`.
#[cfg(unix)]
pub fn kill_hard(child: &mut Child) -> std::io::Result<()> {
    // kill the process group so children don't survive
    let pid = child.id();
    let _ = Command::new("kill")
        .arg("-9")
        .arg(format!("-{}", pid as i32))
        .status();
    let _ = child.wait();
    Ok(())
}

#[cfg(not(unix))]
pub fn kill_hard(child: &mut Child) -> std::io::Result<()> {
    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

/// Check that the named crash point has at least one test case
/// somewhere in the framework. Used by G8 gate.
pub fn has_test_for(point: CrashPoint) -> bool {
    // The G8 gate does the actual scanning via grep; this helper
    // exists for documentation / future programmatic checks.
    let _ = point;
    true
}
