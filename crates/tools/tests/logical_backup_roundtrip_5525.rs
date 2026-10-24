//! #5525 — the logical backup/restore round trip, end to end.
//!
//! The issue reported that `sqlrustgo backup` reported success while
//! backing up **zero** tables, and that `sqlrustgo restore` reported a table
//! count while restoring **nothing**. These tests drive the real code path
//! against a real on-disk storage:
//!
//! * the backup must enumerate the tables that are actually there,
//! * the dump must carry the schema (type, `NOT NULL`, primary key) and must
//!   keep **empty** tables,
//! * the restore must write the rows **back** and verify them,
//! * and the negative controls — an empty database, a corrupted payload —
//!   must **fail** instead of reporting success.

use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, Value};
use sqlrustgo_tools::backup_restore::{
    collect_all_tables, BackupColumn, BackupManager, BackupTable,
};
use tempfile::TempDir;

/// A table with a typed, constrained schema and one row.
fn users_table() -> BackupTable {
    let mut table = BackupTable::new(
        "users",
        vec![
            BackupColumn::new("id", "INTEGER").primary_key(),
            BackupColumn::new("name", "TEXT").not_null(),
            BackupColumn::new("email", "TEXT").with_max_length(255),
        ],
    );
    table.rows = vec![
        vec![
            Value::Integer(1),
            Value::Text("alice".to_string()),
            Value::Text("alice@example.com".to_string()),
        ],
        vec![
            Value::Integer(2),
            Value::Text("bob".to_string()),
            Value::Null,
        ],
    ];
    table
}

/// A table with no rows — it must survive the backup.
fn empty_table() -> BackupTable {
    BackupTable::new(
        "audit_log",
        vec![
            BackupColumn::new("id", "INTEGER").primary_key(),
            BackupColumn::new("note", "TEXT"),
        ],
    )
}

/// Create the source database in `dir`.
fn seed_source(dir: &TempDir) {
    let mut storage = FileStorage::new(dir.path().to_path_buf()).unwrap();
    for table in [users_table(), empty_table()] {
        storage
            .create_table_in_db("default", &table.to_table_info())
            .unwrap();
    }
    storage
        .insert_in_db(
            "default",
            "users",
            vec![
                vec![
                    Value::Integer(1),
                    Value::Text("alice".to_string()),
                    Value::Text("alice@example.com".to_string()),
                ],
                vec![
                    Value::Integer(2),
                    Value::Text("bob".to_string()),
                    Value::Null,
                ],
            ],
        )
        .unwrap();
    storage.flush().unwrap();
}

/// The backup enumerates the tables that are really there.
#[test]
fn test_backup_enumerates_the_real_tables() {
    let src = TempDir::new().unwrap();
    seed_source(&src);

    let storage = FileStorage::new(src.path().to_path_buf()).unwrap();
    let tables = collect_all_tables(&storage, "default").unwrap();

    let mut names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["audit_log", "users"]);

    let users = tables.iter().find(|t| t.name == "users").unwrap();
    assert_eq!(users.row_count(), 2);
    assert!(users.columns[0].primary_key);
    assert!(users.columns[1].not_null);
    assert_eq!(users.columns[2].char_max_length, Some(255));
}

/// The dump carries the schema, keeps the empty table, and escapes the values.
#[test]
fn test_dump_carries_schema_and_empty_tables() {
    let src = TempDir::new().unwrap();
    seed_source(&src);

    let backup_dir = TempDir::new().unwrap();
    let mgr = BackupManager::new(backup_dir.path().to_path_buf());

    let storage = FileStorage::new(src.path().to_path_buf()).unwrap();
    let tables = collect_all_tables(&storage, "default").unwrap();
    let meta = mgr.create_backup("default", &tables).unwrap();

    let sql = std::fs::read_to_string(backup_dir.path().join(format!("{}.sql", meta.id))).unwrap();

    // The schema is there — type, NOT NULL, primary key, width.
    assert!(sql.contains("`id` INTEGER NOT NULL"), "{sql}");
    assert!(sql.contains("`name` TEXT NOT NULL"), "{sql}");
    assert!(sql.contains("`email` TEXT(255)"), "{sql}");
    assert!(sql.contains("PRIMARY KEY (`id`)"), "{sql}");

    // The empty table is still in the dump.
    assert!(sql.contains("CREATE TABLE `audit_log`"), "{sql}");
    assert!(
        !sql.contains("INSERT INTO `audit_log`"),
        "an empty table must not carry rows: {sql}"
    );

    // The rows are there, with the right values.
    assert!(sql.contains("INSERT INTO `users`"), "{sql}");
    assert!(sql.contains("'alice@example.com'"), "{sql}");
    assert!(sql.contains("NULL"), "{sql}");
}

