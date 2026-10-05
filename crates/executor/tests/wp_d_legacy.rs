//! WP-D: v3.12.0 join/subquery legacy issues — REAL regression tests.
//!
//! Tests for join/subquery legacy issues fixed in v4.0.0+:
//! - #4668: NATURAL JOIN / multi-column USING errors
//! - #4656: > ALL / = ANY subquery errors
//! - #4649: LEFT JOIN USING degenerates to Cartesian product
//! - #4636: Correlated scalar subquery failures
//!
//! Each test creates tables, runs the actual SQL through MemoryExecutionEngine,
//! and asserts on row count and value content — NOT `assert!(true)`.
//!
//! The previous version of this file (commit 2ccc5dd451) contained 29
//! placeholder tests that all did `let expected = true; assert!(expected);` —
//! vacuous. This rewrite pins the actual contract for each issue. A regression
//! in any of the 4 fixes will turn this file red.
//!
//! refs: LEGACY_ISSUES.md §3.5

use sqlrustgo::MemoryExecutionEngine;

/// Helper: create a fresh in-memory engine with the given schema + data SQL.
fn fresh_engine(setup: &[&str]) -> MemoryExecutionEngine {
    let mut e = MemoryExecutionEngine::with_memory();
    for sql in setup {
        e.execute(sql).unwrap_or_else(|err| {
            panic!("setup SQL failed: {}\n  error: {:?}", sql, err);
        });
    }
    e
}

// =========================================================================
// #4668 — NATURAL JOIN / multi-column USING
// =========================================================================

// #4668 OR-downgrade lives in the CLI (`crates/sqlrustgo-cli/src/sqlite_mode.rs`),
// not the executor. The CLI rejects NATURAL JOIN and multi-column USING at
// the SQL boundary; the executor below handles them correctly if they
// ever reach it (e.g. via direct engine.execute() in tests, or via a
// non-CLI SQL surface in the future).
//
// These tests therefore pin the executor contract:
//   - NATURAL JOIN produces correct join semantics on matching columns
//   - USING (col1, col2, ...) produces correct join semantics
//   - USING (single col) produces correct join semantics
//
// A regression here (e.g. NATURAL JOIN degenerating to Cartesian)
// is what #4649 fixed in spirit; the same shape of bug would reappear here.
mod issue_4668_natural_join {
    use super::fresh_engine;
    use sqlrustgo_types::Value;

