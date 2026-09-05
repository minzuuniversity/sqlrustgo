use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;
fn e() -> ExecutionEngine<MemoryStorage> {
    let s = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(s)
}

#[test]
fn test_rollup_two_col() {
    let mut x = e();
    x.execute("CREATE TABLE t (region TEXT, product TEXT, sales INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES ('N','A',10),('N','B',20),('S','A',30),('S','B',40)")
        .unwrap();
    let r = x
        .execute("SELECT region, product, SUM(sales) FROM t GROUP BY region, product WITH ROLLUP")
        .unwrap();
    // 2x2 = 4 detail + 2 region subtotals + 1 grand total = 7
    assert!(
        r.rows.len() >= 5,
        "expected at least 5 rows, got {}",
        r.rows.len()
    );
}

#[test]
fn test_cube_two_col() {
    let mut x = e();
    x.execute("CREATE TABLE t (region TEXT, product TEXT, sales INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES ('N','A',10),('N','B',20),('S','A',30),('S','B',40)")
        .unwrap();
    let r = x
        .execute("SELECT region, product, SUM(sales) FROM t GROUP BY region, product WITH CUBE")
        .unwrap();
    // 2^2 = 4 subsets, but original 2x2=4 detail rows + others
    assert!(
        r.rows.len() >= 5,
        "expected at least 5 rows, got {}",
        r.rows.len()
    );
}

#[test]
fn test_rollup_grand_total() {
    let mut x = e();
    x.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES ('a',1),('a',2),('b',3)")
        .unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY g WITH ROLLUP")
        .unwrap();
    // 2 detail + 1 grand total = 3
    let grand_total = r
        .rows
        .iter()
        .find(|row| matches!(&row[0], sqlrustgo::Value::Null));
    assert!(
        grand_total.is_some(),
        "expected grand total with NULL in group col"
    );
}

#[test]
fn test_cube_subset_total() {
    let mut x = e();
    x.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES ('a',1),('b',2)").unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY g WITH CUBE")
        .unwrap();
    // 2 detail (a=1, b=2) + 1 grand (NULL,3) = 3
    assert!(
        r.rows.len() >= 3,
        "expected at least 3 rows, got {}",
        r.rows.len()
    );
}

// V312-86 / Issue #4758: GROUP BY ROLLUP(cols) function-call syntax
// (SQL standard / MySQL 5.7). Previously the parser treated ROLLUP(...)
// as a scalar function call inside the GROUP BY expression list, which
// caused the grouping-set semantics to be silently dropped — the
// function evaluated to NULL per row and the result was a single total.
#[test]
fn test_rollup_function_single_col() {
    let mut x = e();
    x.execute("CREATE TABLE t (g INTEGER, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(2,30),(2,40)")
        .unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY ROLLUP(g)")
        .unwrap();
    // 2 detail (g=1 → 30, g=2 → 70) + 1 grand total (NULL → 100) = 3 rows
    assert_eq!(
        r.rows.len(),
        3,
        "ROLLUP(g) should emit 2 detail + 1 grand total; got {}",
        r.rows.len()
    );
}

#[test]
fn test_rollup_function_two_cols() {
    let mut x = e();
    x.execute("CREATE TABLE t (region TEXT, product TEXT, sales INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES ('N','A',10),('N','B',20),('S','A',30),('S','B',40)")
        .unwrap();
    let r = x
        .execute("SELECT region, product, SUM(sales) FROM t GROUP BY ROLLUP(region, product)")
        .unwrap();
    // 2x2=4 detail + 2 region subtotals + 1 grand total = 7 rows
    assert_eq!(
        r.rows.len(),
        7,
        "ROLLUP(region, product) should emit 7 rows; got {}",
        r.rows.len()
    );
}

#[test]
fn test_cube_function_two_cols() {
    let mut x = e();
    x.execute("CREATE TABLE t (region TEXT, product TEXT, sales INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES ('N','A',10),('N','B',20),('S','A',30),('S','B',40)")
        .unwrap();
    let r = x
        .execute("SELECT region, product, SUM(sales) FROM t GROUP BY CUBE(region, product)")
        .unwrap();
    // 2^2=4 subsets: (region,product), (region), (product), () =
    // 4 detail + 2 region-subtotals + 2 product-subtotals + 1 grand = 9 rows
    assert_eq!(
        r.rows.len(),
        9,
        "CUBE(region, product) should emit 9 rows; got {}",
        r.rows.len()
    );
}

#[test]
fn test_rollup_function_grand_total_value() {
    let mut x = e();
    x.execute("CREATE TABLE t (g INTEGER, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(2,30),(2,40)")
        .unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY ROLLUP(g)")
        .unwrap();
    // Last row should have NULL in g column and SUM=100
    let last = r.rows.last().expect("at least one row");
    assert!(
        matches!(&last[0], sqlrustgo::Value::Null),
        "grand total row should have NULL in g, got {:?}",
        last[0]
    );
    assert_eq!(last[1], sqlrustgo::Value::Integer(100));
}

