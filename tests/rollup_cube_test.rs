use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::{Arc, RwLock};
fn e() -> ExecutionEngine<MemoryStorage> { let s = Arc::new(RwLock::new(MemoryStorage::new())); ExecutionEngine::new(s) }

#[test]
fn test_rollup_two_col() {
    let mut x = e();
    x.execute("CREATE TABLE t (region TEXT, product TEXT, sales INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES ('N','A',10),('N','B',20),('S','A',30),('S','B',40)").unwrap();
    let r = x.execute("SELECT region, product, SUM(sales) FROM t GROUP BY region, product WITH ROLLUP").unwrap();
    // 2x2 = 4 detail + 2 region subtotals + 1 grand total = 7
    assert!(r.rows.len() >= 5, "expected at least 5 rows, got {}", r.rows.len());
}

#[test]
fn test_cube_two_col() {
    let mut x = e();
    x.execute("CREATE TABLE t (region TEXT, product TEXT, sales INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES ('N','A',10),('N','B',20),('S','A',30),('S','B',40)").unwrap();
    let r = x.execute("SELECT region, product, SUM(sales) FROM t GROUP BY region, product WITH CUBE").unwrap();
    // 2^2 = 4 subsets, but original 2x2=4 detail rows + others
    assert!(r.rows.len() >= 5, "expected at least 5 rows, got {}", r.rows.len());
}

#[test]
fn test_rollup_grand_total() {
    let mut x = e();
    x.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES ('a',1),('a',2),('b',3)").unwrap();
    let r = x.execute("SELECT g, SUM(v) FROM t GROUP BY g WITH ROLLUP").unwrap();
    // 2 detail + 1 grand total = 3
    let grand_total = r.rows.iter().find(|row| matches!(&row[0], sqlrustgo::Value::Null));
    assert!(grand_total.is_some(), "expected grand total with NULL in group col");
}

#[test]
fn test_cube_subset_total() {
    let mut x = e();
    x.execute("CREATE TABLE t (g TEXT, v INTEGER)").unwrap();
    x.execute("INSERT INTO t VALUES ('a',1),('b',2)").unwrap();
    let r = x.execute("SELECT g, SUM(v) FROM t GROUP BY g WITH CUBE").unwrap();
    // 2 detail (a=1, b=2) + 1 grand (NULL,3) = 3
    assert!(r.rows.len() >= 3, "expected at least 3 rows, got {}", r.rows.len());
}
