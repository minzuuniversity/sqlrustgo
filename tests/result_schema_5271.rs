//! #5271 B0: a SELECT's `ExecutorResult.columns` must describe the result,
//! not be inferred from its rows.
//!
//! The zero-row case is the one that matters. `SELECT * FROM t` over an empty
//! `t(id INT, v INT)` returns no rows and two columns; an empty result is not
//! an empty schema. Before the output-schema channel existed, the engine
//! compared the table's column count against the first row's width and, when
//! they disagreed, fell back to synthesized `c1`/`c2` names — which for an
//! empty table meant *no* columns at all.
//!
//! These assertions are on the value a real SELECT returns. Constructing an
//! `ExecutorResult` by hand would only test the struct.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Arc;

fn engine(
    tag: &str,
) -> (
    ExecutionEngine<FileStorage>,
    Arc<parking_lot::RwLock<FileStorage>>,
    PathBuf,
) {
    let dir = std::env::temp_dir().join(format!("schema5271_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let storage = Arc::new(parking_lot::RwLock::new(
        FileStorage::new(dir.clone()).expect("FileStorage"),
    ));
    let engine = ExecutionEngine::new(storage.clone());
    (engine, storage, dir)
}

/// The gate: zero rows, but the two columns the table declares.
#[test]
fn star_projection_over_empty_table_keeps_both_columns() {
    let (mut engine, _s, dir) = engine("star_empty");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();

    let r = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(
        r.columns.len(),
        2,
        "an empty table still has two columns; got {:?}",
        r.columns
    );
    assert_eq!(r.columns[0].name, "id");
    assert_eq!(r.columns[1].name, "v");
    assert!(r.rows.is_empty(), "the table is empty");

    // `SELECT *` expands from `table_info.columns`, so the types come with it.
    assert_eq!(r.columns[0].data_type.as_deref(), Some("INT"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Non-empty star projection: real names, and they must not be synthesized.
#[test]
fn star_projection_over_populated_table_uses_table_columns() {
    let (mut engine, _s, dir) = engine("star_full");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20)")
        .unwrap();

    let r = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(r.columns.len(), 2);
    assert_eq!(r.columns[0].name, "id");
    assert_eq!(r.columns[1].name, "v");
    assert!(
        !r.columns
            .iter()
            .any(|c| c.name == "col_1" || c.name == "c1"),
        "synthesized names must not reappear: {:?}",
        r.columns
    );
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0], Value::Integer(1));

    let _ = std::fs::remove_dir_all(&dir);
}

/// A named projection reports the projection's name — not the table's.
#[test]
fn aliased_projection_reports_the_alias() {
    let (mut engine, _s, dir) = engine("alias");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (7, 70)").unwrap();

    let r = engine.execute("SELECT id AS k FROM t").unwrap();
    assert_eq!(r.columns.len(), 1);
    assert_eq!(r.columns[0].name, "k");

    let _ = std::fs::remove_dir_all(&dir);
}

/// Row width must agree with the declared columns, on every projection.
#[test]
fn declared_columns_match_row_width() {
    let (mut engine, _s, dir) = engine("width");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        .unwrap();

    for sql in [
        "SELECT * FROM t",
        "SELECT id FROM t",
        "SELECT id, v FROM t",
        "SELECT COUNT(*) FROM t",
        "SELECT * FROM t WHERE id = 2",
        "SELECT * FROM t ORDER BY id",
    ] {
        let r = engine.execute(sql).unwrap_or_else(|e| panic!("{sql}: {e}"));
        for row in &r.rows {
            assert_eq!(
                row.len(),
                r.columns.len(),
                "{sql}: row has {} values but {} columns are declared ({:?})",
                row.len(),
                r.columns.len(),
                r.columns.iter().map(|c| &c.name).collect::<Vec<_>>()
            );
        }
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// Every row must be exactly as wide as the declared schema, and the schema
/// must be asserted independently — on an empty result, the row loop proves
/// nothing on its own.
fn assert_schema_matches_rows(result: &sqlrustgo::ExecutorResult, expected: &[&str]) {
    for (i, row) in result.rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            result.columns.len(),
            "row {i} is {} wide but {} columns are declared",
            row.len(),
            result.columns.len()
        );
    }
    let names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, expected, "declared schema");
}

