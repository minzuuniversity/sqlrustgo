//! Regression tests for the 4 bugs reported in
//! `docs/reference/sqlrustgo-bug-report.md` (v3.11.0 GA local build,
//! 2026-08-26). All four bugs were verified fixed on
//! `develop/v3.12.0` HEAD after the V312-bug-report-3120 PR series.
//!
//! Bugs covered:
//! - BUG-1:  UTF-8 Chinese string data lexer panic
//! - BUG-2a: nested function call parse error (e.g. `year(now())`)
//! - BUG-2b: built-in functions returning Null (`now`/`curdate`/
//!           `datediff`/`round`/`rand`/`length`)
//! - BUG-3a: JOIN `alias.column` non-aggregate non-GROUP BY column
//!           returning Null in GROUP BY query
//! - BUG-3b: scalar subquery in WHERE returning 0 rows
//! - BUG-4:  `char(n)` blank-padded comparison failure

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_bug1_utf8_chinese_string_data_roundtrip() {
    // BUG-1: `insert into u values(1, '张三')` used to panic in
    // lexer.rs:103 because `read_string` advanced `position` by 1
    // byte per char instead of `ch.len_utf8()`. The fix in 252
    // develop/v3.12.0 (read_string) preserves char boundaries so
    // Chinese string data round-trips through INSERT/SELECT.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE u (id INTEGER, name CHAR(8))")
        .unwrap();
    engine.execute("INSERT INTO u VALUES (1, '张三')").unwrap();

    let result = engine.execute("SELECT name FROM u WHERE id = 1").unwrap();
    assert_eq!(result.rows.len(), 1);
    // CHAR(8) is blank-padded to 8 chars in storage; trim_end gives
    // the user-visible value.
    match &result.rows[0][0] {
        Value::Text(s) => assert_eq!(s.trim_end(), "张三", "got {:?}", s),
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn test_bug2a_nested_function_call_no_parse_error() {
    // BUG-2a: `year(now())` used to fail with
    // "Parse error: Expected RParen, got Eof" because the parser
    // unconditionally consumed a trailing `)` after every
    // function-call args list, eating the parent call's closing
    // paren. The fix gates the consume on `name == "CAST"`.
    let mut engine = create_engine();
    // year(now()) must not parse-error and must return the current
    // year as an Integer. We can't assert an exact year (the test
    // runs whenever), but it must be > 2000 and an Integer.
    let result = engine.execute("SELECT year(now())").unwrap();
    assert_eq!(
        result.rows.len(),
        1,
        "expected 1 row, got {:?}",
        result.rows
    );
    match &result.rows[0][0] {
        Value::Integer(y) => assert!(*y > 2000, "year(now()) returned {}, expected > 2000", y),
        other => panic!("expected Integer year, got {:?}", other),
    }
}

#[test]
fn test_bug2b_builtin_functions_not_null() {
    // BUG-2b: `now`/`curdate`/`curtime`/`datediff`/`round`/`rand`/
    // `length` used to return Null silently because `eval_fn` was
    // missing arms for them. The fix adds the arms.
    let mut engine = create_engine();

    // now() — returns a Text "YYYY-MM-DD HH:MM:SS" (length 19).
    let r = engine.execute("SELECT now()").unwrap();
    match &r.rows[0][0] {
        Value::Text(s) => assert_eq!(s.len(), 19, "now() = {:?}", s),
        other => panic!("now() expected Text, got {:?}", other),
    }

    // curdate() — returns a Text "YYYY-MM-DD" (length 10).
    let r = engine.execute("SELECT curdate()").unwrap();
    match &r.rows[0][0] {
        Value::Text(s) => assert_eq!(s.len(), 10, "curdate() = {:?}", s),
        other => panic!("curdate() expected Text, got {:?}", other),
    }

    // datediff('2019-01-01','2018-01-01') — 365 days.
    let r = engine
        .execute("SELECT datediff('2019-01-01', '2018-01-01')")
        .unwrap();
    assert_eq!(r.rows[0][0], Value::Integer(365));

    // round(3.14159, 2) — 3.14 (Float, d>0).
    let r = engine.execute("SELECT round(3.14159, 2)").unwrap();
    assert_eq!(r.rows[0][0], Value::Float(3.14));

    // round(3.6) — 4 (Integer, d<=0).
    let r = engine.execute("SELECT round(3.6)").unwrap();
    assert_eq!(r.rows[0][0], Value::Integer(4));

    // rand() — Float in [0, 1).
    let r = engine.execute("SELECT rand()").unwrap();
    match &r.rows[0][0] {
        Value::Float(f) => assert!((0.0..1.0).contains(f), "rand() = {}, expected [0, 1)", f),
        other => panic!("rand() expected Float, got {:?}", other),
    }

    // length('alice') — 5.
    let r = engine.execute("SELECT length('alice')").unwrap();
    assert_eq!(r.rows[0][0], Value::Integer(5));

    // abs(-5) — 5.
    let r = engine.execute("SELECT abs(-5)").unwrap();
    assert_eq!(r.rows[0][0], Value::Integer(5));
}

#[test]
fn test_bug3a_join_alias_column_in_group_by() {
    // BUG-3a: `select s.name, avg(sc.final) from s join sc on
    // s.id = sc.sid group by sc.sid` returned Null for `s.name`
    // (the aggregate value was correct). The non-aggregate non-
    // GROUP BY column had no entry in the re-projection lookup
    // maps, so the engine emitted Null. The fix falls back to
    // evaluating the column's expression against the first row of
    // the group (MySQL non-strict-mode semantics).
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE s (id INTEGER, name CHAR(8))")
        .unwrap();
    engine
        .execute("CREATE TABLE sc (sid INTEGER, cid INTEGER, final INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO s VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();
    engine
        .execute("INSERT INTO sc VALUES (1, 100, 90), (1, 101, 85), (2, 100, 70)")
        .unwrap();

    let result = engine
        .execute("SELECT s.name, avg(sc.final) FROM s JOIN sc ON s.id = sc.sid GROUP BY sc.sid")
        .unwrap();
    assert_eq!(
        result.rows.len(),
        2,
        "expected 2 groups, got {:?}",
        result.rows
    );

    // Group by sc.sid produces sid=1 (alice, 87.5) and sid=2 (bob, 70.0).
    // Order may vary by HashMap iteration; collect (name, avg) pairs.
    let pairs: Vec<(String, f64)> = result
        .rows
        .iter()
        .map(|r| {
            let name = match &r[0] {
                Value::Text(s) => s.trim_end().to_string(),
                other => panic!("expected Text name, got {:?}", other),
            };
            let avg = match &r[1] {
                Value::Float(f) => *f,
                other => panic!("expected Float avg, got {:?}", other),
            };
            (name, avg)
        })
        .collect();

    assert!(
        pairs
            .iter()
            .any(|(n, a)| n == "alice" && (*a - 87.5).abs() < 1e-9),
        "alice/87.5 missing in {:?}",
        pairs
    );
    assert!(
        pairs
            .iter()
            .any(|(n, a)| n == "bob" && (*a - 70.0).abs() < 1e-9),
        "bob/70.0 missing in {:?}",
        pairs
    );
}

#[test]
fn test_bug3b_scalar_subquery_in_where() {
    // BUG-3b: `select * from s where id = (select min(id) from s
    // where name = 'bob')` returned 0 rows because
    // `substitute_outer_refs_in_select` was mis-substituting the
    // subquery's own `name` column (a column of the inner FROM
    // table `s`) with the outer row's `name` value. The prefix-
    // based own_column detection only handled TPC-H table prefixes
    // (l_, p_, s_, etc.), so for classroom tables like `s(id,
    // name)`, bare `name` was treated as an outer ref. The fix
    // looks up the inner table's actual schema from storage and
    // protects its columns from substitution.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE s (id INTEGER, name CHAR(8))")
        .unwrap();
    engine
        .execute("INSERT INTO s VALUES (1, 'alice'), (2, 'bob')")
        .unwrap();

    let result = engine
        .execute("SELECT * FROM s WHERE id = (SELECT min(id) FROM s WHERE name = 'bob')")
        .unwrap();
    assert_eq!(
        result.rows.len(),
        1,
        "expected 1 row, got {:?}",
        result.rows
    );
    assert_eq!(result.rows[0][0], Value::Integer(2));
    match &result.rows[0][1] {
        Value::Text(s) => assert_eq!(s.trim_end(), "bob", "got {:?}", s),
        other => panic!("expected Text name, got {:?}", other),
    }
}

#[test]
fn test_bug4_char_n_blank_padded_comparison() {
    // BUG-4: `char(n)` is blank-padded on store in MySQL (e.g.
    // CHAR(2) of 'F' is stored as "F "). An equality against the
    // bare literal 'F' must ignore trailing spaces. The fix adds
    // trim_end to Text comparison in `eq_cross` and `sql_compare`.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER, sex CHAR(2))")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 'F'), (2, 'M')")
        .unwrap();

    let r = engine
        .execute("SELECT count(*) FROM t WHERE sex = 'F'")
        .unwrap();
    assert_eq!(r.rows[0][0], Value::Integer(1));

    let r = engine.execute("SELECT * FROM t WHERE sex = 'F'").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(1));
}
