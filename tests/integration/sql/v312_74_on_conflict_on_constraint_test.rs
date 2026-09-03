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