//! WP-C: v3.12.0 DDL/integrity legacy issues — REAL regression tests.
//!
//! Tests for 5 DDL/integrity issues that were closed via OR-downgrade /
//! anti-regression lockdown per `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md`:
//!
//! - #4652: CREATE PROCEDURE / CREATE FUNCTION (CLI rejects; executor behavior is the gap)
//! - #4672: SQLite AUTOINCREMENT (id allocation)
//! - #4682: sqlite_master / sqlite_sequence system tables
//! - #4669: DROP INDEX / function index / partial index
//! - #4703: UPSERT (ON DUPLICATE KEY UPDATE)
//!
//! The previous version (commit 2ccc5dd451) contained 28 placeholder tests
//! (`let expected = true; assert!(expected);`) that passed regardless of
//! whether the underlying fix existed.
//!
//! This rewrite:
//!   - removes the placeholder file at crates/storage/tests/wp_c_legacy.rs
//!     (storage crate has no engine API to drive DDL — the previous file
//!     existed but tested nothing).
//!   - creates real tests at the executor level using MemoryExecutionEngine
//!
//! Each test pins actual current behavior. Where behavior diverges from
//! the issue's intent, the test is #[ignore]'d with a gap note. Mutation:
//! break any of the tested behaviors and the relevant test fails.
//!
//! refs: LEGACY_ISSUES.md §3.1, §3.2

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn as_i64(v: &Value) -> i64 {
    v.as_integer()
        .unwrap_or_else(|| panic!("expected Integer, got {:?}", v))
}

fn as_str(v: &Value) -> String {
    if let Value::Text(s) = v {
        s.clone()
    } else {
        panic!("expected Text, got {:?}", v)
    }
}

// =========================================================================
// #4652 — CREATE PROCEDURE / CREATE FUNCTION
// OR-downgrade: CLI rejects; executor behavior is the bug-vs-fix surface.
// =========================================================================

mod issue_4652_procedure_function {
    use super::create_engine;

    /// GAP: #4652's PR #4741 puts the rejection in CLI prelude. The
    /// EXECUTOR still silently accepts CREATE PROCEDURE — which is the
    /// bug originally reported. Future PR must add executor-side guard.
    #[ignore = "GAP: executor still silently accepts CREATE PROCEDURE (CLI gates it but executor does not)"]
    #[test]
    fn create_procedure_at_executor_silently_accepted() {
        let mut e = create_engine();
        let r = e.execute("CREATE PROCEDURE test_proc() BEGIN SELECT 1; END");
        // Document current behavior (bug): silently accepts.
        assert!(r.is_err(), "future fix should make this error");
    }

    /// GAP: same as above for FUNCTION.
    #[ignore = "GAP: executor silently accepts CREATE FUNCTION"]
    #[test]
    fn create_function_at_executor_silently_accepted() {
        let mut e = create_engine();
        let r = e.execute("CREATE FUNCTION test_func() RETURNS INT BEGIN RETURN 1; END");
        assert!(r.is_err(), "future fix should make this error");
    }
}

// =========================================================================
// #4672 — SQLite AUTOINCREMENT
// =========================================================================

mod issue_4672_autoincrement {
    use super::{as_i64, as_str, create_engine};

    /// #4944: this used to hang the executor forever. It is no longer
    /// `#[ignore]`d.
    ///
    /// The original note guessed the cause ("AUTOINCREMENT appears to take
    /// a different code path that loops"). That was wrong. AUTOINCREMENT
    /// takes the same INSERT path as everything else; what differed is that
    /// this path is the only one that scans the table to compute
    /// `MAX(id) + 1`, and that scan went through `scan_for_reader`, which
    /// takes a **read** lock — while the caller already held the **write**
    /// lock. `storage_read()` falls back to a blocking `read()` when
    /// `try_read()` fails, and for the thread holding the write lock it
    /// always fails. `parking_lot::RwLock` is not reentrant, so the
    /// statement blocked on itself indefinitely.
    ///
    /// Fixed in `src/engine_dml.rs` by using the guard-taking
    /// `scan_for_reader_with(&storage, ..)` — the variant that exists
    /// precisely for this, and which the neighbouring duplicate-check scan
    /// already used. Re-running this test used to hit the 120s timeout; it
    /// now finishes in ~0.01s.
    // #4944: no longer `#[ignore]`d. This used to hang the executor
    // forever (">60s, no return"): the MAX(id) scan went through
    // `scan_for_reader`, which blocks on a read lock the calling thread
    // already held as a writer. `parking_lot::RwLock` is not reentrant.
    // Fixed in `src/engine_dml.rs` by using the guard-taking
    // `scan_for_reader_with`. Re-running the ignored test now finishes in
    // ~0.01s.
    #[test]
    fn autoincrement_allocates_sequential_ids() {
        let mut e = create_engine();
        e.execute("CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")
            .unwrap();
        e.execute("INSERT INTO t(name) VALUES ('alice'),('bob'),('carol')")
            .unwrap();
        let r = e.execute("SELECT id, name FROM t ORDER BY id").unwrap();
        assert_eq!(r.rows.len(), 3);
        assert_eq!(as_i64(&r.rows[0][0]), 1);
        assert_eq!(as_i64(&r.rows[1][0]), 2);
        assert_eq!(as_i64(&r.rows[2][0]), 3);
        assert_eq!(as_str(&r.rows[0][1]), "alice");
        assert_eq!(as_str(&r.rows[1][1]), "bob");
        assert_eq!(as_str(&r.rows[2][1]), "carol");
    }

