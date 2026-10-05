//! Regression tests for Issue #5009 — `SHOW DATABASES` must list the
//! databases the user actually created.
//!
//! Background
//! ----------
//! `execute_show_databases` returned a single hard-coded `default` row
//! (introduced by V312-58 / Issue #4516, whose scope was the *row shape*
//! and the wire column header, not the *set of rows*). The consequence
//! was that `CREATE DATABASE x` succeeded, `DROP DATABASE x` succeeded,
//! and `SHOW DATABASES` never mentioned `x` — the engine had no way to
//! ask, because the `StorageEngine` trait exposed `create_database` /
//! `drop_database` but had no enumerator at all.
//!
//! These tests pin the observable contract: create -> listed, drop ->
//! gone, duplicates collapse, and a name that sorts before `default`
//! does not displace the implicit `default` from the result.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{MemoryStorage, Value};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Collect the `SHOW DATABASES` column as plain strings.
fn databases(e: &mut ExecutionEngine<MemoryStorage>) -> Vec<String> {
    let r = e.execute("SHOW DATABASES").unwrap_or_else(|err| {
        panic!("SHOW DATABASES failed: {err}");
    });
    r.rows
        .iter()
        .map(|row| match &row[0] {
            Value::Text(s) => s.clone(),
            other => panic!("expected Text, got {other:?}"),
        })
        .collect()
}

#[test]
fn created_database_appears_in_show_databases() {
    let mut e = engine();
    e.execute("CREATE DATABASE shop").unwrap();
    let dbs = databases(&mut e);
    assert!(
        dbs.iter().any(|d| d == "shop"),
        "#5009: a created database must be listed; got {dbs:?}"
    );
}

#[test]
fn multiple_databases_are_all_listed() {
    let mut e = engine();
    e.execute("CREATE DATABASE shop").unwrap();
    e.execute("CREATE DATABASE blog").unwrap();
    e.execute("CREATE DATABASE warehouse").unwrap();
    let dbs = databases(&mut e);
    for want in ["shop", "blog", "warehouse"] {
        assert!(
            dbs.iter().any(|d| d == want),
            "#5009: {want} missing from {dbs:?}"
        );
    }
    assert_eq!(dbs.len(), 4, "#5009: default + 3 created; got {dbs:?}");
}

#[test]
fn drop_database_removes_it_from_listing() {
    let mut e = engine();
    e.execute("CREATE DATABASE shop").unwrap();
    e.execute("CREATE DATABASE blog").unwrap();
    e.execute("DROP DATABASE blog").unwrap();
    let dbs = databases(&mut e);
    assert!(
        !dbs.iter().any(|d| d == "blog"),
        "#5009: dropped database must disappear; got {dbs:?}"
    );
    assert!(
        dbs.iter().any(|d| d == "shop"),
        "#5009: surviving database must remain; got {dbs:?}"
    );
}

#[test]
fn duplicate_create_does_not_duplicate_the_row() {
    let mut e = engine();
    e.execute("CREATE DATABASE shop").unwrap();
    e.execute("CREATE DATABASE shop").unwrap();
    let dbs = databases(&mut e);
    assert_eq!(
        dbs.iter().filter(|d| d.as_str() == "shop").count(),
        1,
        "#5009: re-creating must not duplicate the row; got {dbs:?}"
    );
}

#[test]
fn default_is_always_present() {
    let mut e = engine();
    e.execute("CREATE DATABASE shop").unwrap();
    let dbs = databases(&mut e);
    assert!(
        dbs.iter().any(|d| d == "default"),
        "#5009: the implicit working database must always be listed; got {dbs:?}"
    );
}

#[test]
fn result_is_sorted_and_default_survives_earlier_names() {
    // A user database sorting *before* "default" is the case that would
    // break any implementation that assumes "default" is row 0.
    let mut e = engine();
    e.execute("CREATE DATABASE aaa").unwrap();
    let dbs = databases(&mut e);
    assert_eq!(
        dbs,
        vec!["aaa".to_string(), "default".to_string()],
        "#5009: output must be name-sorted and keep `default`"
    );
}

#[test]
fn empty_engine_still_lists_only_default() {
    // Pins the pre-existing #4516 contract that the hard-coded row was
    // there to satisfy: an engine with no user databases shows one row.
    let mut e = engine();
    assert_eq!(databases(&mut e), vec!["default".to_string()]);
}

// ---------------------------------------------------------------------------
// #5019 — `SHOW SCHEMAS` is an exact synonym of `SHOW DATABASES` in MySQL.
// Before the fix it hit the generic arm and produced
//   Parse error: Unexpected token after SHOW: Identifier("SCHEMAS")
// ---------------------------------------------------------------------------

#[test]
fn show_schemas_is_a_synonym_for_show_databases() {
    let mut e = engine();
    e.execute("CREATE DATABASE shop").unwrap();
    let r = e
        .execute("SHOW SCHEMAS")
        .unwrap_or_else(|err| panic!("SHOW SCHEMAS failed: {err}"));
    let names: Vec<String> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Text(s) => s.clone(),
            other => panic!("expected Text, got {other:?}"),
        })
        .collect();
    assert!(
        names.iter().any(|n| n == "shop"),
        "#5019: SHOW SCHEMAS must list created databases; got {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "default"),
        "#5019: SHOW SCHEMAS must keep listing `default`; got {names:?}"
    );
}

#[test]
fn show_schemas_and_show_databases_agree() {
    let mut e = engine();
    e.execute("CREATE DATABASE a_db").unwrap();
    e.execute("CREATE DATABASE b_db").unwrap();
    let via_databases = databases(&mut e);
    let via_schemas = e.execute("SHOW SCHEMAS").unwrap();
    let via_schemas: Vec<String> = via_schemas
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Text(s) => s.clone(),
            other => panic!("expected Text, got {other:?}"),
        })
        .collect();
    assert_eq!(
        via_databases, via_schemas,
        "#5019: SHOW SCHEMAS and SHOW DATABASES must return identical rows"
    );
}

#[test]
fn show_schemas_on_empty_engine_returns_default() {
    let mut e = engine();
    let r = e.execute("SHOW SCHEMAS").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "default"));
}
