//! Regression test for V312-48 issue #4278: canonical TPC-H Q16 NOT IN subquery
//! returns the wrong row count due to a thread-local flag leak between the
//! outer comma-join query and its non-correlated NOT IN subquery.
//!
//! Root cause (PR #4357+followup): the hash-chain fast path in
//! `try_comma_join_hash_chain` set a thread-local
//! `COMMA_JOIN_WHERE_CONSUMED=true` flag. When step 1.6 of the WHERE pipeline
//! (non-correlated IN/NOT IN rewrite) recursively called `execute_select` for
//! the subquery, the subquery inherited the parent's `skip_where=true` and
//! skipped its own `WHERE s_comment LIKE '%Customer%Complaints%'` filter.
//! This produced a `NotInList` of all 10 000 supplier keys, which eliminated
//! every partsupp row → 0 results.
//!
//! Fix: reset `COMMA_JOIN_WHERE_CONSUMED` to `false` at the start of every
//! `execute_select` call so each invocation's WHERE handling is independent.
//!
//! Expected baseline (SQLite, sf1): 18 314 rows.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::sync::Arc;

const FIXTURE: &str = "/home/openclaw/sqlrustgo_work/tests/data/tpch-sf01";

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
            let mut s = storage.write();
            let _ = s.insert(tbl, batch.clone());
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut s = storage.write();
        let _ = s.insert(tbl, batch.clone());
        count += batch.len();
    }
    count
}

#[test]
fn q16_canonical_subquery_only() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)").unwrap();
    load(&storage, "supplier", 7);

    let r = engine
        .execute("SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%Customer%Complaints%' ORDER BY s_suppkey")
        .unwrap();
    assert!(
        r.rows.len() >= 4,
        "expected >=4 Customer+Complaints matches, got {}",
        r.rows.len()
    );
}

#[test]
fn q16_canonical_notin_full() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))").unwrap();
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)").unwrap();
    load(&storage, "part", 9);
    load(&storage, "partsupp", 5);
    load(&storage, "supplier", 7);

    let sql = "SELECT p_brand, p_type, p_size, COUNT(DISTINCT ps_suppkey) AS supplier_cnt \
               FROM partsupp, part \
               WHERE p_partkey = ps_partkey \
                 AND p_brand <> 'Brand#45' \
                 AND p_type NOT LIKE 'MEDIUM POLISHED%' \
                 AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9) \
                 AND ps_suppkey NOT IN (\
                   SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%Customer%Complaints%'\
                 ) \
               GROUP BY p_brand, p_type, p_size \
               ORDER BY supplier_cnt DESC, p_brand, p_type, p_size";
    let r = engine.execute(sql).unwrap();
    // SQLite sf1 baseline: 18314 rows.
    assert_eq!(
        r.rows.len(),
        18314,
        "Q16 row count must match SQLite baseline (18314)"
    );
}
