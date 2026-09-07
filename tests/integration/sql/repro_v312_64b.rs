//! Reproduction / regression tests for the v312-64b semantic-fixes batch.
//!
//! Closes the executor / semantic side of:
//!   - #4650 — GROUP_CONCAT aggregator + ORDER BY + SEPARATOR
//!   - #4657 — HAVING with multiple AND / subquery predicate
//!   - #4659 — EXTRACT(YEAR/MONTH/DAY FROM date) output format
//!
//! #4656 (`> ALL` / `= ANY` quantified subquery operators) was originally
//! in this batch but was dropped before PR #4690 after V312-66 / #4687
//! (commit d18482e246) shipped the same fix via
//! `pre_evaluate_quantified_subquery` + Step 1.6. #4687 supersedes #4656.
//! The two issues describe the same root cause, and merging both would
//! cause the correlated-quantified regression in
//! `v312_66_quantified_correlated_does_not_panic` because the two paths
//! (this batch's `eval_predicate_with_subq_full` + #4687's
//! `pre_evaluate_quantified_subquery`) consume the same correlated
//! QuantifiedOp AST differently.
//!
//! These remaining issues all live in the executor's expression
//! evaluator (`src/engine_select.rs` + `crates/executor/src/expr/mod.rs`).
//! Unlike v312-64a (which was parser-only), this batch needs to touch
//! the aggregate dispatch path AND the predicate evaluator.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

// ============================================================================
// Issue #4650 — GROUP_CONCAT aggregator with ORDER BY + SEPARATOR
// ============================================================================

#[test]
fn repro_4650_group_concat_aggregates_with_order_by_separator() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(grp int, val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(1,30),(2,5),(2,15)")
        .unwrap();
    let r = x
        .execute("SELECT GROUP_CONCAT(val ORDER BY val SEPARATOR ',') FROM t")
        .expect("GROUP_CONCAT must execute");
    assert_eq!(
        r.rows.len(),
        1,
        "GROUP_CONCAT without GROUP BY must aggregate to 1 row (got {})",
        r.rows.len()
    );
    assert_eq!(
        format!("{:?}", r.rows[0][0]),
        "Text(\"5,10,15,20,30\")",
        "ORDER BY val ASC must sort, SEPARATOR ',' must separate"
    );
}

#[test]
fn repro_4650_group_concat_with_group_by() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(grp int, val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(1,30),(2,5),(2,15)")
        .unwrap();
    let r = x
        .execute("SELECT grp, GROUP_CONCAT(val ORDER BY val SEPARATOR '|') FROM t GROUP BY grp")
        .expect("GROUP_CONCAT with GROUP BY must execute");
    assert_eq!(r.rows.len(), 2, "expected 2 groups");
    // Group order is not guaranteed without explicit ORDER BY (HashMap
    // iteration), so collect by grp value and assert each group has the
    // expected concat result.
    let mut by_grp: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    for row in &r.rows {
        let grp = match &row[0] {
            sqlrustgo::Value::Integer(n) => *n,
            other => panic!("grp column should be Integer, got {:?}", other),
        };
        let concat = match &row[1] {
            sqlrustgo::Value::Text(s) => s.clone(),
            other => panic!("GROUP_CONCAT column should be Text, got {:?}", other),
        };
        by_grp.insert(grp, concat);
    }
    assert_eq!(
        by_grp.get(&1).map(String::as_str),
        Some("10|20|30"),
        "group 1 should concat to 10|20|30 (got {:?})",
        by_grp.get(&1)
    );
    assert_eq!(
        by_grp.get(&2).map(String::as_str),
        Some("5|15"),
        "group 2 should concat to 5|15 (got {:?})",
        by_grp.get(&2)
    );
}

#[test]
fn repro_4650_group_concat_bare_form() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(val int)").unwrap();
    x.execute("INSERT INTO t VALUES (30),(10),(20)").unwrap();
    let r = x
        .execute("SELECT GROUP_CONCAT(val) FROM t")
        .expect("bare GROUP_CONCAT must run");
    assert_eq!(r.rows.len(), 1);
    // bare form uses default ',' separator and input order
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"30,10,20\")");
}

// ============================================================================
// Issue #4657 — HAVING with multiple AND conditions / subquery
// ============================================================================

#[test]
fn repro_4657_having_with_and_predicate() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE orders(cust varchar(20), amt int)")
        .unwrap();
    x.execute(
        "INSERT INTO orders VALUES ('alice', 100), ('alice', 50), ('bob', 200), ('bob', 150), ('carol', 300)",
    )
    .unwrap();
    let r = x
        .execute("SELECT cust FROM orders GROUP BY cust HAVING count(*) >= 2 AND sum(amt) > 100")
        .expect("HAVING with AND must execute");
    let custs: Vec<String> = r
        .rows
        .iter()
        .filter_map(|r| match &r[0] {
            sqlrustgo::Value::Text(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        r.rows.len(),
        2,
        "alice(150) and bob(350) match: got {:?}",
        custs
    );
    assert!(custs.iter().any(|c| c.contains("alice")));
    assert!(custs.iter().any(|c| c.contains("bob")));
}

#[test]
fn repro_4657_having_with_subquery() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE orders(cust varchar(20), amt int)")
        .unwrap();
    x.execute(
        "INSERT INTO orders VALUES ('alice', 100), ('alice', 50), ('bob', 200), ('bob', 150), ('carol', 300)",
    )
    .unwrap();
    let r = x
        .execute("SELECT cust FROM orders GROUP BY cust HAVING count(*) > (SELECT count(*) FROM orders WHERE cust = 'carol')")
        .expect("HAVING with subquery must execute");
    let custs: Vec<String> = r
        .rows
        .iter()
        .filter_map(|r| match &r[0] {
            sqlrustgo::Value::Text(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        r.rows.len(),
        2,
        "alice(count=2) and bob(count=2) > carol(count=1): got {:?}",
        custs
    );
}

// ============================================================================
// Issue #4659 — EXTRACT(YEAR/MONTH/DAY FROM date) output format
// ============================================================================

#[test]
fn repro_4659_extract_year_per_row() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(dt date)").unwrap();
    x.execute("INSERT INTO t VALUES ('2026-09-01'), ('2026-09-15'), ('2026-10-01')")
        .unwrap();
    let r = x
        .execute("SELECT EXTRACT(YEAR FROM dt) FROM t")
        .expect("EXTRACT YEAR must run");
    assert_eq!(r.rows.len(), 3, "expected 3 rows, got {}", r.rows.len());
    for (i, row) in r.rows.iter().enumerate() {
        assert_eq!(
            format!("{:?}", row[0]),
            "Text(\"2026\")",
            "row {} must be 2026",
            i
        );
    }
}

#[test]
fn repro_4659_extract_year_month_combined() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(dt date)").unwrap();
    x.execute("INSERT INTO t VALUES ('2026-09-01'), ('2026-09-15'), ('2026-10-01')")
        .unwrap();
    let r = x
        .execute("SELECT EXTRACT(YEAR FROM dt), EXTRACT(MONTH FROM dt) FROM t")
        .expect("EXTRACT YEAR+MONTH must run");
    assert_eq!(r.rows.len(), 3);
    let expected: Vec<(&str, &str)> = vec![("2026", "09"), ("2026", "09"), ("2026", "10")];
    for (i, row) in r.rows.iter().enumerate() {
        assert_eq!(
            format!("{:?}", row[0]),
            format!("Text(\"{}\")", expected[i].0)
        );
        assert_eq!(
            format!("{:?}", row[1]),
            format!("Text(\"{}\")", expected[i].1)
        );
    }
}
