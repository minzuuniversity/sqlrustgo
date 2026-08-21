//! HAVING Filter Execution Tests — V312-58 Q11/Q12 (#4377/#4378)
//!
//! Reproduces the #4377 bug: TPC-H Q11 with arithmetic inside SUM
//! returns all 200,000 groups instead of filtering via HAVING.
//!
//! The existing `aggregate_having` test in aggregate_smoke_test.rs
//! covers the SIMPLE case (SUM(v) > 5 with no arithmetic). This file
//! extends coverage to:
//!   - SUM(arithmetic) — bug reported in #4377
//!   - HAVING with ArithmeticOp on aggregate result
//!   - Multiple groups where only some pass the predicate
//!
//! Related: openspec/changes/v312-58-q11-q12-having-filter/

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Reproduce #4377: HAVING SUM(arithmetic) > threshold.
///
/// Pre-fix bug: HAVING is silently dropped, so all 3 groups are returned.
#[test]
fn test_having_sum_arithmetic_filters_groups() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (g INTEGER, a INTEGER, b INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 10, 10), (1, 20, 5), (2, 20, 100), (3, 5, 10)")
        .unwrap();

    let r = x
        .execute(
            "SELECT g, SUM(a*b) AS v FROM t GROUP BY g \
             HAVING SUM(a*b) > 1000 ORDER BY g",
        )
        .expect("SELECT");

    assert_eq!(
        r.rows.len(),
        1,
        "HAVING must filter groups; expected 1 row (g=2), got {} rows. \
         This is the #4377 bug if rows.len() > 1.",
        r.rows.len()
    );
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(2));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Integer(2000));
}

/// Q11-like pattern: arithmetic in SUM inside HAVING, with 2 groups.
#[test]
fn test_having_q11_pattern_two_groups() {
    let mut x = fresh();
    x.execute("CREATE TABLE sales (region TEXT, qty INTEGER, price INTEGER)")
        .unwrap();
    x.execute("INSERT INTO sales VALUES ('north', 1, 1), ('south', 10, 100)")
        .unwrap();

    let r = x
        .execute(
            "SELECT region, SUM(qty*price) AS revenue FROM sales \
             GROUP BY region \
             HAVING SUM(qty*price) > 500 \
             ORDER BY revenue DESC",
        )
        .expect("SELECT");

    assert_eq!(
        r.rows.len(),
        1,
        "expected 1 row (south); got {} rows. If 2 rows: HAVING was \
         dropped (Q11/#4377 bug).",
        r.rows.len()
    );
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("south".into()));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Integer(1000));
}

/// Confirm parser wires Q11's SELECT aggregate and HAVING aggregate
/// consistently — both must be Expression::Aggregate (not FunctionCall).
#[test]
fn test_q11_parser_aggregates_consistent() {
    use sqlrustgo_parser::{parse, AggregateFunction, Expression, Statement};

    let q11 = "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
               FROM partsupp, supplier, nation \
               WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' \
               GROUP BY ps_partkey \
               HAVING SUM(ps_supplycost * ps_availqty) > 10000 \
               ORDER BY part_value DESC";

    let stmt = parse(q11).expect("parse Q11");
    let select = match stmt {
        Statement::Select(s) => s,
        _ => panic!("expected SELECT"),
    };

    assert!(!select.aggregates.is_empty(), "Q11 must populate aggregates");
    assert_eq!(select.aggregates[0].func, AggregateFunction::Sum);
    assert!(select.having.is_some(), "Q11 must have HAVING");

    let mut found_aggregate = false;
    fn walk(expr: &Expression, found: &mut bool) {
        match expr {
            Expression::Aggregate(_) => *found = true,
            Expression::BinaryOp(l, _, r) => {
                walk(l, found);
                walk(r, found);
            }
            Expression::UnaryOp(_, e)
            | Expression::IsNull(e)
            | Expression::IsNotNull(e) => walk(e, found),
            _ => {}
        }
    }
    walk(select.having.as_ref().unwrap(), &mut found_aggregate);
    assert!(
        found_aggregate,
        "HAVING must contain an Aggregate node so eval_aggregate_lookup \
         can resolve via the synthetic schema"
    );
}