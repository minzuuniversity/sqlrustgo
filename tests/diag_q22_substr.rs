//! Q22 parse error - isolate which part fails

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;

#[test]
fn diag_q22_substr() {
    let storage = std::sync::Arc::new(std::sync::RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT, c_phone TEXT, c_acctbal REAL)").unwrap();
    engine.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT)").unwrap();

    // Just SUBSTR in SELECT
    let r = engine.execute("SELECT SUBSTR(c_phone, 1, 2) AS cntrycode FROM customer");
    eprintln!("SUBSTR in SELECT: {:?}", r.as_ref().err());

    // SUBSTR in WHERE
    let r2 = engine.execute("SELECT c_custkey FROM customer WHERE SUBSTR(c_phone, 1, 2) = '13'");
    eprintln!("SUBSTR in WHERE: {:?}", r2.as_ref().err());

    // NOT EXISTS subquery
    let r3 = engine.execute("SELECT c_custkey FROM customer WHERE NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)");
    eprintln!("NOT EXISTS: {:?}", r3.as_ref().err());
}
