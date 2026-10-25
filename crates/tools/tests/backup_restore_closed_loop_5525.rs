//! #5525 — the backup → restore → **verify the data** closed loop.
//!
//! WHY THIS FILE EXISTS
//! ---------------------
//! The Owner directive of 2026-10-25 (hub issue #5, comment 198163) lists
//! "#5525 备份假成功" as one of five **P0 data-safety red lines**, and its
//! acceptance criterion is explicit: "验收=真实恢复/真实并发场景证据，禁止
//! '仅不报错'". The same directive asks, for pull request #5587, that an
//! **independent reviewer** actually run a "备份→恢复→验证数据" closed loop
//! and that "仅不报错" does not count as passing.
//!
//! "Restore returned without an error" is not evidence of a restore. So
//! this file proves, with data:
//!
//!   1. a backup of a non-empty table set writes **real artifacts** — the
//!      `.sql` contains the table names *and* the row values, the
//!      `.data.json` is a non-empty payload — not an empty file that is
//!      then reported as a success;
//!   2. the restored content is **equal** to the source content
//!      (`BackupContent: PartialEq`, i.e. a deep comparison of every
//!      column, every row and every value — not a row count);
//!   3. a restore into a **real storage engine** lands the same rows:
//!      `restore_into` creates the tables, inserts the rows and verifies
//!      the landed row count, and this test then reads the rows back and
//!      compares them to the source rows, value by value;
//!   4. the three ways the previous implementation could report a **false
//!      success** are all refused: an empty table set, a corrupted
//!      payload, and an overwrite of an existing table.
//!
//! Every assertion in this file is a comparison against the source data.
//! There is no assertion of the shape "the call did not fail" anywhere,
//! because that is exactly the shape of evidence the directive rejects.
//!
//! The closed loop is also runnable by a reviewer with one command:
//!
//! ```bash
//! bash scripts/gate/check_backup_restore_closed_loop.sh
//! ```

use std::fs;

use sqlrustgo_storage::engine::{MemoryStorage, StorageEngine};
use sqlrustgo_storage::Value;
use sqlrustgo_tools::backup_restore::{BackupColumn, BackupManager, BackupTable};
use tempfile::TempDir;

const DB: &str = "closedloop";

/// Two tables, several rows, mixed value types.
///
/// The rows are deliberately **not** uniform: a loop that only ever sees
/// one integer column cannot tell a real restore from a row count.
fn source_tables() -> Vec<BackupTable> {
    let mut users = BackupTable::new(
        "users",
        vec![
            BackupColumn::new("id", "INTEGER"),
            BackupColumn::new("name", "VARCHAR(64)"),
            BackupColumn::new("active", "BOOLEAN"),
        ],
    );
    users.rows = vec![
        vec![
            Value::Integer(1),
            Value::Text("alice".to_string()),
            Value::Boolean(true),
        ],
        vec![
            Value::Integer(2),
            Value::Text("bob".to_string()),
            Value::Boolean(false),
        ],
        vec![
            Value::Integer(3),
            Value::Text("carol".to_string()),
            Value::Boolean(true),
        ],
    ];

    let mut orders = BackupTable::new(
        "orders",
        vec![
            BackupColumn::new("id", "INTEGER"),
            BackupColumn::new("user_id", "INTEGER"),
            BackupColumn::new("amount", "DOUBLE"),
        ],
    );
    orders.rows = vec![
        vec![Value::Integer(100), Value::Integer(1), Value::Float(12.5)],
        vec![Value::Integer(101), Value::Integer(2), Value::Float(0.25)],
    ];

    vec![users, orders]
}

fn total_rows(tables: &[BackupTable]) -> usize {
    tables.iter().map(|t| t.rows.len()).sum()
}

