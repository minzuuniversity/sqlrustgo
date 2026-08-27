use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

#[test]
fn repro_issue_4517_avg_over_partition_by() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut e = ExecutionEngine::new(storage);

    e.execute("CREATE TABLE score (studentno TEXT, courseno TEXT, final INTEGER)")
        .unwrap();
    e.execute("INSERT INTO score VALUES ('a', 'c1', 90), ('b', 'c1', 88), ('c', 'c2', 92), ('d', 'c2', 91)").unwrap();

    let res = e
        .execute("SELECT studentno, courseno, avg(final) OVER (PARTITION BY courseno) FROM score")
        .unwrap();
    println!("rows: {}", res.rows.len());
    for row in &res.rows {
        println!("{:?}", row);
    }
}
