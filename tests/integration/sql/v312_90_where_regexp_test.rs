//! V312-90 / P3-STR-004 regression integration test:
//! `WHERE s REGEXP 'pat'` — was silently broken because
//! `src/engine_utils.rs::sql_compare` (the WHERE-clause predicate
//! path) only knew about comparison operators (`=`, `>`, `LIKE`,
//! …). `s REGEXP 'pat'` fell through to `false`, so every row was
//! filtered out. Meanwhile, the same expression in the SELECT list
//! worked correctly because that path goes through
//! `eval_binary_op`, which DOES dispatch REGEXP to
//! `eval_regexp`.
//!
//! After the fix, `sql_compare` delegates REGEXP/RLIKE to
//! `sqlrustgo_executor::expr::eval_regexp` so the SELECT-list and
//! WHERE-predicate paths share the same regex engine.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn v312_90_where_regexp_anchored_alpha_only() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('abc123'), ('hello'), ('test_value')")
        .unwrap();
    // `^[a-z]+[0-9]+$` should match 'abc123' only.
    let res = x
        .execute("SELECT s FROM t WHERE s REGEXP '^[a-z]+[0-9]+$' ORDER BY s")
        .unwrap();
    assert_eq!(res.rows.len(), 1, "WHERE REGEXP must filter rows");
    assert_eq!(res.rows[0][0], Value::Text("abc123".into()));
}

#[test]
fn v312_90_where_regexp_partial_match() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('abc'), ('def'), ('ghi')")
        .unwrap();
    // `^[ad]` should match 'abc' and 'def' (not 'ghi').
    let res = x
        .execute("SELECT s FROM t WHERE s REGEXP '^[ad]' ORDER BY s")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(res.rows[0][0], Value::Text("abc".into()));
    assert_eq!(res.rows[1][0], Value::Text("def".into()));
}

#[test]
fn v312_90_where_rlike_alias() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('a@b.com'), ('plain'), ('c@d.org')")
        .unwrap();
    // RLIKE is the MySQL alias for REGEXP — must work identically in WHERE.
    let res = x
        .execute("SELECT s FROM t WHERE s RLIKE '@' ORDER BY s")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(res.rows[0][0], Value::Text("a@b.com".into()));
    assert_eq!(res.rows[1][0], Value::Text("c@d.org".into()));
}

#[test]
fn v312_90_where_regexp_no_match_returns_empty() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('abc'), ('def')").unwrap();
    // No row contains a digit.
    let res = x.execute("SELECT s FROM t WHERE s REGEXP '[0-9]'").unwrap();
    assert_eq!(res.rows.len(), 0);
}

#[test]
fn v312_90_where_regexp_select_consistency() {
    // The same predicate must yield identical results whether
    // evaluated in a SELECT-list expression (already worked via
    // eval_binary_op) or in a WHERE clause (broken before this fix).
    let mut x = fresh();
    x.execute("CREATE TABLE t (s TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('a1'), ('b2'), ('c')")
        .unwrap();
    let select_res = x
        .execute("SELECT s, s REGEXP '^[a-z][0-9]$' AS m FROM t ORDER BY s")
        .unwrap();
    let where_res = x
        .execute("SELECT s FROM t WHERE s REGEXP '^[a-z][0-9]$' ORDER BY s")
        .unwrap();
    // SELECT path: every row's `m` must agree with the WHERE filter.
    let where_set: Vec<String> = where_res
        .rows
        .iter()
        .map(|r| match &r[0] {
            Value::Text(s) => s.clone(),
            other => panic!("expected Text, got {:?}", other),
        })
        .collect();
    for row in &select_res.rows {
        let s = match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!(),
        };
        let m = match &row[1] {
            Value::Boolean(b) => *b,
            other => panic!("expected Boolean, got {:?}", other),
        };
        let in_where = where_set.contains(&s);
        assert_eq!(
            m, in_where,
            "SELECT ({}) and WHERE disagreed on row '{}'",
            m, s
        );
    }
    // And the WHERE set must be exactly {a1, b2}.
    assert_eq!(where_set, vec!["a1".to_string(), "b2".to_string()]);
}
