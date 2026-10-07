//! #5025: table-space isolation by database — end-to-end regression tests.
//!
//! The symptom this pins, measured over the real MySQL protocol before the
//! fix (baseline `f900f4fb6`):
//!
//! ```sql
//! CREATE DATABASE d1;  CREATE DATABASE d2;
//! -- t1 created in d1, t2 created in d2
//! USE d1; SHOW TABLES;        -- t1 t2 vectors documents content
//! SHOW TABLES FROM d2;        -- t1 t2 vectors documents content   <-- identical
//! SELECT COUNT(*) FROM t2;   -- 0                                  <-- d2's table visible from d1
//! ```
//!
//! All three said the same thing: **d1 and d2 shared one table namespace.**
//! The `documents` / `vectors` / `content` entries are per-database system
//! tables and are unrelated to isolation.
//!
//! `tests/e2e/multi_db_e2e_test.rs` could not catch this. It drives
//! `MemoryExecutionEngine` over CREATE/DROP DATABASE only — no cross-database
//! visibility assertion anywhere in it — so all 12 of its tests stayed green
//! while two databases could read each other's tables.
//!
//! These tests use the same in-process engine, so they run without a server.
//! The FileStorage + mysql-server production path is covered separately by
//! `probe_db_isolation.sh`; a unit-level test cannot reach it.

use parking_lot::RwLock;
use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn make_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

/// `USE` must switch the table namespace, and only a table that exists in
/// the current database may be read.
#[test]
fn issue_5025_tables_are_invisible_across_databases() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    engine
        .execute("INSERT INTO t1 VALUES (1)")
        .expect("insert t1");

    // The same table name in the other database is a DIFFERENT table.
    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("t1 in d2 must not clash");

    // d2's t1 starts empty — d1's row is not visible here.
    let rows = engine
        .execute("SELECT COUNT(*) FROM t1")
        .expect("select from d2's t1");
    let text = format!("{rows:?}");
    assert!(
        text.contains('0'),
        "d2's t1 must not see d1's row (shared table namespace), got: {text}"
    );

    // Back in d1 the original row is still there.
    engine.execute("USE d1").expect("use d1");
    let rows = engine
        .execute("SELECT COUNT(*) FROM t1")
        .expect("select from d1's t1");
    let text = format!("{rows:?}");
    assert!(
        text.contains('1'),
        "d1's t1 must still hold its own row, got: {text}"
    );
}

/// `SHOW TABLES` must list the current database's tables only.
#[test]
fn issue_5025_show_tables_scoped_to_current_database() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE only_in_d1 (id INT)")
        .expect("create in d1");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE only_in_d2 (id INT)")
        .expect("create in d2");

    engine.execute("USE d1").expect("use d1");
    let out = format!("{:?}", engine.execute("SHOW TABLES").expect("show tables"));
    assert!(
        out.contains("only_in_d1"),
        "d1 must list its own table, got: {out}"
    );
    assert!(
        !out.contains("only_in_d2"),
        "d1 must NOT list d2's table (shared namespace), got: {out}"
    );
}

/// `USE` of an unknown database must fail. Before the fix it returned
/// `Ok(0)` and silently kept using the previous database — which is how a
/// typo'd database name became a write into the wrong place.
#[test]
fn issue_5025_use_unknown_database_is_rejected() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");

    let r = engine.execute("USE no_such_db");
    assert!(
        r.is_err(),
        "USE of an unknown database must error, not silently keep the old one: {:?}",
        r.map(|v| format!("{v:?}"))
    );

    // The rejected USE must not have changed the active database.
    engine
        .execute("CREATE TABLE still_in_default (id INT)")
        .expect("create after failed USE");
    let out = format!("{:?}", engine.execute("SHOW TABLES").expect("show tables"));
    assert!(
        out.contains("still_in_default"),
        "a rejected USE must leave the active database unchanged, got: {out}"
    );
}

