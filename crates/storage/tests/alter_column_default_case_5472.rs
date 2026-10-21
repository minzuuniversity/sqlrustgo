//! #5472 — `MemoryStorage::set_column_default` must match the column
//! name **case-insensitively**, like its three siblings.
//!
//! ## The defect
//!
//! | method | matching |
//! |---|---|
//! | `drop_column` | `c.name.to_lowercase() == column.to_lowercase()` |
//! | `modify_column` | `c.name.to_lowercase() == column.to_lowercase()` |
//! | `rename_column` | `c.name.to_lowercase() == old_name.to_lowercase()` |
//! | **`set_column_default`** | **`c.name == column`** ← the odd one out |
//!
//! The stored name is not a stable function of what the user wrote:
//! `create_table` keeps the column name as given, but `modify_column`
//! and `rename_column` **lowercase** it. So after a single ALTER the
//! column is stored as `b` while the DDL still says `B` — and
//! SET/DROP DEFAULT was the only column-level DDL that could no longer
//! find it.
//!
//! ## Discriminating power
//!
//! Revert the match to `c.name == column` and
//! `set_default_finds_the_column_case_insensitively` goes red with
//! "Column not found: B".

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo};
use sqlrustgo_storage::MemoryStorage;

fn table_info(name: &str) -> TableInfo {
    let mut id = ColumnDefinition::new("id", "INTEGER");
    id.primary_key = true;
    TableInfo {
        name: name.to_string(),
        columns: vec![id, ColumnDefinition::new("b", "INTEGER")],
        ..Default::default()
    }
}

fn seeded() -> MemoryStorage {
    let mut s = MemoryStorage::new();
    s.create_table(&table_info("t"))
        .expect("CREATE TABLE must succeed");
    s
}

fn default_of(s: &MemoryStorage, column: &str) -> Option<String> {
    let info = s.get_table_info("t").expect("table must exist");
    info.columns
        .iter()
        .find(|c| c.name == column)
        .expect("column must exist")
        .default_value
        .clone()
}

/// The column is stored as `b`; the DDL says `B`.
#[test]
fn set_default_finds_the_column_case_insensitively() {
    let mut s = seeded();
    s.set_column_default("t", "B", Some("99".to_string()))
        .expect("SET DEFAULT must find the column regardless of case");
    assert_eq!(
        default_of(&s, "b"),
        Some("99".to_string()),
        "the default must land on the stored column `b`"
    );
}

/// `DROP DEFAULT` must be just as case-insensitive.
#[test]
fn drop_default_finds_the_column_case_insensitively() {
    let mut s = seeded();
    s.set_column_default("t", "b", Some("99".to_string()))
        .expect("SET DEFAULT");
    s.set_column_default("t", "B", None)
        .expect("DROP DEFAULT must find the column regardless of case");
    assert_eq!(default_of(&s, "b"), None);
}

/// A column that does not exist must still be reported as missing — the
/// fix must not turn "not found" into a silent success.
#[test]
fn a_missing_column_still_errors() {
    let mut s = seeded();
    let err = s
        .set_column_default("t", "no_such_col", Some("1".to_string()))
        .expect_err("a missing column must be an error");
    assert!(
        err.to_string().contains("Column not found"),
        "unexpected error: {}",
        err
    );
}

/// The exact-case spelling must keep working — the fix is a widening,
/// not a replacement.
#[test]
fn the_exact_case_spelling_still_works() {
    let mut s = seeded();
    s.set_column_default("t", "b", Some("5".to_string()))
        .expect("SET DEFAULT with the exact spelling");
    assert_eq!(default_of(&s, "b"), Some("5".to_string()));
}