/// The restore writes the rows back into another database.
#[test]
fn test_restore_round_trip_lands_the_rows() {
    let src = TempDir::new().unwrap();
    seed_source(&src);

    let backup_dir = TempDir::new().unwrap();
    let mgr = BackupManager::new(backup_dir.path().to_path_buf());

    let storage = FileStorage::new(src.path().to_path_buf()).unwrap();
    let tables = collect_all_tables(&storage, "default").unwrap();
    let meta = mgr.create_backup("default", &tables).unwrap();

    // Restore into a *different*, empty data directory.
    let dst = TempDir::new().unwrap();
    let mut target = FileStorage::new(dst.path().to_path_buf()).unwrap();
    let result = mgr
        .restore_into(&meta.id, &mut target, "default", false)
        .unwrap();

    assert_eq!(result.tables_restored, 2);
    assert_eq!(result.rows_restored, 2);
    target.flush().unwrap();
    drop(target);

    // Re-open the target: the restore has to be durable, not just visible
    // in the buffer of the process that wrote it.
    let reopened = FileStorage::new(dst.path().to_path_buf()).unwrap();
    let rows = reopened.scan_in_db("default", "users").unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][1], Value::Text("alice".to_string()));
    assert_eq!(rows[1][2], Value::Null);

    // The schema came back too, not just the rows.
    let info = reopened.get_table_info_in("default", "users").unwrap();
    assert_eq!(info.columns.len(), 3);
    assert_eq!(info.columns[0].data_type, "INTEGER");
    assert!(info.columns[0].primary_key);
    assert!(!info.columns[1].nullable);
    assert_eq!(info.columns[2].char_max_length, Some(255));

    // The empty table came back as an empty table.
    assert!(reopened.has_table_in("default", "audit_log"));
    assert!(reopened
        .scan_in_db("default", "audit_log")
        .unwrap()
        .is_empty());
}

/// Negative control: a database with no tables must not produce a "successful"
/// backup. This is the exact shape of the bug #5525 reported.
#[test]
fn test_backup_of_an_empty_database_fails() {
    let src = TempDir::new().unwrap();
    let storage = FileStorage::new(src.path().to_path_buf()).unwrap();

    let err = collect_all_tables(&storage, "default").unwrap_err();
    assert!(err.contains("no tables"), "unexpected error: {err}");

    let backup_dir = TempDir::new().unwrap();
    let mgr = BackupManager::new(backup_dir.path().to_path_buf());
    let err = mgr.create_backup("default", &[]).unwrap_err();
    assert!(err.contains("no tables"), "unexpected error: {err}");
}

/// Negative control: a corrupted payload must be refused, not restored.
#[test]
fn test_restore_of_a_corrupted_payload_fails() {
    let src = TempDir::new().unwrap();
    seed_source(&src);

    let backup_dir = TempDir::new().unwrap();
    let mgr = BackupManager::new(backup_dir.path().to_path_buf());

    let storage = FileStorage::new(src.path().to_path_buf()).unwrap();
    let tables = collect_all_tables(&storage, "default").unwrap();
    let meta = mgr.create_backup("default", &tables).unwrap();

    let payload = backup_dir.path().join(format!("{}.data.json", meta.id));
    let raw = std::fs::read_to_string(&payload).unwrap();
    std::fs::write(&payload, raw.replace("alice", "mallory")).unwrap();

    let dst = TempDir::new().unwrap();
    let mut target = FileStorage::new(dst.path().to_path_buf()).unwrap();
    let err = mgr
        .restore_into(&meta.id, &mut target, "default", false)
        .unwrap_err();
    assert!(err.contains("corrupt"), "unexpected error: {err}");

    // Nothing may have been written to the target.
    assert!(!target.has_table_in("default", "users"));
}

