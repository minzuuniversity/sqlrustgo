// V313-97 / Issue #4679: GROUP BY GROUPING SETS((...),(...),...) support.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn setup_t_grp_val(x: &mut ExecutionEngine<MemoryStorage>, rows: &[(i64, i64)]) {
    x.execute("CREATE TABLE t(grp INT, val INT)").unwrap();
    for (g, v) in rows {
        x.execute(&format!("INSERT INTO t VALUES ({}, {})", g, v))
            .unwrap();
    }
}

#[test]
fn grouping_sets_with_grand_total() {
    let mut x = fresh_mem();
    setup_t_grp_val(&mut x, &[(1, 10), (1, 20), (2, 30)]);
    let r = x
        .execute(
            "SELECT grp, SUM(val) FROM t \
             GROUP BY GROUPING SETS((grp), ()) \
             ORDER BY grp IS NULL, grp",
        )
        .expect("GROUPING SETS((grp),()) should execute");
    assert_eq!(
        r.rows.len(),
        3,
        "expected 3 rows (2 grp + 1 grand total), got {:?}",
        r.rows
    );
}

#[test]
fn grouping_sets_empty_alone() {
    let mut x = fresh_mem();
    setup_t_grp_val(&mut x, &[(1, 10), (2, 20)]);
    let r = x
        .execute("SELECT SUM(val) FROM t GROUP BY GROUPING SETS(())")
        .expect("GROUPING SETS(()) should execute");
    assert_eq!(
        r.rows.len(),
        1,
        "expected 1 grand-total row, got {:?}",
        r.rows
    );
}

#[test]
fn grouping_sets_single_column_is_subset() {
    let mut x = fresh_mem();
    setup_t_grp_val(&mut x, &[(1, 10), (2, 20)]);
    let r = x
        .execute("SELECT grp, SUM(val) FROM t GROUP BY GROUPING SETS((grp))")
        .expect("GROUPING SETS((grp)) should execute");
    assert_eq!(r.rows.len(), 2, "expected 2 rows, got {:?}", r.rows);
}