/// `SHOW TABLES FROM <db>` must list the TARGET database's tables.
/// Before this fix the `db` argument was dropped on the floor
/// (`_db` in `execute_show_tables_with_filter`) and the current
/// database's tables were returned under the target's name — the
/// metadata view lied, exactly the failure mode #5025's body warns about.
#[test]
fn issue_5025_show_tables_from_lists_target_database() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE only_in_d1 (id INT)")
        .expect("create in d1");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE only_in_d2 (id INT)")
        .expect("create in d2");

    engine.execute("USE d1").expect("use d1");
    let out = format!(
        "{:?}",
        engine
            .execute("SHOW TABLES FROM d2")
            .expect("show tables from d2")
    );
    assert!(
        out.contains("only_in_d2"),
        "SHOW TABLES FROM d2 must list d2's table, got: {out}"
    );
    assert!(
        !out.contains("only_in_d1"),
        "SHOW TABLES FROM d2 must NOT list the current database's table (the db argument was dropped), got: {out}"
    );
}

/// `SHOW TABLES FROM` with a LIKE filter must filter the TARGET
/// database's tables, not the current database's.
#[test]
fn issue_5025_show_tables_from_like_filters_target_database() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d2").expect("create d2");
    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE keep_this (id INT)")
        .expect("create keep_this");
    engine
        .execute("CREATE TABLE skip_that (id INT)")
        .expect("create skip_that");

    let out = format!(
        "{:?}",
        engine
            .execute("SHOW TABLES FROM d2 LIKE 'keep%'")
            .expect("show tables from d2 like")
    );
    assert!(
        out.contains("keep_this"),
        "LIKE 'keep%' must match d2's table, got: {out}"
    );
    assert!(
        !out.contains("skip_that"),
        "LIKE 'keep%' must not match d2's other table, got: {out}"
    );
}

/// `SHOW TABLES FROM <unknown>` must fail loudly, matching `USE`'s
/// `Unknown database` semantics (PR #5044) instead of silently listing
/// the current database.
#[test]
fn issue_5025_show_tables_from_unknown_database_is_rejected() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("USE d1").expect("use d1");

    let r = engine.execute("SHOW TABLES FROM no_such_db");
    assert!(
        r.is_err(),
        "SHOW TABLES FROM of an unknown database must error, not list the current database: {:?}",
        r.map(|v| format!("{v:?}"))
    );
}

/// `SHOW FULL TABLES FROM <db>` shares the same dropped-`db` defect as
/// the bare form (`execute_show_full_tables`), so it needs its own pin.
#[test]
fn issue_5025_show_full_tables_from_lists_target_database() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE only_in_d1 (id INT)")
        .expect("create in d1");
    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE only_in_d2 (id INT)")
        .expect("create in d2");

    engine.execute("USE d1").expect("use d1");
    let out = format!(
        "{:?}",
        engine
            .execute("SHOW FULL TABLES FROM d2")
            .expect("show full tables from d2")
    );
    assert!(
        out.contains("only_in_d2"),
        "SHOW FULL TABLES FROM d2 must list d2's table, got: {out}"
    );
    assert!(
        !out.contains("only_in_d1"),
        "SHOW FULL TABLES FROM d2 must NOT list the current database's table, got: {out}"
    );
}

/// `SHOW TABLE STATUS FROM <db>` shares the same dropped-`db` defect
/// (`execute_show_table_status`), so it needs its own pin as well.
#[test]
fn issue_5025_show_table_status_from_lists_target_database() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE only_in_d1 (id INT)")
        .expect("create in d1");
    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE only_in_d2 (id INT)")
        .expect("create in d2");

    engine.execute("USE d1").expect("use d1");
    let out = format!(
        "{:?}",
        engine
            .execute("SHOW TABLE STATUS FROM d2")
            .expect("show table status from d2")
    );
    assert!(
        out.contains("only_in_d2"),
        "SHOW TABLE STATUS FROM d2 must include d2's table, got: {out}"
    );
    assert!(
        !out.contains("only_in_d1"),
        "SHOW TABLE STATUS FROM d2 must NOT include the current database's table, got: {out}"
    );
}
