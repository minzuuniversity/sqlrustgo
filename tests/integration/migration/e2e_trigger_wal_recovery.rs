//! E2E WAL Trigger Recovery — T-001, T-002, T-003
//!
//! Validates trigger DML flows through WalStorage and survives crash.
//!
//! T-001: INSERT via trigger → WAL → Crash → Recovery → Value Correct
//! T-002: UPDATE via trigger → WAL → Crash → Recovery → Updated Value Correct
//! T-003: DELETE via trigger → WAL → Crash → Recovery → Row Absent
//!
//! Phase 2a migration: drive every SQL through the canonical
//! `start_ephemeral` MySQL server. Each test boots a fresh server
//! per phase so the engine-drop "crash" is honest — the new
//! server reads the data from the shared data_dir (which the
//! `EphemeralConfig::data_dir` field makes possible; see PR #3049).

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use tempfile::TempDir;

fn open(data_dir: &std::path::Path) -> MySqlTestClient {
    let cfg = EphemeralConfig {
        host: "127.0.0.1".to_string(),
        bootstrap_tables: false,
        bootstrap_users: true,
        data_dir: Some(data_dir.to_path_buf()),
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 16,
        storage: None,
        metrics_port: None,

        ..Default::default()
    };
    let handle = start_ephemeral(cfg).expect("start_ephemeral");
    MySqlTestClient::connect_handle(handle).expect("MySqlTestClient::connect_handle")
}

#[test]
fn test_trigger_insert_wal_recovery_t001() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let mut client = open(&data_dir);

        client
            .exec("CREATE TABLE t1 (id INTEGER, value INTEGER)")
            .unwrap();
        client
            .exec("CREATE TABLE t1_audit (id INTEGER, orig_value INTEGER)")
            .unwrap();

        client
            .exec(
                "CREATE TRIGGER t1_insert_audit BEFORE INSERT ON t1 FOR EACH ROW BEGIN INSERT INTO t1_audit VALUES (NEW.id, NEW.value) END",
            )
            .unwrap();

        client.exec("BEGIN").unwrap();
        client.exec("INSERT INTO t1 VALUES (1, 100)").unwrap();
        client.exec("COMMIT").unwrap();

        let count = client
            .query_one_i64("SELECT COUNT(*) FROM t1_audit")
            .unwrap();
        assert_eq!(
            count, 1,
            "T-001 pre-check: trigger should have inserted audit row"
        );
    }

    let mut client = open(&data_dir);

    let audit_count = client
        .query_one_i64("SELECT COUNT(*) FROM t1_audit")
        .unwrap();
    assert_eq!(
        audit_count, 1,
        "T-001 FAIL: audit row missing after recovery — trigger INSERT did not survive crash"
    );

    let main_count = client
        .query_one_i64("SELECT COUNT(*) FROM t1 WHERE id = 1")
        .unwrap();
    assert_eq!(main_count, 1, "T-001 FAIL: main row missing after recovery");

    println!("T-001 PASS: Trigger INSERT survived crash + recovery");
}

#[test]
fn test_trigger_update_wal_recovery_t002() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let mut client = open(&data_dir);

        client
            .exec("CREATE TABLE t1 (id INTEGER, value INTEGER)")
            .unwrap();
        client
            .exec(
                "CREATE TRIGGER t1_update_log BEFORE UPDATE ON t1 FOR EACH ROW BEGIN UPDATE t1 SET value = NEW.value WHERE id = OLD.id END",
            )
            .unwrap();

        client.exec("BEGIN").unwrap();
        client.exec("INSERT INTO t1 VALUES (1, 100)").unwrap();
        client.exec("COMMIT").unwrap();
    }

    {
        let mut client = open(&data_dir);

        client.exec("BEGIN").unwrap();
        client
            .exec("UPDATE t1 SET value = 999 WHERE id = 1")
            .unwrap();
        client.exec("COMMIT").unwrap();

        let rows = client
            .query_rows("SELECT value FROM t1 WHERE id = 1")
            .unwrap();
        assert_eq!(
            rows[0][0], "999",
            "T-002 pre-check: UPDATE should set value=999"
        );
    }

    let mut client = open(&data_dir);

    let rows = client
        .query_rows("SELECT value FROM t1 WHERE id = 1")
        .unwrap();
    assert_eq!(
        rows[0][0], "999",
        "T-002 FAIL: expected value=999, got {} — UPDATE via trigger did not survive crash",
        rows[0][0]
    );

    println!("T-002 PASS: Trigger UPDATE survived crash + recovery");
}

#[test]
fn test_trigger_delete_wal_recovery_t003() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let mut client = open(&data_dir);

        client
            .exec("CREATE TABLE t1 (id INTEGER, value INTEGER)")
            .unwrap();
        client
            .exec(
                "CREATE TRIGGER t1_delete_backup BEFORE DELETE ON t1 FOR EACH ROW BEGIN INSERT INTO t1 SELECT * FROM t1 WHERE id = old.id END",
            )
            .unwrap();

        client.exec("BEGIN").unwrap();
        client.exec("INSERT INTO t1 VALUES (1, 100)").unwrap();
        client.exec("INSERT INTO t1 VALUES (2, 200)").unwrap();
        client.exec("COMMIT").unwrap();
    }

    {
        let mut client = open(&data_dir);

        client.exec("BEGIN").unwrap();
        client.exec("DELETE FROM t1 WHERE id = 1").unwrap();
        client.exec("COMMIT").unwrap();

        let count = client
            .query_one_i64("SELECT COUNT(*) FROM t1 WHERE id = 1")
            .unwrap();
        assert_eq!(count, 0, "T-003 pre-check: row id=1 should be deleted");
    }

    let mut client = open(&data_dir);

    let count = client
        .query_one_i64("SELECT COUNT(*) FROM t1 WHERE id = 1")
        .unwrap();
    assert_eq!(
        count, 0,
        "T-003 FAIL: row id=1 still present after recovery — DELETE via trigger did not survive crash"
    );

    let count2 = client
        .query_one_i64("SELECT COUNT(*) FROM t1 WHERE id = 2")
        .unwrap();
    assert_eq!(count2, 1, "T-003 FAIL: row id=2 should still exist");

    println!("T-003 PASS: Trigger DELETE survived crash + recovery");
}
