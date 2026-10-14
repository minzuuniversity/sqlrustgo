//! #5268: `CREATE TABLE … AS SELECT *` must store the **resolved** column
//! names, not the parser's `"*"` placeholder.
//!
//! `execute_create_table` (src/engine_create.rs) inferred the new table's
//! column names from the raw SELECT AST (`select_stmt.columns` → alias or
//! name). For `SELECT *` the parser emits a single placeholder column whose
//! name is the literal `"*"`, which is expanded to real columns only at
//! execution time. The executed `select_result` therefore already carries the
//! true schema (`id`, `v`, …), but CTAS ignored it and stored `["*"]`.
//!
//! Consequence: `SELECT * FROM d1.tb` returned the right rows (data was
//! inserted correctly), but any filtered / sorted query
//! (`WHERE id = 1`, `ORDER BY id`) failed in the binder with
//! `column 'id' not found in schema`, because the *declared* schema only had
//! a `"*"` column.
//!
//! This test pins the fix: after a `CTAS … SELECT *`, the resulting table's
//! schema must contain the real source column names so that WHERE / ORDER BY
//! bind.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use std::path::PathBuf;
use std::sync::Arc;

type FS = FileStorage;

fn engine(dir: &PathBuf) -> ExecutionEngine<FS> {
    let storage = Arc::new(parking_lot::RwLock::new(
        FS::new(dir.clone()).expect("open"),
    ));
    ExecutionEngine::new(storage)
}

fn tmpdir(tag: &str) -> PathBuf {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("ctas_star_{}_{}_{}", tag, std::process::id(), n));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

/// `CREATE TABLE AS SELECT *` must materialise the real column names so a
/// subsequent `WHERE <col> = …` binds (issue #5268).
#[test]
fn ctas_select_star_stores_real_column_names_for_where_binding() {
    let mut e = engine(&tmpdir("star"));
    e.execute("CREATE DATABASE d1").expect("d1");
    e.execute("USE d1").expect("use d1");
    e.execute("CREATE TABLE seed (id INT PRIMARY KEY, v INT)")
        .expect("seed");
    e.execute("INSERT INTO seed VALUES (1,100),(2,200)")
        .expect("seed rows");
    e.execute("CREATE TABLE tb AS SELECT * FROM seed")
        .expect("ctas");

    // Data is present (this already worked).
    let all = e.execute("SELECT * FROM tb").expect("select *");
    assert_eq!(all.rows.len(), 2, "CTAS copied both rows");

    // The binder must recognise `id` as a real column of `tb`.
    let filtered = e
        .execute("SELECT * FROM tb WHERE id = 1")
        .expect("WHERE id = 1 must bind against the CTAS table schema");
    assert_eq!(
        filtered.rows.len(),
        1,
        "WHERE id = 1 must return the single matching row"
    );

    // ORDER BY on a real column must also bind.
    let ordered = e
        .execute("SELECT * FROM tb ORDER BY id")
        .expect("ORDER BY id must bind against the CTAS table schema");
    assert_eq!(ordered.rows.len(), 2, "ORDER BY returns both rows");
}