/// The closed loop itself: backup → artifacts → restore → compare → land.
#[test]
fn closed_loop_backup_restore_verifies_the_data() {
    let tmp = TempDir::new().expect("temp dir");
    let backup_dir = tmp.path().join("backups");
    let mgr = BackupManager::new(backup_dir.clone());

    let source = source_tables();
    let expected_rows = total_rows(&source);

    // --- 1. backup ---------------------------------------------------------
    let meta = mgr
        .create_backup(DB, &source)
        .expect("backup of a non-empty table set must succeed");
    assert_eq!(
        meta.tables.len(),
        2,
        "the backup must record both tables, not an empty table set"
    );
    assert!(meta.tables.contains(&"users".to_string()));
    assert!(meta.tables.contains(&"orders".to_string()));

    // --- 2. the artifacts are real ----------------------------------------
    let sql_path = backup_dir.join(format!("{}.sql", meta.id));
    let payload_path = backup_dir.join(format!("{}.data.json", meta.id));
    assert!(sql_path.is_file(), "the .sql artifact must exist");
    assert!(payload_path.is_file(), "the .data.json payload must exist");

    let sql = fs::read_to_string(&sql_path).expect("read the .sql artifact");
    assert!(
        sql.contains("users") && sql.contains("orders"),
        "the dump must name the tables it claims to hold"
    );
    assert!(
        sql.contains("alice") && sql.contains("carol") && sql.contains("12.5"),
        "the dump must carry the row values, not only the schema"
    );
    let payload_bytes = fs::read(&payload_path).expect("read the payload");
    assert!(
        payload_bytes.len() > 0,
        "an empty payload is the signature of the #5516 false success"
    );
    let payload_checksum = meta
        .payload_checksum
        .clone()
        .expect("the payload must carry its own checksum");
    assert_eq!(
        payload_checksum.len(),
        64,
        "the payload checksum is a SHA-256 hex digest"
    );

    // --- 3. restore returns the source content, deep-compared -------------
    let restored = mgr
        .restore(&meta.id)
        .expect("restore of a backup just written must succeed");
    assert_eq!(
        restored.database, DB,
        "the restored content must name the database it came from"
    );
    assert_eq!(
        restored.tables, source,
        "the restored content must equal the source content value by value"
    );
    assert_eq!(
        total_rows(&restored.tables),
        expected_rows,
        "the restored row count must match the source"
    );

    // --- 4. the restore lands in a real storage engine --------------------
    let mut storage = MemoryStorage::new();
    let result = mgr
        .restore_into(&meta.id, &mut storage, DB, false)
        .expect("restore into a fresh storage must succeed");
    assert_eq!(result.tables_restored, 2, "both tables must be created");
    assert_eq!(
        result.rows_restored as usize, expected_rows,
        "the rows that landed must be the rows the backup promised"
    );

    for table in &source {
        let landed = storage
            .scan_in_db(DB, &table.name)
            .expect("read the restored table back");
        assert_eq!(
            landed.len(),
            table.rows.len(),
            "table `{}` must hold exactly the backed-up rows",
            table.name
        );
        // The storage returns the rows in insertion order for this engine;
        // compare them as sets of rows so the assertion is about the data
        // and not about the order the engine happens to return.
        let mut expected: Vec<Vec<Value>> = table.rows.clone();
        let mut actual: Vec<Vec<Value>> = landed.iter().cloned().collect();
        expected.sort();
        actual.sort();
        assert_eq!(
            actual, expected,
            "the rows that landed in table `{}` must equal the backed-up rows",
            table.name
        );
    }

    // The evidence line an independent reviewer can quote.
    println!(
        "EVIDENCE closed_loop backup_id={} tables={} rows={} payload_sha256={}",
        meta.id,
        result.tables_restored,
        result.rows_restored,
        payload_checksum
    );
}