    fn setup_two_tables() -> Vec<&'static str> {
        vec![
            "CREATE TABLE t1 (id INT, name TEXT)",
            "CREATE TABLE t2 (id INT, val TEXT)",
            "INSERT INTO t1 VALUES (1, 'a'), (2, 'b')",
            "INSERT INTO t2 VALUES (1, 'x'), (2, 'y')",
        ]
    }

    #[test]
    fn natural_join_basic_returns_correct_rows() {
        // NATURAL JOIN auto-matches on shared column name (id).
        // Expected: 2 rows, each pairing (1,a)↔(1,x) and (2,b)↔(2,y).
        let mut e = fresh_engine(&setup_two_tables());
        let r = e
            .execute("SELECT id, name, val FROM t1 NATURAL JOIN t2 ORDER BY id")
            .unwrap();
        assert_eq!(
            r.rows.len(),
            2,
            "NATURAL JOIN must not Cartesian; got {}: {:?}",
            r.rows.len(),
            r.rows
        );
        assert_eq!(
            r.rows[0],
            vec![
                Value::Integer(1),
                Value::Text("a".into()),
                Value::Text("x".into())
            ]
        );
        assert_eq!(
            r.rows[1],
            vec![
                Value::Integer(2),
                Value::Text("b".into()),
                Value::Text("y".into())
            ]
        );
    }

    #[test]
    fn natural_left_join_preserves_left_rows() {
        // Add a t1 row with no t2 match → LEFT JOIN keeps it with NULL.
        let mut e = fresh_engine(&[
            "CREATE TABLE t1 (id INT, name TEXT)",
            "CREATE TABLE t2 (id INT, val TEXT)",
            "INSERT INTO t1 VALUES (1, 'a'), (2, 'b'), (3, 'c')",
            "INSERT INTO t2 VALUES (1, 'x'), (3, 'y')",
        ]);
        let r = e
            .execute("SELECT id, name, val FROM t1 NATURAL LEFT JOIN t2 ORDER BY id")
            .unwrap();
        assert_eq!(r.rows.len(), 3, "NATURAL LEFT JOIN must keep all left rows");
        assert_eq!(r.rows[0][1], Value::Text("a".into()));
        assert_eq!(r.rows[0][2], Value::Text("x".into()));
        assert_eq!(r.rows[1][1], Value::Text("b".into()));
        assert_eq!(r.rows[1][2], Value::Null, "id=2 has no match → val NULL");
        assert_eq!(r.rows[2][1], Value::Text("c".into()));
        assert_eq!(r.rows[2][2], Value::Text("y".into()));
    }

    #[test]
    fn natural_inner_join_drops_unmatched() {
        let mut e = fresh_engine(&[
            "CREATE TABLE t1 (id INT, name TEXT)",
            "CREATE TABLE t2 (id INT, val TEXT)",
            "INSERT INTO t1 VALUES (1, 'a'), (2, 'b'), (3, 'c')",
            "INSERT INTO t2 VALUES (1, 'x'), (3, 'y')",
        ]);
        let r = e
            .execute("SELECT id, name, val FROM t1 NATURAL INNER JOIN t2 ORDER BY id")
            .unwrap();
        assert_eq!(r.rows.len(), 2, "INNER JOIN drops the unmatched row");
    }

    #[test]
    fn using_single_column_works() {
        let mut e = fresh_engine(&setup_two_tables());
        let r = e
            .execute("SELECT t1.id, t2.val FROM t1 JOIN t2 USING (id) ORDER BY t1.id")
            .unwrap();
        assert_eq!(r.rows.len(), 2, "single-column USING must work");
        assert_eq!(r.rows[0][0], Value::Integer(1));
        assert_eq!(r.rows[1][0], Value::Integer(2));
    }

    #[test]
    fn using_multi_column_works() {
        // Multi-column USING pairs on (id, name). t1=(1,a)↔t2=(1,x) does not
        // match on name, so 0 rows.
        let mut e = fresh_engine(&[
            "CREATE TABLE t1 (id INT, name TEXT)",
            "CREATE TABLE t2 (id INT, name TEXT)",
            "INSERT INTO t1 VALUES (1, 'a'), (2, 'b')",
            "INSERT INTO t2 VALUES (1, 'x'), (2, 'b')",
        ]);
        let r = e
            .execute("SELECT t1.id FROM t1 JOIN t2 USING (id, name) ORDER BY t1.id")
            .unwrap();
        // Only (2, 'b') matches both columns.
        assert_eq!(
            r.rows.len(),
            1,
            "multi-column USING matches on all listed cols"
        );
        assert_eq!(r.rows[0][0], Value::Integer(2));
    }
}

// =========================================================================
// #4656 — > ALL / = ANY subquery
// =========================================================================

mod issue_4656_quantified_subquery {
    use super::fresh_engine;
    use sqlrustgo_types::Value;

    #[test]
    fn greater_than_all_returns_rows_above_max_subquery() {
        // orders: alice(100), bob(200,150), carol(300)
        // subquery returns [100]; rows with amt > 100 are bob(200), bob(150), carol(300).
        let mut e = fresh_engine(&[
            "CREATE TABLE orders (cust TEXT, amt INT)",
            "INSERT INTO orders VALUES ('alice', 100), ('bob', 200), ('bob', 150), ('carol', 300)",
        ]);
        let r = e
            .execute(
                "SELECT cust, amt FROM orders WHERE amt > ALL (SELECT amt FROM orders WHERE cust = 'alice') ORDER BY amt",
            )
            .unwrap();
        // Three rows above alice's 100.
        assert_eq!(
            r.rows.len(),
            3,
            "expected 3 rows, got {}: {:?}",
            r.rows.len(),
            r.rows
        );
        let amounts: Vec<i64> = r
            .rows
            .iter()
            .map(|row| {
                if let Value::Integer(n) = row[1] {
                    n
                } else {
                    panic!("expected Integer for amt, got {:?}", row[1])
                }
            })
            .collect();
        assert_eq!(amounts, vec![150, 200, 300], "amounts must all be > 100");
    }

