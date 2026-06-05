//! Q11 isolated - check WHERE filter result count

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
fn diag_q11_where_only() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))").unwrap();
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)").unwrap();
    let n1 = load(&storage, "partsupp", 5);
    let n2 = load(&storage, "supplier", 7);
    let n3 = load(&storage, "nation", 4);
    eprintln!("loaded: partsupp={}, supplier={}, nation={}", n1, n2, n3);

    // Step 1: just 3-way join without WHERE - should produce cross product
    let r1 = engine.execute("SELECT COUNT(*) FROM partsupp, supplier, nation").unwrap();
    eprintln!("Q11 3-way cross: rows.len() = {} (expected {})", r1.rows.len(), r1.rows.first().and_then(|r| r.first().cloned()).unwrap_or(sqlrustgo_types::Value::Null));

    // Step 2: with WHERE filter (Germany)
    let r2 = engine.execute("SELECT COUNT(*) FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY'").unwrap();
    eprintln!("Q11 with WHERE: rows.len() = {} (expected 8000-ish)", r2.rows.len());

    // Step 3: explicit JOIN
    let r3 = engine.execute("SELECT COUNT(*) FROM partsupp INNER JOIN supplier ON ps_suppkey = s_suppkey INNER JOIN nation ON s_nationkey = n_nationkey WHERE n_name = 'GERMANY'").unwrap();
    eprintln!("Q11 explicit JOIN: rows.len() = {}", r3.rows.len());
}