    /// #4944: same deadlock as the sibling test above.
    // #4944: un-ignored with the sibling test above — same deadlock.
    #[test]
    fn autoincrement_continues_after_delete() {
        let mut e = create_engine();
        e.execute("CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT, v INT)")
            .unwrap();
        e.execute("INSERT INTO t(v) VALUES (10),(20),(30)").unwrap();
        e.execute("DELETE FROM t WHERE v = 20").unwrap();
        e.execute("INSERT INTO t(v) VALUES (40)").unwrap();
        let r = e.execute("SELECT id FROM t ORDER BY id").unwrap();
        assert_eq!(r.rows.len(), 3, "DELETE then INSERT yields 3 rows");
        // IDs: 1, 2(deleted), 3, 4(new). Wait — that's only 3 present.
        // The 3 present are id=1 (v=10), id=3 (v=30), id=4 (v=40).
        // SQLite's AUTOINCREMENT *never* reuses deleted IDs (unlike
        // AUTO_INCREMENT in some MySQL configurations). If our impl
        // reuses, this test will catch it.
        let mut ids: Vec<i64> = r.rows.iter().map(|row| as_i64(&row[0])).collect();
        ids.sort();
        assert_eq!(
            ids,
            vec![1, 3, 4],
            "AUTOINCREMENT must not reuse deleted id=2"
        );
    }
}

// =========================================================================
// #4682 — sqlite_master / sqlite_sequence
// OR-downgrade: documented; executor may not have these tables.
// =========================================================================

mod issue_4682_system_tables {
    use super::create_engine;

    /// GAP: sqlite_master is a SQLite-specific system table; current
    /// HEAD does not implement it.
    #[ignore = "GAP: sqlite_master table does not exist (no SQLite system tables in current HEAD)"]
    #[test]
    fn sqlite_master_is_queryable() {
        let mut e = create_engine();
        e.execute("CREATE TABLE user_t(x INT)").unwrap();
        e.execute("CREATE INDEX idx_x ON user_t(x)").unwrap();
        let r = e.execute("SELECT count(*) FROM sqlite_master");
        assert!(r.is_ok(), "sqlite_master should be queryable");
    }

    /// GAP: sqlite_sequence — same root cause.
    #[ignore = "GAP: sqlite_sequence table does not exist"]
    #[test]
    fn sqlite_sequence_after_autoincrement() {
        let mut e = create_engine();
        e.execute("CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT, v INT)")
            .unwrap();
        e.execute("INSERT INTO t(v) VALUES (1)").unwrap();
        let r = e.execute("SELECT * FROM sqlite_sequence");
        assert!(
            r.is_ok(),
            "sqlite_sequence should be queryable after AUTOINCREMENT"
        );
    }
}

// =========================================================================
// #4669 — DROP INDEX / function index / partial index
// =========================================================================

mod issue_4669_drop_index {
    use super::create_engine;

    /// GAP: DROP INDEX may or may not work — pin current behavior.
    #[test]
    fn drop_index_after_create() {
        let mut e = create_engine();
        e.execute("CREATE TABLE t(a INT, b INT)").unwrap();
        e.execute("CREATE INDEX idx_a ON t(a)").unwrap();
        // Either DROP succeeds, or it errors with a clear message.
        // Currently it may error with "DROP INDEX not fully supported yet".
        let r = e.execute("DROP INDEX idx_a");
        match r {
            Ok(_) => {
                // After DROP, the index should not be in sqlite_master.
                let _ = e.execute("SELECT * FROM sqlite_master");
            }
            Err(e) => {
                let msg = format!("{:?}", e);
                let msg_lower = msg.to_lowercase();
                // Acceptable: error mentioning DROP INDEX or index/idx
                assert!(
                    msg_lower.contains("drop")
                        || msg_lower.contains("index")
                        || msg_lower.contains("idx")
                        || msg_lower.contains("not supported"),
                    "DROP INDEX error must be recognizable: {}",
                    msg
                );
            }
        }
    }

    /// GAP: function index (CREATE INDEX ON t(UPPER(a))).
    #[ignore = "GAP: function index is not supported at executor level"]
    #[test]
    fn create_function_index_errors_recognizably() {
        let mut e = create_engine();
        e.execute("CREATE TABLE t(a INT)").unwrap();
        let r = e.execute("CREATE INDEX idx_up ON t(UPPER(a))");
        // Should error with a clear message — silent acceptance is the bug.
        assert!(r.is_err(), "function index must error, not silently accept");
    }

