//! #5509 — the column-level DDL must match a column name
//! **case-insensitively**, on **every** backend.
//!
//! ## The defect
//!
//! `FileStorage` matched the column of a column-level DDL **case-exactly**
//! (`c.name == column`), while `MemoryStorage` matched it case-insensitively
//! (`c.name.to_lowercase() == column.to_lowercase()`). The *same* SQL
//! therefore succeeded on the memory backend and failed on the on-disk one:
//!
//! | DDL (column stored as `B`) | memory backend | file backend (before) |
//! |---|---|---|
//! | `ALTER TABLE t DROP COLUMN b` | hit | **"Column not found: b"** |
//! | `ALTER TABLE t ALTER COLUMN b SET DEFAULT 1` | hit | **"Column not found: b"** |
//! | `ALTER TABLE t ALTER COLUMN b SET DATA TYPE VARCHAR` | hit | **"Column not found: b"** |
//! | `ALTER TABLE t RENAME COLUMN b TO c` | hit | **"Column not found: b"** |
//!
//! The stored name was not a stable function of what the user wrote either:
//! `create_table` keeps the name **as given**, but `modify_column` /
//! `rename_column` **lower-case** it — so `DROP COLUMN B` used to work or
//! fail depending on whether the column had ever been renamed. That is not a
//! consistent strictness, it is an accident of the column's history.
//!
//! ## The fix
//!
//! All **eight** column-level DDL match sites (four per backend) plus the
//! ninth copy in the SQL layer (`src/engine_ddl.rs`, the `SET DATA TYPE`
//! path) now go through the storage crate's **single** column-name
//! resolver, `sqlrustgo_storage::find_column_index_in` — the same one the
//! SQL layer already uses to resolve `SELECT` targets. It prefers an exact
//! match and falls back to a case-insensitive one, so the widening keeps
//! the previous behaviour for the exact spelling.
//!
//! `FileStorage::modify_column` additionally lower-cases the **stored**
//! name, which `MemoryStorage` already did — otherwise the two backends
//! produced *different schemas* for the same DDL history and `SHOW COLUMNS`
//! disagreed between them.
//!
//! ## Discriminating power
//!
//! Revert any one of the eight match sites to `c.name == column` and the
//! matching test below goes red with "Column not found". Revert the
//! `to_lowercase()` in `modify_column` and
//! `the_stored_name_does_not_depend_on_the_backend` goes red.

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo};
use sqlrustgo_storage::{FileStorage, MemoryStorage};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// A one-column table. The caller picks the column's **case**: that is the
/// whole point of the test.
fn table_info(name: &str, col: &str) -> TableInfo {
    let mut c = ColumnDefinition::new(col, "INTEGER");
    c.nullable = true;
    TableInfo {
        name: name.to_string(),
        columns: vec![c],
        ..Default::default()
    }
}

/// The stored column names of table `t`, in order.
fn stored_columns<S: StorageEngine>(s: &S) -> Vec<String> {
    s.get_table_info("t")
        .expect("table must exist")
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect()
}

fn file_storage() -> (tempfile::TempDir, FileStorage) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let s = FileStorage::new(tmp.path().to_path_buf()).expect("FileStorage::new");
    (tmp, s)
}

