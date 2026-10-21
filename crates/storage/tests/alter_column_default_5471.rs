//! #5471 — `ALTER TABLE ... ALTER COLUMN ... SET/DROP DEFAULT` must work
//! on the **on-disk** backends, and the default must survive a restart.
//!
//! ## The defect
//!
//! `StorageEngine::set_column_default`'s trait default answers
//! `"set_column_default not supported by this storage engine"`, and only
//! `MemoryStorage` overrode it. So the DDL was **impossible on every
//! on-disk backend** — including the production one.
//!
//! Worse, the production stack is a **stack** of wrappers:
//!
//! ```text
//! ExecutionEngine<BoxStorageEngine>
//!   → BoxStorageEngine
//!     → ParallelWalStorage
//!       → MvccStorage
//!         → FileStorage
//! ```
//!
//! A forwarding override is needed at **every** level: a single missing
//! link makes the trait default answer "not supported" again. That is
//! why this file drives the wrappers explicitly and not just the leaf.
//!
//! ## The second half: the default must reach the disk
//!
//! The default lives in the **schema** (`info.columns[i].default_value`),
//! not in the rows. The delta-append path only carries rows, so a
//! default written through it would silently vanish on restart. The
//! implementation therefore writes a **full snapshot**.
//!
//! ## Discriminating power
//!
//! Each case below goes red when its own link is removed:
//!
//! * drop the `FileStorage` impl → the whole file fails with
//!   "not supported";
//! * drop one wrapper's forward → the wrapper case fails with
//!   "not supported";
//! * persist through the delta path instead of a full snapshot → the
//!   restart case fails.

use sqlrustgo_storage::engine::{ColumnDefinition, IndexInfo, StorageEngine, TableInfo, Value};
use sqlrustgo_parser::IndexColumnSpec;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::mvcc_storage::MvccStorage;
use sqlrustgo_storage::wal::FileBackedWalManager;
use sqlrustgo_storage::{BoxStorageEngine, MemoryStorage, ParallelWalStorage, WalStorage};
use std::path::PathBuf;

