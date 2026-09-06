//! V312-74 / Issue #4711 regression integration test:
//! `INSERT ... ON CONFLICT ON CONSTRAINT <name>` — SQLite standard
//! UPSERT targeting a named PRIMARY/UNIQUE constraint (not a column
//! list). Before this fix, the parser rejected the second `ON` keyword
//! with `Parse error: Expected Do, got On`.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn v312_74_on_conflict_on_constraint_updates_existing() {
    let mut x = fresh();
    x.execute("CREATE TABLE u(id INT PRIMARY KEY, val INT)").unwrap();
    x.execute("INSERT INTO u VALUES (1, 100)").unwrap();
    x.execute(
        "INSERT INTO u VALUES (1, 999) ON CONFLICT ON CONSTRAINT u_pkey \
         DO UPDATE SET val = 999",
    )
    .unwrap();
    let res = x.execute("SELECT id, val FROM u").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(1));
    assert_eq!(res.rows[0][1], Value::Integer(999));
}

#[test]
fn v312_74_on_conflict_on_constraint_do_nothing() {
    let mut x = fresh();
    x.execute("CREATE TABLE u(id INT PRIMARY KEY, val INT)").unwrap();
    x.execute("INSERT INTO u VALUES (1, 100)").unwrap();
    x.execute(
        "INSERT INTO u VALUES (1, 999) ON CONFLICT ON CONSTRAINT u_pkey \
         DO NOTHING",
    )
    .unwrap();
    // First row is preserved (DO NOTHING on conflict).
    let res = x.execute("SELECT val FROM u").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(100));
}

#[test]
fn v312_74_on_conflict_compound_key_still_works() {
    // Regression: the (col_list) form must still parse after the
    // ON CONSTRAINT branch is added.
    let mut x = fresh();
    x.execute("CREATE TABLE m(a INT, b INT, val INT, UNIQUE(a, b))")
        .unwrap();
    x.execute("INSERT INTO m VALUES (1, 1, 100)").unwrap();
    x.execute(
        "INSERT INTO m VALUES (1, 1, 999) \
         ON CONFLICT (a, b) DO UPDATE SET val = val + 1",
    )
    .unwrap();
    let res = x.execute("SELECT val FROM m").unwrap();
    assert_eq!(res.rows.len(), 1);
    // Conflict → DO UPDATE SET val = val + 1 → 100 + 1 = 101.
    assert_eq!(res.rows[0][0], Value::Integer(101));
}

#[test]
fn v312_74_on_conflict_bare_do_nothing_still_works() {
    // Regression: bare `ON CONFLICT DO NOTHING` (no target, no
    // constraint) must still parse.
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY, val INT)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 999) ON CONFLICT DO NOTHING")
        .unwrap();
    let res = x.execute("SELECT val FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Value::Integer(100));
}
// V312-90 / Issue #4807 — `INSERT ... ON CONFLICT (col) DO UPDATE SET
// col = EXCLUDED.col` must actually update the conflicting row. Before
// this fix, the engine evaluated `EXCLUDED.col` against the existing
// row context, so `cnt = EXCLUDED.cnt` was a no-op (the engine read
// the existing row's `cnt` and assigned it back to itself, leaving the
// row visually untouched). The fix threads the new row into
// `apply_odku` and intercepts `EXCLUDED.col` references before the
// generic row evaluator runs.
#[test]
fn v312_90_on_conflict_excluded_dot_col_uses_new_row() {
    let mut x = fresh();
    x.execute(
        "CREATE TABLE t(id INT PRIMARY KEY, name TEXT, cnt INT DEFAULT 0)",
    )
    .unwrap();
    x.execute("INSERT INTO t VALUES (1, 'a', 0)").unwrap();
    // EXCLUDED.cnt should resolve to the new row's `cnt` (= 5), not
    // the existing row's `cnt` (= 0).
    x.execute(
        "INSERT INTO t (id, name, cnt) VALUES (1, 'a', 5) \
         ON CONFLICT (id) DO UPDATE SET cnt = EXCLUDED.cnt",
    )
    .unwrap();
    let res = x.execute("SELECT cnt FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(
        res.rows[0][0],
        Value::Integer(5),
        "EXCLUDED.cnt must resolve to the new row's value (5), not the existing row's (0)"
    );
}

#[test]
fn v312_90_on_conflict_excluded_combined_with_existing_col() {
    // EXCLUDED.col resolves to the new row; plain column reference
    // resolves to the existing row. Both are needed in the same
    // assignment.
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY, base INT, delta INT)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 10, 0)").unwrap();
    // base + EXCLUDED.delta → 10 + 7 = 17
    x.execute(
        "INSERT INTO t (id, base, delta) VALUES (1, 0, 7) \
         ON CONFLICT (id) DO UPDATE SET base = base + EXCLUDED.delta",
    )
    .unwrap();
    let res = x.execute("SELECT base FROM t").unwrap();
    assert_eq!(res.rows[0][0], Value::Integer(17));
}

#[test]
fn v312_90_on_conflict_multiple_excluded_columns() {
    // Multiple EXCLUDED.col references in a single SET clause.
    let mut x = fresh();
    x.execute(
        "CREATE TABLE t(id INT PRIMARY KEY, name TEXT, val INT, cnt INT)",
    )
    .unwrap();
    x.execute("INSERT INTO t VALUES (1, 'a', 10, 100)").unwrap();
    x.execute(
        "INSERT INTO t (id, name, val, cnt) VALUES (1, 'b', 20, 200) \
         ON CONFLICT (id) DO UPDATE SET \
         name = EXCLUDED.name, val = EXCLUDED.val, cnt = EXCLUDED.cnt",
    )
    .unwrap();
    let res = x.execute("SELECT name, val, cnt FROM t").unwrap();
    assert_eq!(res.rows[0][0], Value::Text("b".to_string()));
    assert_eq!(res.rows[0][1], Value::Integer(20));
    assert_eq!(res.rows[0][2], Value::Integer(200));
}
