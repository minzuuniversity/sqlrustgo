//! PR-SHOW-TABLES — P1 backlog fix for v3.7.0
//!
//! **Issue**: `SHOW TABLES` returned "Unsupported statement type" because
//! `ExecutionEngine::execute()` had no match arm for `Statement::Show`.
//! See docs/releases/v3.7.0/GA_GAP_REPORT.md §3.1.
//!
//! **Fix**: Add `Statement::Show` dispatch + `execute_show_databases` and
//! `execute_show_tables` handlers using `StorageEngine::list_tables()`.
//!
//! **Phase 2a migration**: driven through the wire protocol via the
//! embedded `start_ephemeral` harness (see
//! `openspec/changes/mysql-server-canonical-entry/specs/wire-protocol-execution/spec.md`).
//!
//! **Test isolation**: every test passes its own `tempfile::TempDir` as
//! `EphemeralConfig::data_dir`. Without an explicit path, `start_ephemeral`
//! falls through to the shared `${cwd}/.sqlrustgo/data/` default, which
//! leaks the catalog of the previous test (the first run of
//! `show_tables_on_empty_db_returns_empty_result` would observe
//! `t1/t2/t3/keep_me/repro_t` left behind by earlier tests). The TempDir
//! pattern keeps every ephemeral server in its own directory; Drop on
//! the TempDir removes it.
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::EphemeralConfig;
use tempfile::TempDir;

fn clean_client() -> (TempDir, MySqlTestClient) {
    let dir = TempDir::new().expect("create tempdir for ephemeral server");
    let client = MySqlTestClient::connect_with_config(EphemeralConfig {
        bootstrap_tables: false,
        data_dir: Some(dir.path().to_path_buf()),
        ..EphemeralConfig::default()
    })
    .expect("ephemeral server (clean catalog) + raw client should come up");
    (dir, client)
}

#[test]
fn show_tables_on_empty_db_returns_empty_result() {
    let (_dir, mut client) = clean_client();
    let rows = client
        .query_rows("SHOW TABLES")
        .expect("SHOW TABLES should succeed");
    assert_eq!(
        rows.len(),
        0,
        "SHOW TABLES on empty DB should return 0 rows, got {}",
        rows.len()
    );
}

#[test]
fn show_tables_lists_all_created_tables() {
    let (_dir, mut client) = clean_client();
    client
        .exec("CREATE TABLE t1 (id INTEGER)")
        .expect("CREATE t1");
    client
        .exec("CREATE TABLE t2 (id INTEGER, name TEXT)")
        .expect("CREATE t2");
    client
        .exec("CREATE TABLE t3 (id INTEGER)")
        .expect("CREATE t3");

    let rows = client
        .query_rows("SHOW TABLES")
        .expect("SHOW TABLES should succeed");
    assert_eq!(rows.len(), 3, "expected 3 tables, got {:?}", rows);
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(names.contains(&"t1"));
    assert!(names.contains(&"t2"));
    assert!(names.contains(&"t3"));
}

#[test]
fn show_databases_returns_one_row() {
    let (_dir, mut client) = clean_client();
    let rows = client
        .query_rows("SHOW DATABASES")
        .expect("SHOW DATABASES should succeed");
    assert!(
        !rows.is_empty(),
        "SHOW DATABASES should return at least one row, got 0"
    );
}

#[test]
fn show_tables_after_drop_reflects_drop() {
    let (_dir, mut client) = clean_client();
    client
        .exec("CREATE TABLE keep_me (id INTEGER)")
        .expect("CREATE keep_me");
    client
        .exec("CREATE TABLE drop_me (id INTEGER)")
        .expect("CREATE drop_me");
    client.exec("DROP TABLE drop_me").expect("DROP drop_me");

    let rows = client
        .query_rows("SHOW TABLES")
        .expect("SHOW TABLES should succeed");
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    assert!(names.contains(&"keep_me"));
    assert!(
        !names.contains(&"drop_me"),
        "drop_me should not appear after DROP"
    );
}
