//! #5271 R2-A: a projected source column carries the source column's type.
//!
//! `SELECT *` already reports real types — C4 confirmed `INT`→3,
//! `DOUBLE`→5, `BIGINT`→8 against a real client. Anything spelled out in the
//! projection list went through `ResultColumn::unnamed_type`, so every one of
//! them came out as `VAR_STRING` on the wire regardless of what it selected.
//!
//! These assert on `data_type`, not on the protocol encoding, and on values
//! as well as names: a projection that reorders columns must reorder the
//! types and the data together, and an alias must rename the output without
//! changing what it means.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Arc;

fn engine(tag: &str) -> (ExecutionEngine<FileStorage>, PathBuf) {
    let dir = std::env::temp_dir().join(format!("r2a_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let storage = Arc::new(parking_lot::RwLock::new(
        FileStorage::new(dir.clone()).expect("FileStorage"),
    ));
    (ExecutionEngine::new(storage), dir)
}

fn types(e: &mut ExecutionEngine<FileStorage>, sql: &str) -> Vec<(String, Option<String>)> {
    let r = e.execute(sql).unwrap_or_else(|err| panic!("{sql}: {err}"));
    r.columns
        .iter()
        .map(|c| (c.name.clone(), c.data_type.clone()))
        .collect()
}

/// A named projection reports the source column's type.
#[test]
fn named_projection_inherits_the_source_type() {
    let (mut e, dir) = engine("named");
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, v INT, s VARCHAR(32), d DOUBLE)")
        .unwrap();

    let got = types(&mut e, "SELECT id FROM t");
    assert_eq!(got[0].0.as_str(), "id");
    assert_eq!(
        got[0].1.as_deref(),
        Some("INT"),
        "SELECT id must report INT, got {:?}",
        got[0].1
    );

    let got = types(&mut e, "SELECT s FROM t");
    assert_eq!(got[0].1.as_deref(), Some("VARCHAR"));

    let got = types(&mut e, "SELECT d FROM t");
    assert_eq!(got[0].1.as_deref(), Some("DOUBLE"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// An alias renames the output and nothing else.
#[test]
fn alias_renames_without_changing_the_type() {
    let (mut e, dir) = engine("alias");
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();

    let got = types(&mut e, "SELECT id AS k FROM t");
    assert_eq!(got[0].0.as_str(), "k");
    assert_eq!(got[0].1.as_deref(), Some("INT"));

    // The alias must not be used to look the source column up.
    let got = types(&mut e, "SELECT v AS id FROM t");
    assert_eq!(got[0].0.as_str(), "id");
    assert_eq!(
        got[0].1.as_deref(),
        Some("INT"),
        "SELECT v AS id is still v — an INT column named `v`"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Reordering the projection reorders names, types and values together.
#[test]
fn reordering_reorders_types_with_values() {
    let (mut e, dir) = engine("reorder");
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, s VARCHAR(8), d DOUBLE)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1,'x',2.5)").unwrap();

    let r = e.execute("SELECT s, d, id FROM t").unwrap();
    let names: Vec<&str> = r.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["s", "d", "id"]);

    let types: Vec<Option<&str>> = r.columns.iter().map(|c| c.data_type.as_deref()).collect();
    assert_eq!(
        types,
        vec![Some("VARCHAR"), Some("DOUBLE"), Some("INT")],
        "types must follow the projection order, not the table's"
    );

    assert_eq!(r.rows[0][0], Value::Text("x".into()));
    assert_eq!(r.rows[0][1], Value::Float(2.5));
    assert_eq!(r.rows[0][2], Value::Integer(1));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Types are a property of the schema, so an empty table still reports them.
#[test]
fn empty_table_keeps_the_column_types() {
    let (mut e, dir) = engine("empty");
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, s VARCHAR(16))")
        .unwrap();

    let r = e.execute("SELECT id, s FROM t").unwrap();
    assert!(r.rows.is_empty());
    assert_eq!(r.columns[0].data_type.as_deref(), Some("INT"));
    assert_eq!(r.columns[1].data_type.as_deref(), Some("VARCHAR"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Same column name, different tables, different types. A lookup that lost the
/// database would answer both with the same type — the shape of the #5250 and
/// #5266 namespace bugs, in a new place.
#[test]
fn same_named_columns_in_two_databases_keep_their_own_types() {
    let (mut e, dir) = engine("crossdb");
    e.execute("CREATE DATABASE d1").unwrap();
    e.execute("CREATE DATABASE d2").unwrap();
    e.execute("CREATE TABLE d1.t(id INT PRIMARY KEY)").unwrap();
    e.execute("CREATE TABLE d2.t(id VARCHAR(20) PRIMARY KEY)")
        .unwrap();

    let d1 = types(&mut e, "SELECT id FROM d1.t");
    assert_eq!(d1[0].1.as_deref(), Some("INT"), "d1.t.id is an INT");

    let d2 = types(&mut e, "SELECT id FROM d2.t");
    assert_eq!(d2[0].1.as_deref(), Some("VARCHAR"), "d2.t.id is a VARCHAR");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `SELECT *` keeps working, and an expression we do not resolve stays
/// `None` rather than being given a guessed type.
#[test]
fn star_is_unchanged_and_unknown_expressions_stay_unresolved() {
    let (mut e, dir) = engine("star");
    e.execute("CREATE TABLE t(id INT PRIMARY KEY, s VARCHAR(8))")
        .unwrap();

    let got = types(&mut e, "SELECT * FROM t");
    assert_eq!(got[0].1.as_deref(), Some("INT"));
    assert_eq!(got[1].1.as_deref(), Some("VARCHAR"));

    // R2-B territory: aggregates and arithmetic are not resolved yet, and
    // saying so is the point. A wrong type would be worse than none.
    for sql in [
        "SELECT COUNT(*) FROM t",
        "SELECT SUM(s) FROM t",
        "SELECT id + 1 FROM t",
    ] {
        let got = types(&mut e, sql);
        assert_eq!(
            got[0].1, None,
            "{sql} is not covered by R2-A and must not claim a type"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