fn table_info(name: &str) -> TableInfo {
    let mut id = ColumnDefinition::new("id", "INTEGER");
    id.primary_key = true;
    TableInfo {
        name: name.to_string(),
        columns: vec![id, ColumnDefinition::new("v", "INTEGER")],
        ..Default::default()
    }
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "alter_column_default_5471_{tag}_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// The default recorded on column `v` of table `t`.
fn default_of<S: StorageEngine>(s: &S, column: &str) -> Option<String> {
    let info = s.get_table_info("t").expect("table must exist");
    info.columns
        .iter()
        .find(|c| c.name == column)
        .expect("column must exist")
        .default_value
        .clone()
}

// ---------------------------------------------------------------------------
// The leaf backend
// ---------------------------------------------------------------------------

/// `FileStorage` must accept the DDL at all.
#[test]
fn file_storage_accepts_set_column_default() {
    let dir = scratch("leaf");
    let mut store = FileStorage::new(dir).expect("FileStorage::new");
    store.create_table(&table_info("t")).expect("create table");

    store
        .set_column_default("t", "v", Some("99".to_string()))
        .expect("SET DEFAULT must be supported by the file backend");
    assert_eq!(default_of(&store, "v"), Some("99".to_string()));
}

/// The default must survive a **restart** — the schema is what carries it.
#[test]
fn file_storage_persists_the_default_across_a_reopen() {
    let dir = scratch("persist");
    {
        let mut store = FileStorage::new(dir.clone()).expect("FileStorage::new");
        store.create_table(&table_info("t")).expect("create table");
        store
            .set_column_default("t", "v", Some("7".to_string()))
            .expect("SET DEFAULT");
    }

    let reopened = FileStorage::new(dir).expect("reopen");
    assert_eq!(
        default_of(&reopened, "v"),
        Some("7".to_string()),
        "the default must survive a restart — it lives in the schema, and \
         the delta path only carries rows"
    );
}

/// The default must survive a restart **when the table grew and an index
/// was built in between**.
///
/// This pins the persistence across a sequence that touches both the row
/// set and the schema:
///
/// 1. `CREATE TABLE t` + `INSERT` two rows + `flush()` ⇒ the base
///    snapshot `t.json` holds 2 rows;
/// 2. `INSERT` one more row;
/// 3. `CREATE INDEX` on `t(v)` ⇒ the index builder drains the write
///    buffer into `tables.rows` so it can assign row ids;
/// 4. `SET DEFAULT` ⇒ the schema changes;
/// 5. reopen ⇒ the default must still be there.
///
/// **What this test does NOT discriminate** (and why):
/// persisting through `save_table` (the row-count-driven decision)
/// instead of `save_table_full` produces **the same result** here, so
/// this case is **not** a red light for that choice. The reason is
/// structural: `save_table` only takes the delta path when
/// `rows.len() > last_saved_row_count`, and **every** path that grows
/// `tables.rows` (`insert_buffered`, `flush_buffer`) saves in the same
/// critical section — so that condition cannot hold at the moment a
/// schema change is written. `save_table_full` is used anyway because a
/// schema write should not *depend* on that invariant, which lives in a
/// different function's implementation details.
#[test]
fn file_storage_persists_the_default_when_the_table_grew_since_the_last_save() {
    let dir = scratch("grew");
    {
        let mut store = FileStorage::new(dir.clone()).expect("FileStorage::new");
        store.create_table(&table_info("t")).expect("create table");

        store
            .insert(
                "t",
                vec![
                    vec![Value::Integer(1), Value::Integer(10)],
                    vec![Value::Integer(2), Value::Integer(20)],
                ],
            )
            .expect("insert 2 rows");
        store.flush().expect("flush — writes the base snapshot");

        store
            .insert(
                "t",
                vec![vec![Value::Integer(3), Value::Integer(30)]],
            )
            .expect("insert 1 more row");

        // Draining the buffer without saving is what `create_index` does.
        // Fully-qualified: the inherent `FileStorage::create_index` takes
        // (table, column, index) and would shadow the trait method.
        StorageEngine::create_index(
            &mut store,
            IndexInfo {
                name: "idx_v".to_string(),
                table: "t".to_string(),
                columns: vec![IndexColumnSpec::column("v")],
                is_unique: false,
                original_sql: String::new(),
            },
        )
        .expect("create index");

        store
            .set_column_default("t", "v", Some("5".to_string()))
            .expect("SET DEFAULT");
    }

    let reopened = FileStorage::new(dir).expect("reopen");
    assert_eq!(
        default_of(&reopened, "v"),
        Some("5".to_string()),
        "the default must survive a restart even when the table grew \
         since its last save — a row-count-driven save decision takes \
         the delta path, which carries rows but not the schema"
    );
}

/// `DROP DEFAULT` must clear it, on disk too.
#[test]
fn file_storage_persists_drop_default_across_a_reopen() {
    let dir = scratch("drop");
    {
        let mut store = FileStorage::new(dir.clone()).expect("FileStorage::new");
        store.create_table(&table_info("t")).expect("create table");
        store
            .set_column_default("t", "v", Some("7".to_string()))
            .expect("SET DEFAULT");
        store
            .set_column_default("t", "v", None)
            .expect("DROP DEFAULT");
    }

    let reopened = FileStorage::new(dir).expect("reopen");
    assert_eq!(
        default_of(&reopened, "v"),
        None,
        "DROP DEFAULT must survive a restart as well"
    );
}

/// A missing table must still be reported as missing — the fix must not
/// turn "not found" into a silent success.
#[test]
fn file_storage_reports_a_missing_table() {
    let dir = scratch("missing-table");
    let mut store = FileStorage::new(dir).expect("FileStorage::new");
    store.create_table(&table_info("t")).expect("create table");

    let err = store
        .set_column_default("no_such_table", "v", Some("1".to_string()))
        .expect_err("a missing table must be an error");
    assert!(
        err.to_string().contains("Table not found"),
        "unexpected error: {}",
        err
    );
}

/// A missing column must still be reported as missing.
#[test]
fn file_storage_reports_a_missing_column() {
    let dir = scratch("missing-column");
    let mut store = FileStorage::new(dir).expect("FileStorage::new");
    store.create_table(&table_info("t")).expect("create table");

    let err = store
        .set_column_default("t", "no_such_col", Some("1".to_string()))
        .expect_err("a missing column must be an error");
    assert!(
        err.to_string().contains("Column not found"),
        "unexpected error: {}",
        err
    );
}

// ---------------------------------------------------------------------------
// The wrappers — a single missing link makes the trait default answer again
// ---------------------------------------------------------------------------

/// `WalStorage` must forward the DDL to the inner engine.
#[test]
fn wal_storage_forwards_set_column_default() {
    let dir = scratch("wal");
    let file = FileStorage::new(dir.clone()).expect("FileStorage::new");
    let wal = FileBackedWalManager::new(dir.join("sqlrustgo.wal")).expect("wal manager");
    let mut store = WalStorage::new(file, wal).expect("WalStorage::new");
    store.create_table(&table_info("t")).expect("create table");

    store
        .set_column_default("t", "v", Some("42".to_string()))
        .expect("WalStorage must forward SET DEFAULT to the inner engine");
    assert_eq!(default_of(&store, "v"), Some("42".to_string()));
}

/// `MvccStorage` must forward it too.
#[test]
fn mvcc_storage_forwards_set_column_default() {
    let dir = scratch("mvcc");
    let file = FileStorage::new(dir).expect("FileStorage::new");
    let mut store = MvccStorage::new(file);
    store.create_table(&table_info("t")).expect("create table");

    store
        .set_column_default("t", "v", Some("42".to_string()))
        .expect("MvccStorage must forward SET DEFAULT to the inner engine");
    assert_eq!(default_of(&store, "v"), Some("42".to_string()));
}

/// `BoxStorageEngine` — the type every server-created storage is
/// wrapped in — must forward it.
#[test]
fn box_storage_engine_forwards_set_column_default() {
    let dir = scratch("box");
    let file = FileStorage::new(dir).expect("FileStorage::new");
    let mut store = BoxStorageEngine::new(file);
    store.create_table(&table_info("t")).expect("create table");

    store
        .set_column_default("t", "v", Some("42".to_string()))
        .expect("BoxStorageEngine must forward SET DEFAULT to the inner engine");
    assert_eq!(default_of(&store, "v"), Some("42".to_string()));
}

/// The **production** stack shape: `ParallelWalStorage` over
/// `MvccStorage<FileStorage>`. Every link in the chain is exercised at
/// once, so this case goes red if any single forward is missing.
#[test]
fn the_production_storage_stack_accepts_set_column_default() {
    let dir = scratch("prod");
    let file = FileStorage::new(dir.clone()).expect("FileStorage::new");
    let wal = FileBackedWalManager::new(dir.join("sqlrustgo.wal")).expect("wal manager");
    let mut store = ParallelWalStorage::new(MvccStorage::new(file), wal);
    store.create_table(&table_info("t")).expect("create table");

    store
        .set_column_default("t", "v", Some("42".to_string()))
        .expect("the production storage stack must accept SET DEFAULT");
    assert_eq!(default_of(&store, "v"), Some("42".to_string()));
}

// ---------------------------------------------------------------------------
// The memory backend must not regress
// ---------------------------------------------------------------------------

/// `MemoryStorage` keeps working — the batch must not trade one backend
/// for another.
#[test]
fn memory_storage_still_accepts_set_column_default() {
    let mut store = MemoryStorage::new();
    store.create_table(&table_info("t")).expect("create table");

    store
        .set_column_default("t", "v", Some("99".to_string()))
        .expect("MemoryStorage must keep accepting SET DEFAULT");
    assert_eq!(default_of(&store, "v"), Some("99".to_string()));
}
