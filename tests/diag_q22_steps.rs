//! Q22 isolated - step by step

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};

#[test]
fn diag_q22_steps() {
    let storage = std::sync::Arc::new(std::sync::RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT, c_phone TEXT, c_acctbal REAL)").unwrap();
    engine.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT)").unwrap();

    // Inner subquery only
    let inner = "SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, c_acctbal FROM customer WHERE SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17') AND c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00 AND SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)";
    eprintln!("inner: {:?}", engine.execute(inner).err());

    // Just SUBSTR with alias
    let r2 = engine.execute("SELECT SUBSTR(c_phone, 1, 2) AS cntrycode FROM customer");
    eprintln!("SUBSTR AS alias: {:?}", r2.err());

    // Just SUBSTR in WHERE for a few rows
    let r3 = engine.execute("SELECT c_custkey FROM customer WHERE SUBSTR(c_phone, 1, 2) IN ('13')");
    eprintln!("SUBSTR in WHERE IN: {:?}", r3.err());
}
