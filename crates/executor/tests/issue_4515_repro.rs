//! Reproduction for Issue #4515 — `CREATE USER 'name'@'host'` parses
//! and registers a user (previously hit a parse error because the
//! `parse_create` dispatcher did not recognise `Token::User`).
//!
//! Issue verbatim:
//!
//! ```sql
//! CREATE USER ex_user@localhost;
//! -- actual: Parse error: Expected TABLE, INDEX, PROCEDURE, TRIGGER, ROLE,
//! --        VIEW, SEQUENCE, or DATABASE after CREATE, got User
//! ```

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

#[test]
fn issue_repro_create_user_parses_and_executes() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut e = ExecutionEngine::new(storage);

    // Verbatim repro: must no longer hit the parse error.
    let res = e.execute("CREATE USER ex_user@localhost");
    match res {
        Ok(_) => {}
        Err(err) => panic!("CREATE USER should parse + execute, got: {}", err),
    }
}
