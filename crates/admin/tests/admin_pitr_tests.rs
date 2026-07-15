//! Admin pitr (point-in-time recovery) tests
//!
//! Tests: pitr_replay file-not-found error path, parse_target_time edge cases

use sqlrustgo_admin::pitr::{parse_target_time, pitr_replay};
use sqlrustgo_admin::backup::BackupError;

#[test]
fn test_pitr_replay_file_not_found() {
    let result = pitr_replay(std::path::Path::new("/nonexistent/wal/file.wal"), 1000);
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Should be an EntryNotFound or Io error
    let msg = err.to_string();
    assert!(msg.contains("not found") || msg.contains("Nonexistent") || msg.contains(" WAL"),
        "expected not-found error, got: {}", msg);
}

#[test]
fn test_parse_target_time_unix_zero() {
    // Unix epoch = 0 is valid
    let t = parse_target_time("0").unwrap();
    assert_eq!(t, 0);
}

#[test]
fn test_parse_target_time_large_unix() {
    // Year 3000 unix timestamp
    let t = parse_target_time("32503680000").unwrap();
    assert_eq!(t, 32503680000);
}

#[test]
fn test_parse_target_time_rfc3339_utc() {
    let t = parse_target_time("2026-07-15T00:00:00Z").unwrap();
    assert!(t > 0);
}

#[test]
fn test_parse_target_time_rfc3339_with_offset() {
    let t = parse_target_time("2026-07-15T12:00:00+08:00").unwrap();
    assert!(t > 0);
}

#[test]
fn test_parse_target_time_empty_string() {
    let result = parse_target_time("");
    assert!(result.is_err());
}
