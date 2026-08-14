//! E2E WAL Trigger Recovery — T-001, T-002, T-003 + V55D
//!
//! Validates trigger DML flows through WalStorage and survives crash.
//!
//! T-001: INSERT via trigger → WAL → Crash → Recovery → Value Correct
//! T-002: UPDATE via trigger → WAL → Crash → Recovery → Updated Value Correct
//! T-003: DELETE via trigger → WAL → Crash → Recovery → Row Absent
//! V55D (Round-26): BEGIN + INSERT (trigger fires) + ROLLBACK → both base
//!                   and audit empty; re-open via shared data_dir simulates
//!                   kill -9 → replay must show the same empty state
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

// =============================================================================
// V312-55D (Round-26) — Trigger 事务 + WAL + recovery
//
// 规范要求:
//   BEGIN 内 AFTER INSERT 写 audit + ROLLBACK 后 base/audit 均为空;
//   kill -9 replay 一致。
//
// 命名: `test_trigger_after_insert_rollback_v55d` — 唯一带 `v55d` 子串的
// 测试 (gate V55D-WAL-Recovery 用 `trigger_after_insert_rollback_v55d` 子串
// 过滤, 严格匹配 1-test)。
// =============================================================================

#[test]
fn test_trigger_after_insert_rollback_v55d() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Phase A: 在第一段 server 中执行 BEGIN + INSERT + ROLLBACK, 触发器
    // 是 AFTER INSERT, 每向 base 写入一行都会触发 INSERT INTO audit。
    {
        let mut client = open(&data_dir);

        client
            .exec("CREATE TABLE base (id INTEGER, val TEXT)")
            .unwrap();
        client
            .exec("CREATE TABLE audit (id INTEGER, msg TEXT)")
            .unwrap();
        client
            .exec(
                "CREATE TRIGGER base_audit AFTER INSERT ON base FOR EACH ROW BEGIN INSERT INTO audit VALUES (1, 'logged') END",
            )
            .unwrap();

        // 事务边界 + ROLLBACK
        client.exec("BEGIN").unwrap();
        client.exec("INSERT INTO base VALUES (1, 'orig')").unwrap();
        client.exec("ROLLBACK").unwrap();

        // pre-check: ROLLBACK 后两张表都应为空, trigger 副作用必须随父事务回滚
        let base_count = client.query_one_i64("SELECT COUNT(*) FROM base").unwrap();
        assert_eq!(
            base_count, 0,
            "V55D pre-check: base must be empty after ROLLBACK (got {})",
            base_count
        );

        let audit_count = client.query_one_i64("SELECT COUNT(*) FROM audit").unwrap();
        assert_eq!(
            audit_count, 0,
            "V55D pre-check: audit must be empty after ROLLBACK — trigger side-effect MUST roll back with the parent transaction (got {})",
            audit_count
        );
    }

    // Phase B: kill -9 replay — 重新启动 server, 共享 data_dir, 验证 ROLLBACK
    // 决定持久化到磁盘且重启后一致 (与 COMMIT 路径形成对比)。
    let mut client = open(&data_dir);

    let base_count = client.query_one_i64("SELECT COUNT(*) FROM base").unwrap();
    assert_eq!(
        base_count, 0,
        "V55D FAIL: base has {} rows after recovery — ROLLBACK did not survive crash",
        base_count
    );

    let audit_count = client.query_one_i64("SELECT COUNT(*) FROM audit").unwrap();
    assert_eq!(
        audit_count, 0,
        "V55D FAIL: audit has {} rows after recovery — trigger side-effect did not roll back",
        audit_count
    );

    println!("V55D PASS: BEGIN + INSERT + ROLLBACK semantics survive crash + recovery");
}
