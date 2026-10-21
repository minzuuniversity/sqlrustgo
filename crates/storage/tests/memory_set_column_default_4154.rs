//! `MemoryStorage::set_column_default` must find the table it is asked to
//! modify.
//!
//! **The defect**: `MemoryStorage::create_table` stores a table under the
//! *database-scoped* key `db\u{1}table` (see `scoped_key` / `tbl`), but
//! `set_column_default` looked the table up with the **bare lower-cased
//! table name**. The two can never be equal, so the lookup always missed
//! and every
//!
//! ```sql
//! ALTER TABLE t ALTER COLUMN c SET DEFAULT 99
//! ALTER TABLE t ALTER COLUMN c DROP DEFAULT
//! ```
//!
//! failed with `ExecutionError("Table not found: t")` — the feature was
//! completely non-functional on `MemoryStorage`.
//!
//! Its three sibling methods (`drop_column`, `modify_column`,
//! `rename_column`) already resolve through `self.tbl(table)`; this test
//! pins `set_column_default` to the same key scheme.
//!
//! Surfaced by the D6 integration gate, which for the first time actually
//! executed its curated test list (see
//! `docs/releases/v4.1.0/evidence/GATE_HONESTY_5417_5419_2026-10-21.md`).

use sqlrustgo_storage::engine::{StorageEngine, TableInfo};
use sqlrustgo_storage::{ColumnDefinition, MemoryStorage};

fn table(name: &str, columns: &[(&str, &str)]) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: columns
            .iter()
            .map(|(c, t)| ColumnDefinition::new(c, t))
            .collect(),
        ..TableInfo::default()
    }
}

fn seeded() -> MemoryStorage {
    let mut s = MemoryStorage::new();
    s.create_table(&table("t", &[("a", "INTEGER"), ("b", "INTEGER")]))
        .expect("CREATE TABLE must succeed");
    s
}

fn default_of(s: &MemoryStorage, table_name: &str, column: &str) -> Option<String> {
    let info = s
        .get_table_info(table_name)
        .expect("table must exist after SET DEFAULT");
    info.columns
        .iter()
        .find(|c| c.name == column)
        .expect("column must exist")
        .default_value
        .clone()
}

/// The table must be found — the pre-fix code always returned
/// "Table not found: t" here.
#[test]
fn set_column_default_finds_the_table_it_modifies() {
    let mut s = seeded();
    s.set_column_default("t", "b", Some("99".to_string()))
        .expect("SET DEFAULT must find the table it modifies");
    assert_eq!(default_of(&s, "t", "b"), Some("99".to_string()));
}

/// The same path with `None` is `DROP DEFAULT`.
#[test]
fn drop_column_default_finds_the_table_it_modifies() {
    let mut s = seeded();
    s.set_column_default("t", "b", Some("99".to_string()))
        .expect("SET DEFAULT must succeed");
    s.set_column_default("t", "b", None)
        .expect("DROP DEFAULT must find the table it modifies");
    assert_eq!(default_of(&s, "t", "b"), None);
}

/// The key scheme must be the same one `get_table_info` uses — otherwise the
/// write lands somewhere the read cannot see (or vice versa).
#[test]
fn set_column_default_uses_the_same_key_scheme_as_get_table_info() {
    let mut s = seeded();
    assert!(
        s.get_table_info("t").is_ok(),
        "the table must be readable before the default is set"
    );
    s.set_column_default("t", "a", Some("7".to_string()))
        .expect("SET DEFAULT must succeed");
    assert_eq!(
        default_of(&s, "t", "a"),
        Some("7".to_string()),
        "a default written by set_column_default must be visible to get_table_info"
    );
}

/// A genuinely missing table must still be reported as missing — the fix
/// must not turn "not found" into a silent success.
#[test]
fn set_column_default_on_a_missing_table_still_errors() {
    let mut s = seeded();
    let err = s
        .set_column_default("no_such_table", "a", Some("1".to_string()))
        .expect_err("a missing table must be an error");
    assert!(
        err.to_string().contains("Table not found"),
        "unexpected error: {}",
        err
    );
}