/// Run `DROP COLUMN` on a freshly created table and report the outcome as
/// a plain `Result`, so the two backends can be compared **by their
/// behaviour** rather than by re-implementing the check.
fn drop_column_outcome<S: StorageEngine>(
    s: &mut S,
    create_col: &str,
    ddl_col: &str,
) -> Result<(), String> {
    s.create_table(&table_info("t", create_col))
        .map_err(|e| e.to_string())?;
    s.drop_column("t", ddl_col).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// the file backend must match case-insensitively
// ---------------------------------------------------------------------------

/// The column is stored as `B`; the DDL says `b`.
#[test]
fn drop_column_finds_the_column_case_insensitively() {
    let (_tmp, mut s) = file_storage();
    s.create_table(&table_info("t", "B"))
        .expect("CREATE TABLE must succeed");
    s.drop_column("t", "b")
        .expect("DROP COLUMN must find the column regardless of case");
    assert!(
        stored_columns(&s).is_empty(),
        "the column must actually be gone"
    );
}

/// Same, for `SET / DROP DEFAULT` (#5471 added it to the file backend).
#[test]
fn set_column_default_finds_the_column_case_insensitively() {
    let (_tmp, mut s) = file_storage();
    s.create_table(&table_info("t", "B"))
        .expect("CREATE TABLE must succeed");
    s.set_column_default("t", "b", Some("99".to_string()))
        .expect("SET DEFAULT must find the column regardless of case");
    let info = s.get_table_info("t").expect("table must exist");
    assert_eq!(
        info.columns[0].default_value,
        Some("99".to_string()),
        "the default must land on the stored column"
    );
}

/// `DROP DEFAULT` must be just as case-insensitive.
#[test]
fn drop_column_default_finds_the_column_case_insensitively() {
    let (_tmp, mut s) = file_storage();
    s.create_table(&table_info("t", "B"))
        .expect("CREATE TABLE must succeed");
    s.set_column_default("t", "b", Some("99".to_string()))
        .expect("SET DEFAULT");
    s.set_column_default("t", "B", None)
        .expect("DROP DEFAULT must find the column regardless of case");
    let info = s.get_table_info("t").expect("table must exist");
    assert_eq!(info.columns[0].default_value, None);
}

/// `MODIFY COLUMN` — the storage-level spelling of `ALTER COLUMN ...
/// SET DATA TYPE`.
#[test]
fn modify_column_finds_the_column_case_insensitively() {
    let (_tmp, mut s) = file_storage();
    s.create_table(&table_info("t", "B"))
        .expect("CREATE TABLE must succeed");
    let mut new_def = ColumnDefinition::new("B", "VARCHAR");
    new_def.nullable = true;
    s.modify_column("t", "b", new_def)
        .expect("MODIFY COLUMN must find the column regardless of case");
    let info = s.get_table_info("t").expect("table must exist");
    assert_eq!(
        info.columns[0].data_type, "VARCHAR",
        "the new type must land on the stored column"
    );
}

/// `RENAME COLUMN` — and the new name must be found by *its* spelling too.
#[test]
fn rename_column_finds_the_column_case_insensitively() {
    let (_tmp, mut s) = file_storage();
    s.create_table(&table_info("t", "B"))
        .expect("CREATE TABLE must succeed");
    s.rename_column("t", "b", "C")
        .expect("RENAME COLUMN must find the column regardless of case");
    assert_eq!(
        stored_columns(&s),
        vec!["c".to_string()],
        "the renamed column must be stored under its new name"
    );
}

/// The widening must **not** turn "not found" into a silent success.
#[test]
fn a_missing_column_still_errors_on_both_backends() {
    let (_tmp, mut file) = file_storage();
    let mut mem = MemoryStorage::new();

    file.create_table(&table_info("t", "B"))
        .expect("CREATE TABLE must succeed");
    mem.create_table(&table_info("t", "B"))
        .expect("CREATE TABLE must succeed");

    let file_err = file
        .drop_column("t", "no_such_col")
        .expect_err("a missing column must be an error")
        .to_string();
    let mem_err = mem
        .drop_column("t", "no_such_col")
        .expect_err("a missing column must be an error")
        .to_string();

    assert!(
        file_err.contains("Column not found"),
        "file backend: unexpected error: {}",
        file_err
    );
    assert!(
        mem_err.contains("Column not found"),
        "memory backend: unexpected error: {}",
        mem_err
    );
}

/// The exact-case spelling must keep working — the fix is a widening, not
/// a replacement.
#[test]
fn the_exact_case_spelling_still_works() {
    let (_tmp, mut s) = file_storage();
    s.create_table(&table_info("t", "B"))
        .expect("CREATE TABLE must succeed");
    s.drop_column("t", "B")
        .expect("DROP COLUMN with the exact spelling");
}

// ---------------------------------------------------------------------------
// the two backends must not diverge
// ---------------------------------------------------------------------------

/// The core of the issue: **the same SQL must have the same outcome on
/// both backends**, whatever case the user wrote.
#[test]
fn both_backends_agree_on_the_ddl_outcome() {
    let (_tmp, mut file) = file_storage();
    let mut mem = MemoryStorage::new();

    for (create_col, ddl_col) in [("B", "b"), ("b", "B"), ("B", "B")] {
        let m = drop_column_outcome(&mut mem, create_col, ddl_col);
        let f = drop_column_outcome(&mut file, create_col, ddl_col);
        assert_eq!(
            m.is_ok(),
            f.is_ok(),
            "backends disagree for CREATE({}) / DROP({}): memory={:?} file={:?}",
            create_col,
            ddl_col,
            m,
            f
        );
        assert!(
            f.is_ok(),
            "the file backend must match case-insensitively (CREATE {} / DROP {})",
            create_col,
            ddl_col
        );
    }
}

/// The **stored schema** must not depend on the backend either: the same
/// DDL history must produce the same column names everywhere.
#[test]
fn the_stored_name_does_not_depend_on_the_backend() {
    fn modify_and_read_back<S: StorageEngine>(s: &mut S) -> Vec<String> {
        s.create_table(&table_info("t", "B"))
            .expect("CREATE TABLE must succeed");
        let mut new_def = ColumnDefinition::new("B", "VARCHAR");
        new_def.nullable = true;
        s.modify_column("t", "b", new_def)
            .expect("MODIFY COLUMN must succeed");
        stored_columns(s)
    }

    let (_tmp, mut file) = file_storage();
    let mut mem = MemoryStorage::new();

    assert_eq!(
        modify_and_read_back(&mut mem),
        modify_and_read_back(&mut file),
        "the two backends must store the same column names for the same DDL"
    );
}

/// The DDL **history** must not decide whether a column is found either:
/// a column created as `B`, a column created as `b` and a column that has
/// been renamed must all be reachable by either spelling.
#[test]
fn the_ddl_history_does_not_change_whether_a_column_is_found() {
    let (_tmp, mut s) = file_storage();

    // 1. created with an upper-case name.
    s.create_table(&table_info("t1", "B"))
        .expect("CREATE TABLE must succeed");
    s.drop_column("t1", "b")
        .expect("a column created as `B` must be droppable as `b`");

    // 2. created with a lower-case name.
    s.create_table(&table_info("t2", "b"))
        .expect("CREATE TABLE must succeed");
    s.drop_column("t2", "B")
        .expect("a column created as `b` must be droppable as `B`");

    // 3. renamed — the stored name is lower-cased by `rename_column`, so
    //    this is the case the issue called out ("改过一次名字之后就命中不了").
    s.create_table(&table_info("t3", "X"))
        .expect("CREATE TABLE must succeed");
    s.rename_column("t3", "x", "Y")
        .expect("RENAME COLUMN must succeed");
    s.drop_column("t3", "Y")
        .expect("a renamed column must be droppable by its new name");
}

// ---------------------------------------------------------------------------
// end to end, through the SQL layer
// ---------------------------------------------------------------------------

/// The same scenario through the SQL layer, on the **on-disk** backend.
///
/// This mirrors the DuckDB conformance fixture
/// `crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/case_insensitive_alter.test`,
/// which the sqllogictest runner only ever executes against
/// `MemoryStorage` — so the on-disk behaviour was untested by construction.
#[test]
fn the_sql_layer_matches_the_column_case_insensitively_on_disk() {
    use parking_lot::RwLock;
    use sqlrustgo::ExecutionEngine;
    use std::sync::Arc;

    let tmp = tempfile::tempdir().expect("tempdir");
    let storage = FileStorage::new_with_buffer_config(tmp.path().to_path_buf(), 100, false)
        .expect("FileStorage::new");
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(storage)));

    // The column is declared in mixed case, the DDL refers to it in a
    // different case — every step below must hit.
    engine
        .execute(r#"CREATE TABLE "MyTable"(i integer, "BigColumn" integer)"#)
        .expect("CREATE TABLE must succeed");

    engine
        .execute("ALTER TABLE MyTable ALTER BIGCOLUMN SET DATA TYPE VARCHAR")
        .expect("SET DATA TYPE must find the column regardless of case");

    engine
        .execute("ALTER TABLE MyTable ALTER COLUMN bigcolumn SET DEFAULT 3")
        .expect("SET DEFAULT must find the column regardless of case");

    engine
        .execute("INSERT INTO MyTable(BIGCOLUMN) VALUES (DEFAULT)")
        .expect("INSERT with the DEFAULT token must succeed");

    let r = engine
        .execute("SELECT BIGCOLUMN FROM MyTable")
        .expect("SELECT must succeed");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(
        r.rows[0][0].to_string(),
        "3",
        "the default must have been materialised into the column"
    );

    engine
        .execute("ALTER TABLE MyTable ALTER COLUMN BIGCOLUMN DROP DEFAULT")
        .expect("DROP DEFAULT must find the column regardless of case");

    engine
        .execute("ALTER TABLE MyTable RENAME BIGCOLUMN TO \"SmallColumn\"")
        .expect("RENAME must find the column regardless of case");

    let r = engine
        .execute("SELECT SmallColumn FROM MyTable")
        .expect("SELECT after RENAME must succeed");
    assert_eq!(r.rows.len(), 1);

    engine
        .execute("ALTER TABLE MyTable DROP COLUMN smallcolumn")
        .expect("DROP COLUMN must find the column regardless of case");
}
