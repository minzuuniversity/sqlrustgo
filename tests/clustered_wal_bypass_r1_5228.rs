//! R1 dynamic replay: CLUSTERED tables bypass WAL entirely (Issue #5228 / A06)
//!
//! Static evidence (SSOT §6, finding `suspected`):
//! - `clustered_table.rs`: pure in-memory `BTreeMap`, no WAL writes
//! - `engine_create.rs:397-424`: routes `ENGINE=Clustered` to in-memory `ClusteredTable` registry
//! - `engine_dml.rs:2018-2115`: `execute_insert_clustered` writes only to in-memory map
//! - `execution_engine.rs:165-171`: `clustered_tables` field is explicitly "in-memory registry"
//!
//! This test performs a **dynamic replay**: create a CLUSTERED table, insert
//! committed rows, drop the engine (simulating process death without a clean
//! shutdown), reopen the same data directory with `FileStorage::new_with_wal`,
//! and verify that the committed rows are gone.
//!
//! Expected outcome (R1 confirmed): rows are lost after reopen, proving the
//! WAL has no record of the CLUSTERED writes and cannot recover them.
//!
//! Evidence contract (per ADR-001 / ADR-014):
//! - Source: `crates/storage/src/clustered_table.rs`, `src/engine_dml.rs`
//! - Run: `cargo test -p sqlrustgo --test clustered_wal_bypass_r1_5228 --all-features`
//! - Finding: R1 (CLUSTERED bypasses WAL) — P0 veto, suspected → confirmed

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::file_storage::FileStorage;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

/// Create an ExecutionEngine backed by FileStorage with WAL enabled.
fn engine_with_wal(data_dir: &std::path::Path) -> ExecutionEngine<FileStorage> {
    let storage = FileStorage::new_with_wal(data_dir.to_path_buf())
        .expect("FileStorage::new_with_wal should succeed");
    ExecutionEngine::new(Arc::new(RwLock::new(storage)))
}

#[test]
fn r1_clustered_table_writes_lost_after_reopen() {
    let dir = TempDir::new().expect("tempdir");
    let data_dir = dir.path().join("r1_wal_bypass");
    fs::create_dir_all(&data_dir).unwrap();

    // Phase 1: create CLUSTERED table and insert committed rows.
    {
        let mut e = engine_with_wal(&data_dir);

        e.execute(
            "CREATE TABLE orders (id INTEGER PRIMARY KEY, name TEXT) \
             ENGINE=InnoDB CLUSTERED",
        )
        .expect("CREATE TABLE ENGINE=InnoDB CLUSTERED should succeed");

        e.execute("INSERT INTO orders VALUES (1, 'alice')")
            .expect("INSERT 1");
        e.execute("INSERT INTO orders VALUES (2, 'bob')")
            .expect("INSERT 2");
        e.execute("INSERT INTO orders VALUES (3, 'charlie')")
            .expect("INSERT 3");

        // Verify rows are visible before "crash".
        let r = e
            .execute("SELECT COUNT(*) FROM orders")
            .expect("SELECT COUNT before crash");
        assert_eq!(
            r.rows[0][0].to_string(),
            "3",
            "pre-crash: expected 3 rows in CLUSTERED table"
        );

        e.execute("COMMIT").ok();

        e.flush().expect("flush heap buffers before crash");

        drop(e);
    }

    {
        let mut e = engine_with_wal(&data_dir);

        let count_result = e.execute("SELECT COUNT(*) FROM orders");

        match count_result {
            Ok(r) => {
                let count = r.rows[0][0].to_string();
                assert_eq!(
                    count, "0",
                    "R1 CONFIRMED: CLUSTERED rows lost after reopen (WAL bypass). Got {count}."
                );
            }
            Err(err) => {
                panic!("R1 CONFIRMED (variant): CLUSTERED table not queryable after reopen: {err}");
            }
        }
    }
}

#[test]
fn r1_heap_table_rows_survive_reopen_control() {
    // Control: heap tables (no ENGINE=CLUSTERED) DO survive reopen,
    // proving the test harness and WAL are functional.
    let dir = TempDir::new().expect("tempdir");
    let data_dir = dir.path().join("r1_heap_control");
    fs::create_dir_all(&data_dir).unwrap();

    // Phase 1: create heap table and insert rows.
    {
        let mut e = engine_with_wal(&data_dir);

        e.execute("CREATE TABLE t_heap (id INTEGER PRIMARY KEY, name TEXT)")
            .expect("CREATE heap table");

        e.execute("INSERT INTO t_heap VALUES (1, 'alice')")
            .expect("INSERT 1");
        e.execute("INSERT INTO t_heap VALUES (2, 'bob')")
            .expect("INSERT 2");

        let r = e
            .execute("SELECT COUNT(*) FROM t_heap")
            .expect("SELECT COUNT before crash");
        assert_eq!(r.rows[0][0].to_string(), "2", "pre-crash: 2 heap rows");

        e.flush().expect("flush heap buffers before drop");

        drop(e); // simulate crash
    }

    // Phase 2: reopen and verify heap rows survive.
    {
        let mut e = engine_with_wal(&data_dir);

        let r = e
            .execute("SELECT COUNT(*) FROM t_heap")
            .expect("SELECT COUNT after reopen (heap)");
        assert_eq!(
            r.rows[0][0].to_string(),
            "2",
            "control: heap table rows should survive reopen (WAL works for heap)"
        );
    }
}
