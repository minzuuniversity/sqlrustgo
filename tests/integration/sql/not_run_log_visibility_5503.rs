//! #5503 — the NOT-RUN disclosure must be visible **locally**, on the same
//! channel CI uses.
//!
//! Before this, the file channel only existed when `TPCH_NOT_RUN_FILE` was
//! set (CI sets it; a local run does not), and the stdout channel was
//! swallowed by `cargo test` for passing tests. A local run therefore showed
//! `test result: ok` with no hint that a target had not run at all.
//!
//! Close conditions from the issue:
//!
//! 1. a local run (no environment variable) leaves the round's NOT-RUN
//!    records in a file at the default path;
//! 2. the default path and its purpose are documented
//!    (`tests/README.md`);
//! 3. a self-proof: a target that reports NOT-RUN has a record in that
//!    file, and a target that actually ran has none.

#[path = "../../common/not_run_log.rs"]
mod not_run_log;

use std::path::{Path, PathBuf};

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "sqlrustgo-not-run-{}-{name}.log",
        std::process::id()
    ));
    path
}

#[test]
fn default_path_lives_under_the_target_directory() {
    let path = not_run_log::default_log_path();
    assert!(
        path.ends_with("tpch_not_run.log"),
        "the default channel must be a single well-known file; got {path:?}"
    );
    let parent = path.parent().expect("default path has a parent");
    match std::env::var("CARGO_TARGET_DIR") {
        Ok(dir) if !dir.trim().is_empty() => assert_eq!(
            parent,
            Path::new(dir.trim()),
            "the default channel must live in the cargo target directory; got {path:?}"
        ),
        _ => assert!(
            parent.ends_with("target"),
            "the default channel must live under the target directory; got {path:?}"
        ),
    }
}

#[test]
fn a_not_run_target_is_recorded_and_a_target_that_ran_is_not() {
    let path = temp_path("visibility");
    let _ = std::fs::remove_file(&path);

    not_run_log::record_to(
        &path,
        "TPCH-NOT-RUN: target_a reason=fixture-missing detail=none",
    );
    let text = std::fs::read_to_string(&path).expect("record file");

    assert!(
        text.contains("target_a"),
        "a target that reported NOT-RUN must have a record; got {text:?}"
    );
    assert!(
        !text.contains("target_b"),
        "a target that actually ran must NOT have a record; got {text:?}"
    );
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("TPCH-NOT-RUN:"))
            .count(),
        1,
        "exactly one record expected; got {text:?}"
    );
    assert!(
        text.ends_with('\n'),
        "a record must be a complete line (the newline is part of the same \
         write); got {text:?}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn several_records_land_as_separate_complete_lines() {
    let path = temp_path("multiline");
    let _ = std::fs::remove_file(&path);

    for i in 0..3 {
        not_run_log::record_to(
            &path,
            &format!("TPCH-NOT-RUN: target_{i} reason=r detail=d"),
        );
    }
    let text = std::fs::read_to_string(&path).expect("record file");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "three records, three lines; got {text:?}");
    for (i, line) in lines.iter().enumerate() {
        assert!(
            line.starts_with(&format!("TPCH-NOT-RUN: target_{i}")),
            "record {i} must be intact; got {line:?}"
        );
    }

    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_default_channel_receives_a_record_without_any_environment_variable() {
    let path = not_run_log::default_log_path();
    let existed = path.exists();
    let before = std::fs::read_to_string(&path).unwrap_or_default();

    not_run_log::record_into_default(
        "TPCH-NOT-RUN: target_default_probe reason=self-proof detail=none",
    );

    let after = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        after.contains("target_default_probe"),
        "the default channel must receive the record without \
         TPCH_NOT_RUN_FILE being set; got {after:?}"
    );

    // Leave the channel as it was found: this is a probe, not a real
    // NOT-RUN, and a stale probe record would be counted by whoever reads
    // the file next.
    if existed {
        let _ = std::fs::write(&path, before.as_bytes());
    } else {
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(round_marker(&path));
    }
}

fn round_marker(log: &Path) -> PathBuf {
    let mut name = log.file_name().unwrap_or_default().to_os_string();
    name.push(".round");
    log.with_file_name(name)
}
