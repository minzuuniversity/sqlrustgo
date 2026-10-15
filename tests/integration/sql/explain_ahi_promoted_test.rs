//! PR3 / #5305 — EXPLAIN `ahi_promoted` suffix snapshot semantics tests.
//!
//! Verifies the PR3 contract:
//! - Every EXPLAIN output row ends with ` | ahi_promoted=<u64>`.
//! - The value matches `engine.adaptive_hash_index().promoted_count()` at
//!   EXPLAIN entry (snapshot semantics, all rows equal).
//! - EXPLAIN itself does not mutate the AHI counter (Ruling 9 final:
//!   no production code path drives AHI on EXPLAIN).

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::adaptive_hash_index::AdaptiveHashIndex;

fn last_token(line: &str) -> Option<&str> {
    line.split(" | ahi_promoted=").nth(1)
}

fn first_cell_text(row: &[sqlrustgo_types::Value], i: usize) -> &str {
    row.first()
        .and_then(|v| match v {
            sqlrustgo_types::Value::Text(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("row {i} first cell must be Value::Text"))
}

#[test]
fn test_explain_includes_ahi_promoted_suffix_on_fresh_engine() {
    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .expect("create table");
    let ahi_at_explain = engine.adaptive_hash_index().promoted_count();
    assert_eq!(ahi_at_explain, 0, "fresh AHI: promoted_count must be 0");

    let result = engine
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1;")
        .expect("EXPLAIN must succeed");

    assert!(
        !result.rows.is_empty(),
        "EXPLAIN must emit at least one operator line"
    );
    for (i, row) in result.rows.iter().enumerate() {
        let line = first_cell_text(row, i);
        let token = last_token(line).unwrap_or_else(|| {
            panic!("row {i} must end with ' | ahi_promoted=<u64>'; got: {line}")
        });
        let n: u64 = token
            .parse()
            .unwrap_or_else(|_| panic!("ahi_promoted value must parse as u64; got: {token}"));
        assert_eq!(
            n, ahi_at_explain,
            "row {i} snapshot must match AHI counter at EXPLAIN entry"
        );
    }
}

#[test]
fn test_explain_ahi_promoted_reflects_promoted_entries_after_record_access() {
    let ahi = AdaptiveHashIndex::new();

    // 20 accesses past threshold=17 promotes at least one entry (per AHI
    // source line 17; see crates/storage/src/adaptive_hash_index.rs:127).
    for _ in 0..20 {
        ahi.record_access("t", &[1u8], 42, 0);
    }
    let expected = ahi.promoted_count();
    assert!(
        expected >= 1,
        "20 accesses past threshold must promote at least 1 entry (got {expected})"
    );

    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .expect("create table");
    // Replace the engine's AHI with our hand-promoted one so EXPLAIN
    // observes the promoted count.
    engine.set_adaptive_hash_index(std::sync::Arc::new(ahi));

    let result = engine
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1;")
        .expect("EXPLAIN must succeed");

    assert!(!result.rows.is_empty());
    for (i, row) in result.rows.iter().enumerate() {
        let line = first_cell_text(row, i);
        let token = last_token(line).unwrap_or_else(|| {
            panic!("row {i} must end with ' | ahi_promoted=<u64>'; got: {line}")
        });
        let n: u64 = token
            .parse()
            .unwrap_or_else(|_| panic!("ahi_promoted value must parse as u64; got: {token}"));
        assert_eq!(
            n, expected,
            "row {i} must reflect promoted count {expected} after record_access"
        );
    }
}

#[test]
fn test_explain_does_not_increment_ahi_promoted_during_explain() {
    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .expect("create table");
    let ahi = engine.adaptive_hash_index();

    let before = ahi.promoted_count();
    engine
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1;")
        .expect("EXPLAIN must succeed");
    let after = ahi.promoted_count();

    assert_eq!(
        before, after,
        "EXPLAIN must not mutate AHI promoted_count (snapshot semantics)"
    );
}
