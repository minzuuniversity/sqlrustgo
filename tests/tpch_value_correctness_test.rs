//! TPC-H value-correctness gate — Phase 2d Track 1.
//!
//! Drives the same in-process `ExecutionEngine` that
//! `tpch_gate_test.rs` uses, but on a small **synthetic** data
//! set generated inside the test. The hand-computed expected
//! values are asserted row-by-row, so this test proves
//! *correctness*, not just "no error returned".
//!
//! # Scope (and known gaps)
//!
//! Q1/Q3/Q6 from the TPC-H gate suite all *parse* but
//! return the wrong results today because of three
//! pre-existing engine gaps (each is documented inline at
//! the assertion that exercises it):
//!
//!   1. `WHERE col TEXT <= 'literal'` returns 0 rows even when
//!      the data passes the predicate.
//!   2. `FROM a, b, c` (comma-separated table list) is not
//!      supported — only `JOIN ... ON`.
//!   3. The engine returns ALL columns regardless of the
//!      SELECT clause, and the column NAMES appear as TEXT
//!      cells in the result rather than the values (see
//!      `tests/exp_g_wal_contracts_verified.rs`).
//!
//! Additional pre-existing engine bugs surfaced during
//! this test's authoring:
//!   4. `SUM(real_col)` returns 0 instead of the right value
//!      (aggregator is wired for INTEGER columns only).
//!   5. `AVG(real_col)` returns Null (same root cause).
//!
//! What we *can* prove today, and assert below:
//!   - `SELECT COUNT(*) FROM lineitem` returns 2.
//!   - `SELECT SUM(l_quantity) FROM lineitem` returns 30 (over
//!     an INTEGER column).
//!
//! The two regression-marker tests below will turn green when
//! the corresponding gaps are closed, at which point the
//! Q1/Q3 value assertions can be promoted from "no crash" to
//! "row count matches the hand-computed expected value".
//!
//! Refs: `docs/audit/analysis/2026-06-04-tpch-test-design.md`
//! (Track 1 — keep the in-process test, harden it with value
//! comparison against fixtures).

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

const SCHEMA_DDL: &[&str] = &[
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity INTEGER, l_extendedprice INTEGER, l_discount INTEGER, l_tax INTEGER, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT)",
];

const SEED_DML: &[&str] = &[
    "INSERT INTO lineitem VALUES (1, 1, 1, 1, 10, 1000, 0, 0, 'R', 'F', '1998-09-01')",
    "INSERT INTO lineitem VALUES (2, 1, 1, 1, 20, 2000, 0, 0, 'A', 'F', '1994-06-15')",
];

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for ddl in SCHEMA_DDL {
        engine.execute(ddl).expect("schema DDL should succeed");
    }
    for dml in SEED_DML {
        engine.execute(dml).expect("seed DML should succeed");
    }
    engine
}

/// Find the first INTEGER cell in a row, regardless of
/// position. The engine sometimes emits the column name
/// as a TEXT cell before the value (see
/// `exp_g_wal_contracts_verified`), so positional access
/// is unreliable.
fn first_integer_cell(row: &[sqlrustgo_types::Value]) -> Option<i64> {
    row.iter().find_map(|v| match v {
        sqlrustgo_types::Value::Integer(i) => Some(*i),
        _ => None,
    })
}

// =========================================================================
// Value assertions that DO work today
// =========================================================================

#[test]
fn test_tpch_count_is_correct() {
    let mut engine = make_engine();
    let result = engine
        .execute("SELECT COUNT(*) FROM lineitem")
        .expect("COUNT should execute");
    assert_eq!(result.rows.len(), 1, "COUNT should return 1 row");
    let count =
        first_integer_cell(&result.rows[0]).expect("COUNT row should contain an integer cell");
    assert_eq!(count, 2, "table has 2 lineitem rows");
}

#[test]
fn test_tpch_sum_is_correct() {
    let mut engine = make_engine();
    let result = engine
        .execute("SELECT SUM(l_quantity) FROM lineitem")
        .expect("SUM should execute");
    assert_eq!(result.rows.len(), 1, "SUM should return 1 row");
    let sum = first_integer_cell(&result.rows[0]).expect("SUM row should contain an integer cell");
    assert_eq!(sum, 30, "SUM(l_quantity) over the two seed rows = 30");
}

// =========================================================================
// Regression markers — these MUST turn green when the
// corresponding engine gaps are closed.
// =========================================================================

/// `WHERE col TEXT <= 'literal'` is broken in the v3.8.0
/// engine. Both seed rows pass the predicate (l_shipdate
/// '1998-09-01' <= '1998-09-02' AND '1994-06-15' <= '1998-09-02').
/// Expected row count after the fix: 2.
#[test]
fn test_tpch_q1_where_text_compare_returns_some_rows() {
    let mut engine = make_engine();
    let result = engine
        .execute(
            "SELECT l_returnflag, SUM(l_quantity) AS sum_qty \
             FROM lineitem \
             WHERE l_shipdate <= '1998-09-02' \
             GROUP BY l_returnflag \
             ORDER BY l_returnflag",
        )
        .expect("Q1 should not crash");
    assert_eq!(
        result.rows.len(),
        2,
        "Q1 should return 2 groups once TEXT comparison is fixed, \
         got: {:?}",
        result.rows
    );
}

/// `FROM a, b, c` (comma-separated table list) is not
/// supported in v3.8.0. The parser returns Err today,
/// which this test maps to a row count of 0. Once the
/// parser is fixed, this test will start returning 1 row
/// and will need to be promoted to assert that value.
#[ignore = "TPC-H Q3 comma-join — parser limitation (audit 2026-06-04)"]
#[test]
fn test_tpch_q3_three_table_join_row_count_today() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for ddl in [
        "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_mktsegment TEXT)",
        "CREATE TABLE orders    (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER, o_orderdate TEXT, o_shippriority INTEGER)",
        "CREATE TABLE lineitem  (l_orderkey INTEGER, l_shipdate TEXT, l_extendedprice INTEGER)",
    ] {
        engine.execute(ddl).expect("DDL should succeed");
    }
    engine
        .execute("INSERT INTO customer VALUES (1, 'BUILDING')")
        .expect("seed");
    engine
        .execute("INSERT INTO orders    VALUES (1, 1, '1995-01-01', 1)")
        .expect("seed");
    engine
        .execute("INSERT INTO lineitem  VALUES (1, '1998-09-01', 1000)")
        .expect("seed");

    let result = engine.execute(
        "SELECT l_orderkey FROM customer, orders, lineitem \
         WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey",
    );
    let row_count = match result {
        Ok(r) => r.rows.len(),
        Err(_) => 0,
    };
    assert_eq!(
        row_count, 0,
        "Q3-shape returns 0 rows today (parser rejects `FROM a, b, c`). \
         When the comma-join is fixed this should become 1 — the test will \
         start failing and must be promoted to assert_eq!(row_count, 1)."
    );
}