/// An aggregate alias is the projection's name, and it survives an empty table.
#[test]
fn aggregate_alias_survives_an_empty_table() {
    let (mut engine, _s, dir) = engine("count_alias");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();

    let r = engine.execute("SELECT COUNT(*) AS n FROM t").unwrap();
    assert_schema_matches_rows(&r, &["n"]);
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(0));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A false predicate yields no rows and keeps the projection.
#[test]
fn false_predicate_keeps_the_projection() {
    let (mut engine, _s, dir) = engine("where_false");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();

    let r = engine.execute("SELECT id FROM t WHERE 1=0").unwrap();
    assert_schema_matches_rows(&r, &["id"]);
    assert!(r.rows.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A false HAVING is the same shape of problem, reached through aggregates.
#[test]
fn false_having_keeps_the_projection() {
    let (mut engine, _s, dir) = engine("having_false");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();

    let r = engine
        .execute("SELECT id FROM t GROUP BY id HAVING COUNT(*) > 5")
        .unwrap();
    assert!(r.rows.is_empty());
    assert!(
        !r.columns.is_empty(),
        "a false HAVING still describes its projection, got {:?}",
        r.columns
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// DISTINCT removes rows, not columns.
#[test]
fn distinct_does_not_change_the_schema() {
    let (mut engine, _s, dir) = engine("distinct");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (1, 10), (2, 20)")
        .unwrap();

    let r = engine.execute("SELECT DISTINCT id FROM t").unwrap();
    assert_schema_matches_rows(&r, &["id"]);
    assert_eq!(r.rows.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A projection must not report more rows wide than it declares columns —
/// the shape that produced `ERROR 2027` at the protocol layer.
#[test]
fn catalog_listings_keep_their_fixed_width() {
    let (mut engine, _s, dir) = engine("catalog");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();

    let tables = engine.execute("SHOW TABLES").unwrap();
    for row in &tables.rows {
        assert_eq!(row.len(), tables.columns.len());
    }

    // `SHOW INDEXES` is not in the grammar; the index listing is reached
    // through `SHOW INDEX` — see the parser's error for the former.
    let indexes = engine.execute("SHOW INDEX FROM t").unwrap();
    for row in &indexes.rows {
        assert_eq!(row.len(), indexes.columns.len());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// #5271 value-level regression for the filtered-projection path.
///
/// The old code built `projected` — the rows selected by the projection —
/// and then returned `rows`, the base table's. Column counts can agree by
/// coincidence, so this asserts the *values*, not the width: a projection
/// that happens to have the same width as the table still has to yield the
/// projected rows.
#[test]
fn filtered_projection_returns_projected_values() {
    let (mut engine, _s, dir) = engine("filtered_values");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        .unwrap();

    // One column selected out of two: width 1 vs 2, so a regression here
    // cannot hide behind matching widths.
    let r = engine.execute("SELECT id FROM t WHERE id = 2").unwrap();
    assert_eq!(r.columns.len(), 1);
    assert_eq!(r.columns[0].name, "id");
    assert_eq!(
        r.rows,
        vec![vec![Value::Integer(2)]],
        "the projected column's value, not the base row"
    );

    // Two columns reordered: `v, id` must come back in that order.
    let reordered = engine.execute("SELECT v, id FROM t WHERE id = 3").unwrap();
    assert_eq!(
        reordered
            .columns
            .iter()
            .map(|c| &c.name)
            .collect::<Vec<_>>(),
        vec!["v", "id"]
    );
    assert_eq!(
        reordered.rows,
        vec![vec![Value::Integer(30), Value::Integer(3)]]
    );

    // Two columns narrowed from three: the third must not survive.
    engine
        .execute("CREATE TABLE u(a INT, b INT, c INT)")
        .unwrap();
    engine.execute("INSERT INTO u VALUES (1, 2, 3)").unwrap();
    let narrowed = engine.execute("SELECT a, c FROM u").unwrap();
    assert_eq!(
        narrowed.columns.iter().map(|c| &c.name).collect::<Vec<_>>(),
        vec!["a", "c"]
    );
    assert_eq!(
        narrowed.rows,
        vec![vec![Value::Integer(1), Value::Integer(3)]]
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `ExecutorResult::new()` still defaults to no columns, so constructing a
/// result successfully does not prove it satisfies the contract. This pins
/// the shape that the protocol layer must not be handed.
#[test]
fn rows_without_a_declared_schema_are_detectable() {
    // DDL/DML command replies legitimately carry no schema.
    let command = engine("negative_cmd").1;
    drop(command);

    let (mut engine, _s, dir) = engine("negative");
    engine
        .execute("CREATE TABLE t(id INT PRIMARY KEY, v INT)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, 10)").unwrap();

    let ddl = engine.execute("CREATE TABLE u(id INT)").unwrap();
    assert!(
        ddl.columns.is_empty() && ddl.rows.is_empty(),
        "a DDL reply is a command, not a result set"
    );

    // Every result set that does have rows must declare its width.
    for sql in [
        "SELECT * FROM t",
        "SELECT id FROM t",
        "SELECT id, v FROM t",
        "SELECT COUNT(*) FROM t",
        "SELECT DISTINCT id FROM t",
    ] {
        let r = engine.execute(sql).unwrap();
        assert!(
            !r.columns.is_empty(),
            "{sql} returned rows but declared no columns: {:?}",
            r.columns
        );
        assert_schema_matches_rows(
            &r,
            &r.columns
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
