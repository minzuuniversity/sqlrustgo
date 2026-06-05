//! Q11 deep - check intermediate steps

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::sync::{Arc, RwLock};

const FIXTURE: &str = "/home/openclaw/sqlrustgo-tpch/data";

fn load(storage: &Arc<RwLock<MemoryStorage>>, tbl: &str, ncols: usize) -> usize {
    let path = format!("{}/{}.tbl", FIXTURE, tbl);
    let content = fs::read_to_string(&path).expect("read");
    const BATCH_SIZE: usize = 10000;
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split('|').collect();
        if values.len() < ncols {
            continue;
        }
        let record: Vec<SqlValue> = values[..ncols]
            .iter()
            .map(|v| {
                let s = v.trim();
                if s.is_empty() {
                    SqlValue::Null
                } else if let Ok(i) = s.parse::<i64>() {
                    SqlValue::Integer(i)
                } else if let Ok(f) = s.parse::<f64>() {
                    SqlValue::Float(f)
                } else {
                    SqlValue::Text(s.to_string())
                }
            })
            .collect();
        batch.push(record);
        if batch.len() >= BATCH_SIZE {
            let mut s = storage.write().unwrap();
            let _ = s.insert(tbl, batch.clone());
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut s = storage.write().unwrap();
        let _ = s.insert(tbl, batch.clone());
        count += batch.len();
    }
    count
}

#[test]
fn diag_q11_steps() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))").unwrap();
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)").unwrap();
    load(&storage, "partsupp", 5);
    load(&storage, "supplier", 7);
    load(&storage, "nation", 4);

    // Step 1: just join supplier (1st JOIN)
    let r1 = engine.execute("SELECT COUNT(*) FROM partsupp INNER JOIN supplier ON ps_suppkey = s_suppkey").unwrap();
    eprintln!("Step1 1-join: rows.len()={}, first={:?}", r1.rows.len(), r1.rows.first());

    // Step 2: full 3-way
    let r2 = engine.execute("SELECT COUNT(*) FROM partsupp INNER JOIN supplier ON ps_suppkey = s_suppkey INNER JOIN nation ON s_nationkey = n_nationkey").unwrap();
    eprintln!("Step2 2-joins: rows.len()={}, first={:?}", r2.rows.len(), r2.rows.first());

    // Step 3: 3-way comma-list
    let r3 = engine.execute("SELECT COUNT(*) FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey").unwrap();
    eprintln!("Step3 3-way comma: rows.len()={}, first={:?}", r3.rows.len(), r3.rows.first());

    // Step 4: add GROUP BY without HAVING
    let r4 = engine.execute("SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY' GROUP BY ps_partkey").unwrap();
    eprintln!("Step4 group_by: rows.len()={}, first={:?}", r4.rows.len(), r4.rows.first());
}
