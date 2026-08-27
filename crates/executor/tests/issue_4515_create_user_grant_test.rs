//! Integration tests for Issue #4515 — `CREATE USER` / `DROP USER` +
//! the existing `GRANT` / `REVOKE` / `SHOW GRANTS FOR` lifecycle.
//!
//! Scope of #4515:
//!   - `CREATE USER 'name'@'host'` registers a user with the catalog's
//!     `AuthManager` (verbatim repro).
//!   - `CREATE USER 'name'@'host' IDENTIFIED BY 'pwd'` stores the
//!     password hash verbatim.
//!   - Re-creating an existing user errors (mirrors `AuthManager`'s
//!     `DuplicateUser` contract).
//!   - `DROP USER 'name'@'host'` removes the registration.
//!   - `DROP USER IF EXISTS` on a missing user is a no-op.
//!   - `DROP USER` without `IF EXISTS` on a missing user errors.
//!   - Round-trip: CREATE USER → SHOW GRANTS FOR → GRANT SELECT →
//!     SHOW GRANTS FOR → REVOKE SELECT → SHOW GRANTS FOR (no rows).
//!
//! Out of scope (pre-existing, not introduced by #4515):
//!   - `ALTER USER` already returns
//!     "ALTER USER not yet implemented" from the dispatch table.
//!   - Privileges on roles / grant chains are not exercised (issue
//!     only requires the user CRUD surface).
//!
//! Verification is observational via the `AuthManager` exposed on the
//! catalog — we read the user set directly.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let catalog = Arc::new(RwLock::new(sqlrustgo_catalog::Catalog::new("default")));
    ExecutionEngine::with_catalog(storage, catalog)
}

/// Look up whether `name@host` is registered. Returns the password
/// hash if present (None otherwise).
fn user_password(e: &ExecutionEngine<MemoryStorage>, name: &str, host: &str) -> Option<String> {
    let catalog_arc = e.catalog();
    let catalog_guard = catalog_arc.as_ref().unwrap().read();
    catalog_guard
        .auth_manager()
        .list_users()
        .iter()
        .find(|u| u.identity.username == name && u.identity.host == host)
        .map(|info| info.password_hash.clone())
}

#[test]
fn issue_4515_repro_create_user_at_localhost() {
    let mut e = fresh_engine();
    e.execute("CREATE USER ex_user@localhost")
        .expect("CREATE USER 'ex_user'@'localhost' must succeed");
    let pwd = user_password(&e, "ex_user", "localhost");
    assert!(
        pwd.is_some(),
        "ex_user@localhost should be registered after CREATE USER"
    );
}

#[test]
fn issue_4515_create_user_with_string_literal_and_password() {
    let mut e = fresh_engine();
    e.execute("CREATE USER 'alice'@'%' IDENTIFIED BY 'secret'")
        .expect("CREATE USER with quoted identity + IDENTIFIED BY must succeed");
    let pwd = user_password(&e, "alice", "%");
    assert_eq!(
        pwd,
        Some("secret".to_string()),
        "password_hash must be stored verbatim"
    );
}

#[test]
fn issue_4515_default_host_is_localhost() {
    // No host token at all — defaults to 'localhost' per `parse_alter_user`
    // semantics we mirror.
    let mut e = fresh_engine();
    e.execute("CREATE USER bob")
        .expect("CREATE USER bob (default host) must succeed");
    assert!(
        user_password(&e, "bob", "localhost").is_some(),
        "no-host CREATE USER must register bob@localhost"
    );
}

#[test]
fn issue_4515_create_user_duplicate_errors() {
    let mut e = fresh_engine();
    e.execute("CREATE USER carol@localhost").unwrap();
    let res = e.execute("CREATE USER carol@localhost");
    assert!(
        res.is_err(),
        "creating an existing user twice must error (DuplicateUser contract)"
    );
    let msg = format!("{}", res.unwrap_err()).to_lowercase();
    assert!(
        msg.contains("user") && (msg.contains("exists") || msg.contains("duplicate")),
        "error message should mention user/duplicate; got: {}",
        msg
    );
}

#[test]
fn issue_4515_drop_user_removes_registration() {
    let mut e = fresh_engine();
    e.execute("CREATE USER drop_me@localhost").unwrap();
    assert!(user_password(&e, "drop_me", "localhost").is_some());

    e.execute("DROP USER drop_me@localhost")
        .expect("DROP USER must remove the registration");
    assert!(
        user_password(&e, "drop_me", "localhost").is_none(),
        "drop_me@localhost must be gone after DROP USER"
    );
}

#[test]
fn issue_4515_drop_user_if_exists_missing_is_noop() {
    let mut e = fresh_engine();
    e.execute("DROP USER IF EXISTS ghost@localhost")
        .expect("DROP USER IF EXISTS on a missing user must be a no-op");
}

#[test]
fn issue_4515_drop_user_missing_without_if_exists_errors() {
    let mut e = fresh_engine();
    let res = e.execute("DROP USER ghost@localhost");
    assert!(
        res.is_err(),
        "DROP USER without IF EXISTS on a missing user must error"
    );
}

#[test]
fn issue_4515_full_lifecycle_create_grant_show_revoke_drop() {
    // Round-trip: CREATE USER → SHOW GRANTS (0 rows) → GRANT SELECT →
    // SHOW GRANTS (1 row) → REVOKE → SHOW GRANTS (0 rows) → DROP USER.
    let mut e = fresh_engine();
    e.execute("CREATE USER lifecycle@localhost").unwrap();

    // Pre-grant: 0 grants
    let pre = e
        .execute("SHOW GRANTS FOR lifecycle@localhost")
        .expect("SHOW GRANTS FOR must parse + execute");
    assert_eq!(
        pre.rows.len(),
        0,
        "newly-created user should have no grants yet"
    );

    e.execute("GRANT SELECT ON t TO lifecycle@localhost")
        .expect("GRANT SELECT must succeed");

    let mid = e
        .execute("SHOW GRANTS FOR lifecycle@localhost")
        .expect("SHOW GRANTS FOR must parse + execute");
    assert_eq!(
        mid.rows.len(),
        1,
        "after GRANT SELECT, the user must have exactly one grant row"
    );

    e.execute("REVOKE SELECT ON t FROM lifecycle@localhost")
        .expect("REVOKE SELECT must succeed");

    let post = e
        .execute("SHOW GRANTS FOR lifecycle@localhost")
        .expect("SHOW GRANTS FOR must parse + execute");
    assert_eq!(
        post.rows.len(),
        0,
        "after REVOKE, the user must have no grants"
    );

    e.execute("DROP USER lifecycle@localhost")
        .expect("DROP USER at end of lifecycle must succeed");
}