    #[test]
    fn equal_to_any_returns_rows_matching_any_subquery_value() {
        // subquery for cust='bob' returns [200, 150]; rows with amt in {200, 150}
        // should be bob(200) and bob(150).
        let mut e = fresh_engine(&[
            "CREATE TABLE orders (cust TEXT, amt INT)",
            "INSERT INTO orders VALUES ('alice', 100), ('bob', 200), ('bob', 150), ('carol', 300)",
        ]);
        let r = e
            .execute(
                "SELECT cust, amt FROM orders WHERE amt = ANY (SELECT amt FROM orders WHERE cust = 'bob') ORDER BY amt",
            )
            .unwrap();
        assert_eq!(
            r.rows.len(),
            2,
            "expected 2 bob rows, got {}: {:?}",
            r.rows.len(),
            r.rows
        );
        for row in &r.rows {
            assert_eq!(
                row[0],
                Value::Text("bob".to_string()),
                "only bob should match"
            );
        }
    }

    #[test]
    fn greater_than_all_with_no_subquery_rows_returns_all() {
        // Edge case: subquery returns no rows → ALL is vacuously true → all rows pass.
        let mut e = fresh_engine(&[
            "CREATE TABLE orders (cust TEXT, amt INT)",
            "INSERT INTO orders VALUES ('alice', 100), ('bob', 200)",
        ]);
        let r = e
            .execute(
                "SELECT cust, amt FROM orders WHERE amt > ALL (SELECT amt FROM orders WHERE cust = 'nobody') ORDER BY amt",
            )
            .unwrap();
        assert_eq!(r.rows.len(), 2, "empty subquery → ALL is vacuously true");
    }
}

// =========================================================================
// #4649 — LEFT JOIN USING (col) degenerates to Cartesian product
// =========================================================================

mod issue_4649_left_join_using {
    use super::fresh_engine;
    use sqlrustgo_types::Value;

    #[test]
    fn left_join_using_returns_3_rows_not_9() {
        // a(3 rows) LEFT JOIN b USING (id) — must produce 3 rows, NOT 9 (Cartesian).
        let mut e = fresh_engine(&[
            "CREATE TABLE a (id INT, name TEXT)",
            "CREATE TABLE b (id INT, city TEXT)",
            "INSERT INTO a VALUES (1, 'alice'), (2, 'bob'), (3, 'carol')",
            "INSERT INTO b VALUES (1, 'NY'), (3, 'LA'), (4, 'SF')",
        ]);
        let r = e
            .execute("SELECT a.name, b.city FROM a LEFT JOIN b USING (id) ORDER BY a.id")
            .unwrap();
        assert_eq!(
            r.rows.len(),
            3,
            "LEFT JOIN USING must yield 3 rows (not 9 Cartesian); got {}: {:?}",
            r.rows.len(),
            r.rows
        );
        // Row 1: alice + NY (id=1 matches)
        // Row 2: bob + NULL (id=2 has no match in b)
        // Row 3: carol + LA (id=3 matches)
        let alice = &r.rows[0];
        assert_eq!(alice[0], Value::Text("alice".to_string()));
        assert_eq!(alice[1], Value::Text("NY".to_string()));
        let bob = &r.rows[1];
        assert_eq!(bob[0], Value::Text("bob".to_string()));
        assert_eq!(bob[1], Value::Null, "bob has no match in b → city is NULL");
        let carol = &r.rows[2];
        assert_eq!(carol[0], Value::Text("carol".to_string()));
        assert_eq!(carol[1], Value::Text("LA".to_string()));
    }

