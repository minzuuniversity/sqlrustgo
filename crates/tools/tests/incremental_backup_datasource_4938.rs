//! #4938 — backups must read the caller's database, not a demo fixture.
//!
//! Complements `incremental_backup_e2e_test.rs` (added separately for
//! AC2/AC3). That file exercises the backup functions on a tempdir, but
//! both entry points used to ignore their `data_dir` argument and export
//! `create_demo_storage()` — a hard-coded in-memory `users` table. A
//! caller could point the tool at a real data directory, get a clean
//! exit and a plausible manifest, and walk away holding a copy of the
//! demo data.
//!
//! These tests therefore assert the one thing the other file cannot:
//! that what comes out corresponds to what the caller actually put in
//! `data_dir`.

use sqlrustgo_storage::{FileStorage, StorageEngine, TableInfo, Value};
use sqlrustgo_tools::backup::{create_full_backup, create_incremental_backup};
use std::path::{Path, PathBuf};

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("b4938src_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Populate `data_dir` with a table whose contents no demo fixture would
/// ever produce.
fn seed_real_data(data_dir: &Path) {
    let mut storage = FileStorage::new(data_dir.to_path_buf()).expect("open FileStorage");
    let info = TableInfo {
        name: "widgets".to_string(),
        columns: vec![
            sqlrustgo_storage::ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                primary_key: true,
                ..Default::default()
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "label".to_string(),
                data_type: "TEXT".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    storage.create_table(&info).expect("create table");
    storage
        .insert(
            "widgets",
            vec![vec![
                Value::Integer(1),
                Value::Text("alpha-one".to_string()),
            ]],
        )
        .expect("insert 1");
    storage
        .insert(
            "widgets",
            vec![vec![Value::Integer(2), Value::Text("beta-two".to_string())]],
        )
        .expect("insert 2");

    // FileStorage keeps recent writes in an insert buffer; `scan` — which
    // is what the backup exporter uses — reads the committed rows. Without
    // this flush the backup would be empty, and the test would "prove" the
    // exporter drops data when in fact nothing was persisted yet.
    storage.flush().expect("flush inserts");
}

fn exported_sql(backup: &Path) -> String {
    let data = backup.join("data");
    let mut out = String::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&data)
        .expect("data dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "sql").unwrap_or(false))
        .collect();
    entries.sort();
    for p in entries {
        out.push_str(&std::fs::read_to_string(&p).unwrap());
    }
    out
}

#[test]
fn full_backup_exports_the_callers_database() {
    let dir = tmp("full");
    let data_dir = dir.join("data_dir");
    std::fs::create_dir_all(&data_dir).unwrap();
    seed_real_data(&data_dir);

    let backup = dir.join("full");
    create_full_backup(&backup, "sql", &data_dir).expect("full backup");

    let sql = exported_sql(&backup);
    assert!(
        sql.contains("widgets"),
        "#4938: the backup must contain the caller's table, not the demo \
         `users` table; got:\n{sql}"
    );
    assert!(
        sql.contains("alpha-one") && sql.contains("beta-two"),
        "#4938: the caller's rows must be present; got:\n{sql}"
    );
    assert!(
        !sql.contains("CREATE TABLE users"),
        "#4938: demo data must no longer leak into a real backup; got:\n{sql}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn incremental_backup_exports_the_callers_database() {
    let dir = tmp("inc");
    let data_dir = dir.join("data_dir");
    std::fs::create_dir_all(&data_dir).unwrap();
    seed_real_data(&data_dir);

    let full = dir.join("full");
    create_full_backup(&full, "sql", &data_dir).expect("full backup");

    let inc = dir.join("inc");
    create_incremental_backup(&full, &inc, "sql", &data_dir).expect("incremental");

    let sql = exported_sql(&inc);
    assert!(
        sql.contains("widgets"),
        "#4938: the incremental path must read the caller's database too; got:\n{sql}"
    );
    assert!(
        sql.contains("alpha-one"),
        "#4938: the caller's rows must be present; got:\n{sql}"
    );
    assert!(
        !sql.contains("CREATE TABLE users"),
        "#4938: demo data must not leak in; got:\n{sql}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_source_is_an_error_not_a_demo_backup() {
    // The crucial half of removing the silent fallback: pointing the tool
    // at a directory that does not exist must fail loudly. It used to
    // succeed and produce a demo backup.
    let dir = tmp("missing");
    let backup = dir.join("full");

    let err = create_full_backup(&backup, "sql", &dir.join("nope"))
        .expect_err("a missing data_dir must not silently produce a backup");

    let msg = format!("{err:#}");
    assert!(
        msg.contains("nope") || msg.contains("does not exist"),
        "#4938: the error must name the unusable path; got: {msg}"
    );
    assert!(
        !backup.join("manifest.json").exists(),
        "#4938: no manifest may be written when the source is unreadable"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_empty_database_produces_no_backup() {
    // A real but empty source is different from a missing one: it should
    // succeed and say there is nothing to back up, not invent data.
    let dir = tmp("empty");
    let data_dir = dir.join("data_dir");
    std::fs::create_dir_all(&data_dir).unwrap();
    let backup = dir.join("full");

    create_full_backup(&backup, "sql", &data_dir).expect("an empty source is not an error");

    assert!(
        !backup.join("manifest.json").exists(),
        "#4938: an empty database must not produce a manifest claiming a backup"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
