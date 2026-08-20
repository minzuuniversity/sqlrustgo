//! Q22 even simpler

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

#[test]
fn diag_q22_minimal() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute("CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_phone TEXT)")
        .unwrap();

    let r = engine.execute("SELECT SUBSTR(c_phone, 1, 2) FROM customer");
    eprintln!("SUBSTR FROM: {:?}", r.err());

    let r2 = engine.execute("SELECT SUBSTR('hello', 1, 2) FROM customer");
    eprintln!("SUBSTR literal: {:?}", r2.err());

    let r3 = engine.execute("SELECT SUBSTR(c_phone, 1, 2) AS x FROM customer");
    eprintln!("SUBSTR AS: {:?}", r3.err());
}
