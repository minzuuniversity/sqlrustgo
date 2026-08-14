//! ForeignKeyConstraint / ColumnDefinition API stability tests (Issue #4170 / V312-36)
//!
//! These tests pin the public API of `ForeignKeyConstraint` and
//! `ColumnDefinition` so that future struct refactors surface as
//! compile/test failures here rather than as drift in callers like
//! `crates/tools/src/backup.rs`. They do NOT add new fields; they
//! lock in the shape that callers and binaries currently depend on.
//!
//! The V312-36 issue was detected on 250 HEAD with 60+ compile errors
//! across `foreign_key_test`, `mysql_compatibility_test`, and
//! `teaching_scenario_test`. PR #4165 propagated the struct extensions
//! to bench + test_data; the residual fields described in the issue
//! (`referenced_column` singular, `collation`) were already addressed
//! upstream. This test pins the current state so any regression is
//! caught immediately.

use sqlrustgo_storage::{ColumnDefinition, ForeignKeyAction, ForeignKeyConstraint, TableInfo};

#[test]
fn foreign_key_constraint_has_expected_fields() {
    // Issue #4170 acceptance criterion: `ForeignKeyConstraint { referenced_columns: Vec<String>,
    // on_delete: Option<...>, on_update: Option<...> }` must compile.
    let fk = ForeignKeyConstraint {
        name: Some("fk_orders_users".to_string()),
        columns: vec!["user_id".to_string()],
        referenced_table: "users".to_string(),
        referenced_columns: vec!["id".to_string()],
        on_delete: Some(ForeignKeyAction::Cascade),
        on_update: Some(ForeignKeyAction::NoAction),
    };
    assert_eq!(fk.referenced_table, "users");
    assert_eq!(fk.referenced_columns, vec!["id".to_string()]);
    assert_eq!(fk.on_delete, Some(ForeignKeyAction::Cascade));
}

#[test]
fn foreign_key_constraint_supports_empty_referenced_columns() {
    // Edge case: declaration `REFERENCES table` without column list —
    // MySQL allows this when the PK is unambiguous. Storage must
    // represent it as an empty Vec (not panic).
    let fk = ForeignKeyConstraint {
        name: None,
        columns: vec!["user_id".to_string()],
        referenced_table: "users".to_string(),
        referenced_columns: vec![],
        on_delete: None,
        on_update: None,
    };
    assert!(fk.referenced_columns.is_empty());
}

#[test]
fn foreign_key_constraint_supports_multi_column_references() {
    // Composite FK: `FOREIGN KEY (a, b) REFERENCES parent(x, y)`.
    // The plural `referenced_columns` field is the correct shape per
    // SQL:1999 and MySQL 8.0; a singular `referenced_column` would
    // not be sufficient. Pin this so the plural form is preserved.
    let fk = ForeignKeyConstraint {
        name: Some("fk_composite".to_string()),
        columns: vec!["a".to_string(), "b".to_string()],
        referenced_table: "parent".to_string(),
        referenced_columns: vec!["x".to_string(), "y".to_string()],
        on_delete: None,
        on_update: None,
    };
    assert_eq!(fk.referenced_columns.len(), 2);
}

#[test]
fn column_definition_has_collation_field() {
    // Issue #4170 acceptance: `ColumnDefinition.collation` exists and
    // is `Option<String>` so EXCEPT/INTERSECT collation-aware
    // executors can dispatch on it.
    let col = ColumnDefinition {
        name: "name".to_string(),
        data_type: "TEXT".to_string(),
        nullable: true,
        primary_key: false,
        char_max_length: Some(50),
        collation: Some("NOCASE".to_string()),

        default_value: None,    };
    assert_eq!(col.collation.as_deref(), Some("NOCASE"));
    assert!(col.nullable);
}

#[test]
fn column_definition_default_collation_is_none() {
    // Default `collation: None` means binary / case-sensitive
    // comparison (V4077 / Issue #4077 semantics).
    let col = ColumnDefinition::new("id", "INTEGER");
    assert!(col.collation.is_none());
    assert!(!col.primary_key);
    assert!(col.char_max_length.is_none());
}

#[test]
fn table_info_round_trip_with_foreign_key() {
    // Pins that a TableInfo carrying a single-column FK can be
    // constructed via public API. Catches regressions where someone
    // renames fields or moves them out of the public surface.
    let table = TableInfo {
        name: "orders".to_string(),
        columns: vec![ColumnDefinition {
            name: "user_id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,

            default_value: None,        }],
        foreign_keys: vec![ForeignKeyConstraint {
            name: Some("fk_user".to_string()),
            columns: vec!["user_id".to_string()],
            referenced_table: "users".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete: Some(ForeignKeyAction::Cascade),
            on_update: None,
        }],
        unique_constraints: vec![],
        check_constraints: vec![],
        collations: std::collections::HashMap::new(),
        partition_info: None,
        compression: None,
    };
    assert_eq!(table.foreign_keys.len(), 1);
    assert_eq!(
        table.foreign_keys[0].referenced_columns,
        vec!["id".to_string()]
    );
}