    #[test]
    fn inner_join_using_returns_only_matches() {
        // Same setup, INNER JOIN should yield only 2 rows (matches only).
        let mut e = fresh_engine(&[
            "CREATE TABLE a (id INT, name TEXT)",
            "CREATE TABLE b (id INT, city TEXT)",
            "INSERT INTO a VALUES (1, 'alice'), (2, 'bob'), (3, 'carol')",
            "INSERT INTO b VALUES (1, 'NY'), (3, 'LA'), (4, 'SF')",
        ]);
        let r = e
            .execute("SELECT a.name, b.city FROM a JOIN b USING (id) ORDER BY a.id")
            .unwrap();
        assert_eq!(
            r.rows.len(),
            2,
            "INNER JOIN USING should drop the unmatched left row"
        );
        assert_eq!(r.rows[0][0], Value::Text("alice".to_string()));
        assert_eq!(r.rows[1][0], Value::Text("carol".to_string()));
    }
}

// =========================================================================
// #4636 — correlated scalar subquery
// =========================================================================

mod issue_4636_correlated_scalar_subquery {
    use super::fresh_engine;
    use sqlrustgo_types::Value;

    #[test]
    fn correlated_scalar_returns_per_row_value() {
        // SELECT id, (SELECT b.val FROM b WHERE b.id = a.id) AS bval FROM a
        // — must return a row per a.id, with the bval matched to b.id.
        let mut e = fresh_engine(&[
            "CREATE TABLE a (id INT, val INT)",
            "CREATE TABLE b (id INT, val INT)",
            "INSERT INTO a VALUES (1, 100), (2, 200)",
            "INSERT INTO b VALUES (1, 10), (2, 20)",
        ]);
        let r = e
            .execute(
                "SELECT id, (SELECT b.val FROM b WHERE b.id = a.id) AS bval FROM a ORDER BY id",
            )
            .unwrap();
        assert_eq!(r.rows.len(), 2, "expected 2 rows (one per a.id)");
        // Row 1: a.id=1 → bval=10
        // Row 2: a.id=2 → bval=20
        assert_eq!(r.rows[0][0], Value::Integer(1));
        assert_eq!(r.rows[0][1], Value::Integer(10));
        assert_eq!(r.rows[1][0], Value::Integer(2));
        assert_eq!(r.rows[1][1], Value::Integer(20));
    }

    #[test]
    fn correlated_scalar_with_no_match_returns_null() {
        // a.id=3 has no matching b → bval must be NULL, not empty/missing.
        let mut e = fresh_engine(&[
            "CREATE TABLE a (id INT, val INT)",
            "CREATE TABLE b (id INT, val INT)",
            "INSERT INTO a VALUES (1, 100), (2, 200), (3, 300)",
            "INSERT INTO b VALUES (1, 10), (2, 20)",
        ]);
        let r = e
            .execute(
                "SELECT id, (SELECT b.val FROM b WHERE b.id = a.id) AS bval FROM a ORDER BY id",
            )
            .unwrap();
        assert_eq!(r.rows.len(), 3);
        // Row 3: id=3 → no match → NULL
        assert_eq!(r.rows[2][0], Value::Integer(3));
        assert_eq!(r.rows[2][1], Value::Null, "missing b match must yield NULL");
    }

    #[test]
    fn correlated_scalar_with_outer_filter() {
        // WHERE outer filter combined with correlated subquery — covers the
        // binder passing outer scope into inner correctly.
        let mut e = fresh_engine(&[
            "CREATE TABLE a (id INT, val INT)",
            "CREATE TABLE b (id INT, val INT)",
            "INSERT INTO a VALUES (1, 100), (2, 200), (3, 300)",
            "INSERT INTO b VALUES (1, 10), (2, 20)",
        ]);
        let r = e
            .execute(
                "SELECT id, (SELECT b.val FROM b WHERE b.id = a.id) AS bval FROM a WHERE a.val > 150 ORDER BY id",
            )
            .unwrap();
        // a.val > 150 matches (2, 200) and (3, 300) → 2 rows.
        assert_eq!(r.rows.len(), 2, "a.val > 150 matches 2 rows");
        assert_eq!(r.rows[0][0], Value::Integer(2));
        assert_eq!(r.rows[0][1], Value::Integer(20));
        assert_eq!(r.rows[1][0], Value::Integer(3));
        assert_eq!(r.rows[1][1], Value::Null, "id=3 has no b match → NULL");
    }
}