#[test]
fn test_rollup_function_mixed_with_regular_col() {
    let mut x = e();
    x.execute("CREATE TABLE t (a INTEGER, g INTEGER, v INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1,1,10),(1,2,20),(2,1,30),(2,2,40)")
        .unwrap();
    // MySQL 5.7 semantics: GROUP BY a, ROLLUP(g) is treated as
    // ROLLUP(a, g) — the regular prefix col joins the ROLLUP list.
    // = 4 detail + 2 a-subtotals (NULL g) + 1 grand total = 7 rows.
    let r = x
        .execute("SELECT a, g, SUM(v) FROM t GROUP BY a, ROLLUP(g)")
        .unwrap();
    assert_eq!(
        r.rows.len(),
        7,
        "GROUP BY a, ROLLUP(g) should emit 7 rows (MySQL 5.7 ROLLUP semantics); got {}",
        r.rows.len()
    );
}

// V312-87 / Issue #4758: ORDER BY ... NULLS FIRST / NULLS LAST on a
// ROLLUP/CUBE query must place subtotal rows (which carry NULL in the
// dropped group column) where the SQL asked, not at the default
// Value::Null < 全部 discriminant position. Without this fix, all four
// NULLS variants produced identical output (NULL always first by
// accidental discriminant order).
#[test]
fn test_rollup_order_by_nulls_first() {
    let mut x = e();
    x.execute("CREATE TABLE t (g INTEGER, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(2,30),(2,40)")
        .unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY g WITH ROLLUP ORDER BY g NULLS FIRST")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "ROLLUP should emit 3 rows");
    // First row must be the grand total (g IS NULL)
    assert!(
        matches!(&r.rows[0][0], sqlrustgo::Value::Null),
        "ORDER BY g NULLS FIRST: grand total should be first; got g={:?}",
        r.rows[0][0]
    );
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Integer(100));
}

#[test]
fn test_rollup_order_by_nulls_last() {
    let mut x = e();
    x.execute("CREATE TABLE t (g INTEGER, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(2,30),(2,40)")
        .unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY g WITH ROLLUP ORDER BY g NULLS LAST")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "ROLLUP should emit 3 rows");
    // Last row must be the grand total (g IS NULL)
    let last = r.rows.last().expect("at least one row");
    assert!(
        matches!(&last[0], sqlrustgo::Value::Null),
        "ORDER BY g NULLS LAST: grand total should be last; got g={:?}",
        last[0]
    );
    assert_eq!(last[1], sqlrustgo::Value::Integer(100));
    // Detail rows must be in ascending g order
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(1));
    assert_eq!(r.rows[1][0], sqlrustgo::Value::Integer(2));
}

#[test]
fn test_rollup_order_by_desc_nulls_first() {
    let mut x = e();
    x.execute("CREATE TABLE t (g INTEGER, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(2,30),(2,40)")
        .unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY g WITH ROLLUP ORDER BY g DESC NULLS FIRST")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "ROLLUP should emit 3 rows");
    // First row must be the grand total (g IS NULL)
    assert!(
        matches!(&r.rows[0][0], sqlrustgo::Value::Null),
        "ORDER BY g DESC NULLS FIRST: grand total should be first; got g={:?}",
        r.rows[0][0]
    );
    // Detail rows must be in descending g order
    assert_eq!(r.rows[1][0], sqlrustgo::Value::Integer(2));
    assert_eq!(r.rows[2][0], sqlrustgo::Value::Integer(1));
}

#[test]
fn test_rollup_order_by_desc_nulls_last() {
    let mut x = e();
    x.execute("CREATE TABLE t (g INTEGER, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(2,30),(2,40)")
        .unwrap();
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY g WITH ROLLUP ORDER BY g DESC NULLS LAST")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "ROLLUP should emit 3 rows");
    // Detail rows must be in descending g order
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Integer(2));
    assert_eq!(r.rows[1][0], sqlrustgo::Value::Integer(1));
    // Last row must be the grand total (g IS NULL)
    let last = r.rows.last().expect("at least one row");
    assert!(
        matches!(&last[0], sqlrustgo::Value::Null),
        "ORDER BY g DESC NULLS LAST: grand total should be last; got g={:?}",
        last[0]
    );
}

#[test]
fn test_cube_order_by_nulls_first() {
    let mut x = e();
    x.execute("CREATE TABLE t (g INTEGER, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(1,20),(2,30),(2,40)")
        .unwrap();
    // CUBE(g) is equivalent to ROLLUP(g) for a single column
    let r = x
        .execute("SELECT g, SUM(v) FROM t GROUP BY g WITH CUBE ORDER BY g NULLS FIRST")
        .unwrap();
    assert_eq!(r.rows.len(), 3, "CUBE should emit 3 rows");
    assert!(
        matches!(&r.rows[0][0], sqlrustgo::Value::Null),
        "ORDER BY g NULLS FIRST on CUBE: grand total should be first; got g={:?}",
        r.rows[0][0]
    );
}
