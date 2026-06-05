//! Diagnostic: 2-table comma-list vs explicit JOIN
//!
//! Tests whether `customer, orders WHERE c_custkey = o_custkey` returns
//! the same count as `customer INNER JOIN orders ON c_custkey = o_custkey`
//! on canonical SF=0.01 data.

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const FIXTURE: &str = "/home/openclaw/sqlrustgo-tpch/data";

fn load_tbl_file(
    storage: &Arc<RwLock<MemoryStorage>>,
    tbl_name: &str,
    tbl_path: &PathBuf,
    columns: usize,
) -> Result<usize, String> {
    let content = fs::read_to_string(tbl_path)
        .map_err(|e| format!("Cannot read {}: {}", tbl_path.display(), e))?;

    const BATCH_SIZE: usize = 10000;
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split('|').collect();
        if values.len() < columns {
            continue;
        }
        let record: Vec<SqlValue> = values[..columns]
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
            let mut storage = storage.write().map_err(|e| format!("Lock error: {}", e))?;
            storage
                .insert(tbl_name, batch.clone())
                .map_err(|e| format!("Insert error: {}", e))?;
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut storage = storage.write().map_err(|e| format!("Lock error: {}", e))?;
        storage
            .insert(tbl_name, batch.clone())
            .map_err(|e| format!("Insert error: {}", e))?;
        count += batch.len();
    }
    Ok(count)
}

#[test]
fn diag_2t_comma_vs_explicit_join() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    // Engine shares the storage so the loader's writes are visible to queries.
    let mut engine = ExecutionEngine::new(storage.clone());

    // Use the standard TPC-H column order matching canonical .tbl files
    engine
        .execute(
            "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT, c_address TEXT, \
             c_nationkey INTEGER, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT)",
        )
        .expect("create customer");
    engine
        .execute(
            "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, \
             o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, \
             o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
        )
        .expect("create orders");

    let cust_n = load_tbl_file(
        &storage,
        "customer",
        &PathBuf::from(format!("{}/customer.tbl", FIXTURE)),
        8,
    )
    .expect("load customer");
    let ord_n = load_tbl_file(
        &storage,
        "orders",
        &PathBuf::from(format!("{}/orders.tbl", FIXTURE)),
        9,
    )
    .expect("load orders");
    eprintln!("customer={}, orders={}", cust_n, ord_n);

    // 1-table check
    let r0 = engine.execute("SELECT COUNT(*) FROM customer").unwrap();
    let cust_count = match &r0.rows[0][0] {
        SqlValue::Integer(n) => *n,
        v => panic!("unexpected: {:?}", v),
    };
    let r0b = engine.execute("SELECT COUNT(*) FROM orders").unwrap();
    let ord_count = match &r0b.rows[0][0] {
        SqlValue::Integer(n) => *n,
        v => panic!("unexpected: {:?}", v),
    };
    eprintln!("1-table customer count via SQL: {}", cust_count);
    eprintln!("1-table orders count via SQL: {}", ord_count);

    // 2-table direct: try with explicit c_custkey=o_custkey (no extra qual)
    let r1 = engine
        .execute("SELECT COUNT(*) FROM customer, orders WHERE c_custkey = o_custkey")
        .expect("comma-list");
    let comma_count = match &r1.rows[0][0] {
        SqlValue::Integer(n) => *n,
        v => panic!("unexpected: {:?}", v),
    };
    eprintln!("comma-list: {}", comma_count);

    // Explicit JOIN
    let r2 = engine
        .execute("SELECT COUNT(*) FROM customer INNER JOIN orders ON c_custkey = o_custkey")
        .expect("explicit JOIN");
    let explicit_count = match &r2.rows[0][0] {
        SqlValue::Integer(n) => *n,
        v => panic!("unexpected: {:?}", v),
    };
    eprintln!("explicit JOIN: {}", explicit_count);

    // SQLite reference: in canonical SF=0.01, customer=1500, orders=15000,
    // and there's 1:1 c_custkey:o_custkey mapping (each customer has ≥1
    // order). So expected = 15000 (one row per order).
    let expected = 15000;

    eprintln!("expected: {}", expected);
    eprintln!(
        "delta comma-list: {} ({}%)",
        comma_count - expected,
        100.0 * (comma_count - expected) as f64 / expected as f64
    );
    eprintln!(
        "delta explicit:   {} ({}%)",
        explicit_count - expected,
        100.0 * (explicit_count - expected) as f64 / expected as f64
    );
}