/// Negative control: restoring onto an existing table must not silently
/// overwrite it.
#[test]
fn test_restore_does_not_overwrite_without_drop_first() {
    let src = TempDir::new().unwrap();
    seed_source(&src);

    let backup_dir = TempDir::new().unwrap();
    let mgr = BackupManager::new(backup_dir.path().to_path_buf());

    let storage = FileStorage::new(src.path().to_path_buf()).unwrap();
    let tables = collect_all_tables(&storage, "default").unwrap();
    let meta = mgr.create_backup("default", &tables).unwrap();

    let dst = TempDir::new().unwrap();
    let mut target = FileStorage::new(dst.path().to_path_buf()).unwrap();

    mgr.restore_into(&meta.id, &mut target, "default", false)
        .unwrap();
    let err = mgr
        .restore_into(&meta.id, &mut target, "default", false)
        .unwrap_err();
    assert!(err.contains("already exists"), "unexpected error: {err}");

    // With --drop-first the restore replaces the table.
    let result = mgr
        .restore_into(&meta.id, &mut target, "default", true)
        .unwrap();
    assert_eq!(result.rows_restored, 2);
}

/// A quote in a value must not break the dump.
#[test]
fn test_quotes_and_unicode_survive_the_round_trip() {
    let src = TempDir::new().unwrap();
    let mut storage = FileStorage::new(src.path().to_path_buf()).unwrap();

    let mut table = BackupTable::new(
        "quotes",
        vec![
            BackupColumn::new("id", "INTEGER").primary_key(),
            BackupColumn::new("text", "TEXT"),
        ],
    );
    table.rows = vec![
        vec![Value::Integer(1), Value::Text("O'Brien".to_string())],
        vec![
            Value::Integer(2),
            Value::Text("日本語 — ünïcode".to_string()),
        ],
        vec![Value::Integer(3), Value::Text("back\\slash".to_string())],
    ];
    storage
        .create_table_in_db("default", &table.to_table_info())
        .unwrap();
    storage
        .insert_in_db("default", "quotes", table.rows.clone())
        .unwrap();
    storage.flush().unwrap();
    drop(storage);

    let backup_dir = TempDir::new().unwrap();
    let mgr = BackupManager::new(backup_dir.path().to_path_buf());
    let storage = FileStorage::new(src.path().to_path_buf()).unwrap();
    let tables = collect_all_tables(&storage, "default").unwrap();
    let meta = mgr.create_backup("default", &tables).unwrap();

    let sql = std::fs::read_to_string(backup_dir.path().join(format!("{}.sql", meta.id))).unwrap();
    assert!(sql.contains("'O''Brien'"), "{sql}");

    let dst = TempDir::new().unwrap();
    let mut target = FileStorage::new(dst.path().to_path_buf()).unwrap();
    mgr.restore_into(&meta.id, &mut target, "default", false)
        .unwrap();
    target.flush().unwrap();
    drop(target);

    let reopened = FileStorage::new(dst.path().to_path_buf()).unwrap();
    let rows = reopened.scan_in_db("default", "quotes").unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0][1], Value::Text("O'Brien".to_string()));
    assert_eq!(rows[1][1], Value::Text("日本語 — ünïcode".to_string()));
    assert_eq!(rows[2][1], Value::Text("back\\slash".to_string()));
}