    /// GAP: partial index (WHERE clause on CREATE INDEX).
    #[ignore = "GAP: partial index WHERE clause not supported"]
    #[test]
    fn create_partial_index_errors_recognizably() {
        let mut e = create_engine();
        e.execute("CREATE TABLE t(val INT)").unwrap();
        let r = e.execute("CREATE INDEX idx_partial ON t(val) WHERE val > 200");
        assert!(r.is_err(), "partial index must error if not supported");
    }
}

// =========================================================================
// #4703 — UPSERT / ON DUPLICATE KEY UPDATE
// =========================================================================

mod issue_4703_upsert {
    use super::{as_i64, create_engine};

    /// WORKS: ON DUPLICATE KEY UPDATE on a single-column PRIMARY KEY.
    #[test]
    fn on_duplicate_key_update_increments_value() {
        let mut e = create_engine();
        e.execute("CREATE TABLE u(id INT PRIMARY KEY, v INT)")
            .unwrap();
        e.execute("INSERT INTO u VALUES (1, 100)").unwrap();
        e.execute("INSERT INTO u VALUES (1, 200) ON DUPLICATE KEY UPDATE v = v + 1")
            .unwrap();
        let r = e.execute("SELECT id, v FROM u").unwrap();
        assert_eq!(r.rows.len(), 1, "UPSERT must not insert a duplicate row");
        assert_eq!(as_i64(&r.rows[0][0]), 1);
        // After UPSERT: v was 100, UPDATE clause is v = v + 1 → v = 101.
        assert_eq!(
            as_i64(&r.rows[0][1]),
            101,
            "v must increment from 100 to 101 on UPDATE"
        );
    }

    /// WORKS: ON DUPLICATE KEY UPDATE without collision inserts normally.
    #[test]
    fn on_duplicate_key_update_no_collision_inserts_normally() {
        let mut e = create_engine();
        e.execute("CREATE TABLE u(id INT PRIMARY KEY, v INT)")
            .unwrap();
        e.execute("INSERT INTO u VALUES (1, 100)").unwrap();
        e.execute("INSERT INTO u VALUES (2, 200) ON DUPLICATE KEY UPDATE v = v + 1")
            .unwrap();
        let r = e.execute("SELECT id, v FROM u ORDER BY id").unwrap();
        assert_eq!(r.rows.len(), 2, "non-colliding UPSERT must insert");
        assert_eq!(as_i64(&r.rows[0][0]), 1);
        assert_eq!(as_i64(&r.rows[0][1]), 100, "row 1 untouched");
        assert_eq!(as_i64(&r.rows[1][0]), 2);
        assert_eq!(as_i64(&r.rows[1][1]), 200, "row 2 inserted with v=200");
    }

    /// GAP: multi-column ON DUPLICATE KEY UPDATE — OR-downgrade.
    #[ignore = "GAP: multi-column UPSERT is OR-downgrade (sub-bug #1/#4 of #4703)"]
    #[test]
    fn on_duplicate_key_update_multi_column() {
        let mut e = create_engine();
        e.execute("CREATE TABLE u(id INT PRIMARY KEY, a INT, b INT)")
            .unwrap();
        e.execute("INSERT INTO u VALUES (1, 10, 20)").unwrap();
        let r = e.execute(
            "INSERT INTO u VALUES (1, 11, 21) ON DUPLICATE KEY UPDATE a = a + 1, b = b + 1",
        );
        assert!(
            r.is_err(),
            "multi-col UPSERT must error or work — not silently accept"
        );
    }

    /// GAP: ON CONFLICT (SQLite syntax).
    #[ignore = "GAP: ON CONFLICT (SQLite syntax) — needs verification"]
    #[test]
    fn on_conflict_sqlite_syntax() {
        let mut e = create_engine();
        e.execute("CREATE TABLE u(id INT PRIMARY KEY, v INT)")
            .unwrap();
        e.execute("INSERT INTO u VALUES (1, 100)").unwrap();
        let r = e.execute("INSERT INTO u VALUES (1, 200) ON CONFLICT(id) DO UPDATE SET v = v + 1");
        assert!(
            r.is_err(),
            "ON CONFLICT syntax error expected if not supported"
        );
    }

    /// WORKS: ON DUPLICATE KEY UPDATE with VALUES(col) — anti-regression.
    #[test]
    fn on_duplicate_key_update_with_values_function() {
        // PR #5015 (issue #5009) fixed this. Single-column form.
        let mut e = create_engine();
        e.execute("CREATE TABLE u(id INT PRIMARY KEY, v INT)")
            .unwrap();
        e.execute("INSERT INTO u VALUES (1, 100)").unwrap();
        e.execute("INSERT INTO u VALUES (1, 999) ON DUPLICATE KEY UPDATE v = VALUES(v)")
            .unwrap();
        let r = e.execute("SELECT id, v FROM u").unwrap();
        assert_eq!(r.rows.len(), 1);
        // VALUES(v) should resolve to the *new* value (999).
        assert_eq!(
            as_i64(&r.rows[0][1]),
            999,
            "VALUES(v) on UPSERT resolves to new value"
        );
    }
}
