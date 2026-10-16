//! A05 (#5227) demonstration: secondary index staleness after a flush.
//!
//! The production write path (`insert_direct` / `insert_buffered` /
//! `flush_buffer` / `update_pk_index`) maintains ONLY the primary-key
//! index. A secondary index created by `CREATE INDEX` is not updated by
//! later INSERT/UPDATE/DELETE. This test demonstrates whether a row
//! flushed AFTER `CREATE INDEX` is visible through the secondary index,
//! and compares the index-scan result to a full-scan control.
//!
//! This is a demonstration (it prints the observed behaviour); it does not
//! assert, so the run-dir log records the actual index/san count.

use sqlrustgo_storage::FileStorage;
use std::path::PathBuf;
use std::sync::Arc;

fn engine_with(dir: &PathBuf) -> sqlrustgo::ExecutionEngine<FileStorage> {
    let storage =
        FileStorage::new_with_buffer_config(dir.clone(), 1_000_000, true).expect("FileStorage");
    sqlrustgo::ExecutionEngine::<FileStorage>::new(Arc::new(parking_lot::RwLock::new(storage)))
}

fn count(e: &mut sqlrustgo::ExecutionEngine<FileStorage>, sql: &str) -> usize {
    e.execute(sql)
        .unwrap_or_else(|err| panic!("{sql} failed: {err}"))
        .rows
        .len()
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("a05_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

/// A row flushed after `CREATE INDEX` on a secondary column: is it visible
/// through the secondary index? Compared against a no-index full-scan
/// control.
#[test]
fn a05_secondary_index_stale_after_flush() {
    let dir = tmpdir("stale");
    let mut e = engine_with(&dir);
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    // 1000 rows; k = i % 7, so k=5 occurs for i=5,12,...,999 (143 rows).
    for cs in (1..=1_000).step_by(500) {
        let vals: Vec<String> = (cs..cs + 500).map(|i| format!("({i},{})", i % 7)).collect();
        e.execute(&format!("INSERT INTO t VALUES {}", vals.join(",")))
            .expect("insert");
    }
    e.flush().ok();
    e.execute("CREATE INDEX idx_k ON t(k)")
        .expect("create index");

    // A new row with k=5, flushed AFTER the secondary index exists.
    e.execute("INSERT INTO t VALUES (1001, 5)").expect("insert");
    e.flush().ok();

    // Index-scan count (the query routes through the secondary index).
    let idx_count = count(&mut e, "SELECT id FROM t WHERE k = 5");
    let expected = 144; // 143 old rows with k=5 + the new row id=1001,k=5.
    eprintln!("A05 STALE HAZARD CHECK: SELECT id FROM t WHERE k = 5");
    eprintln!("  index-scan count = {} (expected {})", idx_count, expected);

    // Control: a separate table WITHOUT the index (full scan).
    let dir2 = tmpdir("control");
    let mut e2 = engine_with(&dir2);
    e2.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    for cs in (1..=1_000).step_by(500) {
        let vals: Vec<String> = (cs..cs + 500).map(|i| format!("({i},{})", i % 7)).collect();
        e2.execute(&format!("INSERT INTO t VALUES {}", vals.join(",")))
            .expect("insert");
    }
    e2.flush().ok();
    e2.execute("INSERT INTO t VALUES (1001, 5)")
        .expect("insert");
    e2.flush().ok();
    let full_count = count(&mut e2, "SELECT id FROM t WHERE k = 5");
    eprintln!("  full-scan count (control, no index) = {}", full_count);

    if idx_count == full_count && idx_count == expected {
        eprintln!("  RESULT: no staleness observed (index-scan and full-scan agree)");
    } else {
        eprintln!(
            "  RESULT: STALE HAZARD — index-scan {} != full-scan {} (missing {} row(s))",
            idx_count,
            full_count,
            full_count.saturating_sub(idx_count)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}
