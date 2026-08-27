//! Reproduction for Issue #4514 — CREATE TRIGGER parsed + invoked in
//! DML commit hook (FOR EACH ROW).
//!
//! Issue verbatim:
//!
//! ```sql
//! CREATE TABLE t(a int);
//! CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW UPDATE student SET phone=phone WHERE 1=0;
//! -- actual: Error: Parse error: Expected Begin, got Update
//! ```

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

#[test]
fn issue_repro_create_trigger_after_insert_parses() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut e = ExecutionEngine::new(storage);
    e.execute("CREATE TABLE t(a int)").unwrap();
    e.execute("CREATE TABLE student(name TEXT, phone TEXT)")
        .unwrap();

    let res = e.execute(
        "CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW UPDATE student SET phone=phone WHERE 1=0"
    );
    match res {
        Ok(_) => {}
        Err(e) => panic!("CREATE TRIGGER should parse, got: {}", e),
    }
}