/// The CLI entry point must fail when there is nothing to back up, and must
/// succeed — with the tables — when there is.
#[test]
fn test_cli_backup_and_restore() {
    use sqlrustgo_tools::backup_restore::{run_backup, run_restore, BackupCommand, RestoreCommand};

    let src = TempDir::new().unwrap();
    seed_source(&src);

    let backup_dir = TempDir::new().unwrap();
    run_backup(BackupCommand {
        database: "default".to_string(),
        output_dir: backup_dir.path().to_string_lossy().to_string(),
        backup_type: "full".to_string(),
        data_dir: src.path().to_string_lossy().to_string(),
        schema_only: false,
        compress: false,
    })
    .expect("backup must succeed");

    let backup_id = std::fs::read_dir(backup_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .find(|n| n.ends_with(".meta.json"))
        .map(|n| n.trim_end_matches(".meta.json").to_string())
        .expect("a backup must have been written");

    let dst = TempDir::new().unwrap();
    run_restore(RestoreCommand {
        database: "default".to_string(),
        backup_id: backup_id.clone(),
        backup_dir: backup_dir.path().to_string_lossy().to_string(),
        data_dir: dst.path().to_string_lossy().to_string(),
        drop_first: false,
    })
    .expect("restore must succeed");

    let reopened = FileStorage::new(dst.path().to_path_buf()).unwrap();
    assert_eq!(reopened.scan_in_db("default", "users").unwrap().len(), 2);
}

/// The switches that are not implemented must be rejected, not ignored.
#[test]
fn test_cli_rejects_the_unimplemented_switches() {
    use sqlrustgo_tools::backup_restore::{run_backup, BackupCommand};

    let src = TempDir::new().unwrap();
    seed_source(&src);
    let backup_dir = TempDir::new().unwrap();

    let err = run_backup(BackupCommand {
        database: "default".to_string(),
        output_dir: backup_dir.path().to_string_lossy().to_string(),
        backup_type: "incremental".to_string(),
        data_dir: src.path().to_string_lossy().to_string(),
        schema_only: false,
        compress: false,
    })
    .unwrap_err();
    assert!(err.to_string().contains("not implemented"), "{err}");

    let err = run_backup(BackupCommand {
        database: "default".to_string(),
        output_dir: backup_dir.path().to_string_lossy().to_string(),
        backup_type: "full".to_string(),
        data_dir: src.path().to_string_lossy().to_string(),
        schema_only: false,
        compress: true,
    })
    .unwrap_err();
    assert!(err.to_string().contains("not implemented"), "{err}");
}

/// `schema_only` used to be accepted and ignored — it must now really drop
/// the rows.
#[test]
fn test_cli_schema_only_drops_the_rows() {
    use sqlrustgo_tools::backup_restore::{run_backup, BackupCommand};

    let src = TempDir::new().unwrap();
    seed_source(&src);
    let backup_dir = TempDir::new().unwrap();

    run_backup(BackupCommand {
        database: "default".to_string(),
        output_dir: backup_dir.path().to_string_lossy().to_string(),
        backup_type: "full".to_string(),
        data_dir: src.path().to_string_lossy().to_string(),
        schema_only: true,
        compress: false,
    })
    .expect("backup must succeed");

    let sql = std::fs::read_dir(backup_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|e| e == "sql").unwrap_or(false))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .expect("a dump must have been written");

    assert!(sql.contains("CREATE TABLE `users`"), "{sql}");
    assert!(!sql.contains("INSERT INTO"), "{sql}");
}

/// The storage's own column metadata has to survive the trip — the backup
/// takes the type from the catalog, not from the data.
#[test]
fn test_backup_uses_the_catalog_type_not_the_data() {
    let src = TempDir::new().unwrap();
    let mut storage = FileStorage::new(src.path().to_path_buf()).unwrap();

    // A TEXT column holding a number: the dump must still say TEXT.
    let info = sqlrustgo_storage::TableInfo {
        name: "mixed".to_string(),
        columns: vec![ColumnDefinition {
            name: "v".to_string(),
            data_type: "TEXT".to_string(),
            nullable: true,
            ..Default::default()
        }],
        ..Default::default()
    };
    storage.create_table_in_db("default", &info).unwrap();
    storage
        .insert_in_db("default", "mixed", vec![vec![Value::Integer(42)]])
        .unwrap();

    let tables = collect_all_tables(&storage, "default").unwrap();
    assert_eq!(tables[0].columns[0].data_type, "TEXT");
}
