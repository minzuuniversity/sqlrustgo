//! Shared NOT-RUN disclosure channel (issue #5503).
//!
//! The NOT-RUN three-state disclosure (`TPCH-NOT-RUN: <target> reason=… detail=…`)
//! used to reach the outside world through **two** channels:
//!
//! 1. `println!` on stdout — which `cargo test` captures for *passing* tests,
//!    so a local run showed `test result: ok` with no hint that the target
//!    had not run at all;
//! 2. the file named by `TPCH_NOT_RUN_FILE` — which only CI sets, so the
//!    reliable channel did not exist locally.
//!
//! This module is the single implementation of the file channel:
//!
//! * `TPCH_NOT_RUN_FILE` set (CI) → append there, unchanged behaviour;
//! * not set (local) → append to the **default** path
//!   `<target>/tpch_not_run.log`, so local and CI use the same channel.
//!
//! The default file holds the records of the **current test round**: the
//! first test binary of a round truncates it, the other binaries of the
//! same round append. A round is identified by the pid of the `cargo`
//! process that spawned the test binary (its parent), which cargo keeps
//! stable for all test binaries of one invocation and changes for the next
//! one. When the parent pid cannot be determined (no `ps` in the image),
//! the file is only appended to — never truncated — so no record is ever
//! lost.
//!
//! Every record is written with **one** `write_all` of `line + "\n"`:
//! the file is shared by every test binary of the round and opened with
//! `O_APPEND`, so `writeln!` (which formats through `write_fmt` and may
//! issue several `write()` syscalls) could split a record between two
//! processes and cost it its trailing newline.

use std::io::Write;
use std::path::{Path, PathBuf};

/// The file the CI summary counts NOT-RUN records from.
pub const NOT_RUN_ENV: &str = "TPCH_NOT_RUN_FILE";

/// The default local channel: `<target>/tpch_not_run.log`.
pub fn default_log_path() -> PathBuf {
    let target_dir = match std::env::var("CARGO_TARGET_DIR") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir.trim()),
        _ => workspace_root().join("target"),
    };
    target_dir.join("tpch_not_run.log")
}

/// The workspace root (the directory holding the `Cargo.toml` with a
/// `[workspace]` section). Falls back to the package directory.
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir: &Path = manifest.as_path();
    loop {
        if let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) {
            if text.contains("[workspace]") {
                return dir.to_path_buf();
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return manifest,
        }
    }
}

/// The pid of the process that spawned this test binary — the `cargo`
/// invocation. `None` when it cannot be determined.
fn parent_pid() -> Option<String> {
    let pid = std::process::id().to_string();
    let output = std::process::Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn marker_path(log: &Path) -> PathBuf {
    let mut name = log.file_name().unwrap_or_default().to_os_string();
    name.push(".round");
    log.with_file_name(name)
}

/// Make sure the default log holds the records of the current round only.
///
/// The first test binary of a round truncates the file and records the
/// round marker; the other binaries of the same round append.
fn prepare_round(log: &Path) {
    if let Some(dir) = log.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let Some(parent) = parent_pid() else {
        // No round detection available — appending is the safe direction:
        // a stale record is better than a lost one.
        return;
    };
    let marker = marker_path(log);
    let current = std::fs::read_to_string(&marker)
        .ok()
        .map(|s| s.trim().to_string());
    if current.as_deref() != Some(parent.as_str()) {
        let _ = std::fs::write(log, b"");
        let _ = std::fs::write(&marker, parent.as_bytes());
    }
}

/// Append one record to `path`, creating it (and its directory) if needed.
pub fn append_to(path: &Path, line: &str) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        // One syscall for the whole record **including** the newline.
        let _ = file.write_all(format!("{line}\n").as_bytes());
    }
}

/// Record one NOT-RUN line through the configured channel.
///
/// `TPCH_NOT_RUN_FILE` wins when it is set (CI); otherwise the default
/// local channel is used.
pub fn record(line: &str) {
    let configured = std::env::var(NOT_RUN_ENV).unwrap_or_default();
    if !configured.trim().is_empty() {
        append_to(Path::new(configured.trim()), line);
        return;
    }
    record_into_default(line);
}

/// Record one NOT-RUN line into the **default** local channel, ignoring
/// `TPCH_NOT_RUN_FILE`.
///
/// Used by the visibility test, which must not depend on the ambient
/// environment (and must not mutate it — `std::env::set_var` is unsafe in
/// edition 2024 and racy under a parallel test runner).
pub fn record_into_default(line: &str) {
    let path = default_log_path();
    prepare_round(&path);
    append_to(&path, line);
}

/// Record one NOT-RUN line into an explicit file (used by the visibility
/// test, which must not depend on the ambient environment).
pub fn record_to(path: &Path, line: &str) {
    append_to(path, line);
}