/// The empty table set must be refused — this is the #5516 false success.
#[test]
fn empty_table_set_is_refused_instead_of_reported_as_success() {
    let tmp = TempDir::new().expect("temp dir");
    let mgr = BackupManager::new(tmp.path().join("backups"));

    let err = mgr
        .create_backup(DB, &[])
        .expect_err("a backup with no table must be an error, not an empty success");
    let message = err.to_lowercase();
    assert!(
        message.contains("no tables") || message.contains("empty"),
        "the error must say that there was nothing to back up, got: {err}"
    );

    // And nothing was written: a refused backup must not leave an artifact
    // behind that a later step could mistake for a successful one.
    let dir = tmp.path().join("backups");
    let leftovers: Vec<_> = fs::read_dir(&dir)
        .expect("read the backup dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".sql") || n.ends_with(".data.json"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "a refused backup must not leave artifacts behind, found: {leftovers:?}"
    );
}

/// A corrupted payload must be detected **before** it is restored.
///
/// Two different kinds of damage are checked, because they are detected by
/// two different guards and a restore that survives either one would be a
/// false success:
///
///  * a **semantic** corruption — the payload is still valid JSON, but its
///    content no longer matches the recorded SHA-256. This is the case the
///    envelope checksum exists for, and the error must name the checksum;
///  * a **structural** corruption — a byte is flipped, so the payload no
///    longer parses. The restore must refuse it too, for a different reason.
#[test]
fn corrupted_payload_is_detected_before_restore() {
    let tmp = TempDir::new().expect("temp dir");
    let backup_dir = tmp.path().join("backups");
    let mgr = BackupManager::new(backup_dir.clone());

    let meta = mgr
        .create_backup(DB, &source_tables())
        .expect("backup must succeed");
    let payload_path = backup_dir.join(format!("{}.data.json", meta.id));

    // --- semantic corruption: valid JSON, wrong content --------------------
    let raw = fs::read_to_string(&payload_path).expect("read the payload");
    let corrupted = raw.replace("alice", "mallory");
    assert_ne!(
        raw, corrupted,
        "the payload must carry the value the corruption targets"
    );
    fs::write(&payload_path, &corrupted).expect("write the corrupted payload");

    let err = mgr
        .restore(&meta.id)
        .expect_err("a payload whose content changed must not restore");
    let message = err.to_lowercase();
    assert!(
        message.contains("checksum") && message.contains("corrupt"),
        "the error must name the checksum mismatch, got: {err}"
    );

    // --- structural corruption: the payload no longer parses ---------------
    let mut bytes = fs::read(&payload_path).expect("read the payload");
    let position = bytes.len() / 2;
    bytes[position] = if bytes[position] == b'a' { b'b' } else { b'a' };
    fs::write(&payload_path, &bytes).expect("write the corrupted payload");

    let err = mgr
        .restore(&meta.id)
        .expect_err("a payload that no longer parses must not restore");
    let message = err.to_lowercase();
    assert!(
        message.contains("parse") || message.contains("checksum") || message.contains("corrupt"),
        "the error must say that the payload is unusable, got: {err}"
    );
}

/// The restore must not overwrite an existing table unless asked to.
#[test]
fn restore_refuses_to_overwrite_an_existing_table() {
    let tmp = TempDir::new().expect("temp dir");
    let mgr = BackupManager::new(tmp.path().join("backups"));

    let meta = mgr
        .create_backup(DB, &source_tables())
        .expect("backup must succeed");

    let mut storage = MemoryStorage::new();
    // First restore creates the tables.
    mgr.restore_into(&meta.id, &mut storage, DB, false)
        .expect("the first restore must succeed");
    assert!(storage.has_table_in(DB, "users"));

    // The second one must refuse rather than silently replace data.
    let err = mgr
        .restore_into(&meta.id, &mut storage, DB, false)
        .expect_err("a restore over an existing table must be refused");
    assert!(
        err.to_lowercase().contains("already exists"),
        "the error must say that the table is already there, got: {err}"
    );

    // With drop_first it is allowed, and the data is still correct.
    let result = mgr
        .restore_into(&meta.id, &mut storage, DB, true)
        .expect("restore with drop_first must succeed");
    assert_eq!(result.rows_restored as usize, 5);
    let landed = storage.scan_in_db(DB, "users").expect("read back");
    assert_eq!(landed.len(), 3, "the dropped-and-restored table must hold its rows");
}
