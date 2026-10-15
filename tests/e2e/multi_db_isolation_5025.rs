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

// ===== issue #5137 阶段 1-2: 跨库限定名 SELECT db.table =====

/// #5137 AC4a: `SELECT * FROM d2.t2` while connected to d1 must read
/// d2's table. Before the fix the schema qualifier was parsed into
/// `SelectStatement.schema` but never consulted by the FROM resolver,
/// so the bare name was looked up in the CURRENT database and failed
/// with `Table not found: t2`.
#[test]
fn issue_5137_select_from_qualified_table_reads_target_database() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    engine
        .execute("INSERT INTO t1 VALUES (1)")
        .expect("insert into t1");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    engine
        .execute("INSERT INTO t2 VALUES (42)")
        .expect("insert into t2");

    // Back in d1: the qualified name must reach across into d2.
    engine.execute("USE d1").expect("use d1");
    let rows = engine
        .execute("SELECT * FROM d2.t2")
        .expect("SELECT from d2.t2 must read d2, not fail in d1");
    let text = format!("{rows:?}");
    assert!(
        text.contains("42"),
        "d2.t2's row (42) must be visible via the qualified name, got: {text}"
    );
    assert!(
        !text.contains("Integer(1)"),
        "must not accidentally read d1's t1, got: {text}"
    );
}

/// #5137 AC4d: the mirrored direction — connected to d2, qualified
/// `FROM d1.t1` must read d1.
#[test]
fn issue_5137_qualified_table_reads_other_database_symmetric() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    engine
        .execute("INSERT INTO t1 VALUES (7)")
        .expect("insert into t1");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");

    let rows = engine
        .execute("SELECT * FROM d1.t1")
        .expect("SELECT from d1.t1 must read d1 while connected to d2");
    let text = format!("{rows:?}");
    assert!(
        text.contains('7'),
        "d1.t1's row (7) must be visible via the qualified name, got: {text}"
    );
}

/// #5137: a qualified name against an UNKNOWN database must fail with
/// the same `Unknown database` error `USE` produces — not silently
/// fall back to the current database (which would read the WRONG table)
/// and not a bare `Table not found`.
#[test]
fn issue_5137_qualified_unknown_database_is_rejected() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");

    let r = engine.execute("SELECT * FROM no_such_db.t1");
    let err = format!("{:?}", r.expect_err("qualified unknown db must error"));
    assert!(
        err.contains("Unknown database"),
        "must be rejected like USE does (Unknown database), got: {err}"
    );
    assert!(
        !err.contains("Table not found"),
        "must not degrade into a bare Table-not-found in the current db, got: {err}"
    );
}

/// #5137 AC4b: WHERE against a qualified FROM with a bare column
/// reference — the qualifier only affects table resolution, the column
/// space is still the target table's.
#[test]
fn issue_5137_qualified_table_where_filter_works() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    engine
        .execute("INSERT INTO t2 VALUES (42)")
        .expect("insert 42");
    engine
        .execute("INSERT INTO t2 VALUES (99)")
        .expect("insert 99");

    engine.execute("USE d1").expect("use d1");
    let rows = engine
        .execute("SELECT id FROM d2.t2 WHERE id = 42")
        .expect("qualified FROM + WHERE must work");
    let text = format!("{rows:?}");
    assert!(
        text.contains('4') && text.contains('2'),
        "the matching row (42) must come back, got: {text}"
    );
    assert!(
        !text.contains('9'),
        "the non-matching row (99) must be filtered out, got: {text}"
    );
}

/// #5137 AC4e: a comma list spanning two databases — the primary table
/// and every `extra_tables` / auto-rewritten join entry must each be
/// resolved against its own database, not the connection's current one.
#[test]
fn issue_5137_comma_list_across_databases() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    engine
        .execute("INSERT INTO t1 VALUES (1)")
        .expect("insert into t1");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    engine
        .execute("INSERT INTO t2 VALUES (42)")
        .expect("insert into t2");

    engine.execute("USE d1").expect("back to d1");
    let rows = engine
        .execute("SELECT * FROM d1.t1, d2.t2")
        .expect("cross-db comma list must execute");
    let text = format!("{rows:?}");
    assert!(
        text.contains('1') && text.contains("42"),
        "cartesian join must see d1.t1 (1) and d2.t2 (42), got: {text}"
    );
}

/// #5137 AC4c: three-segment column reference `db.table.col` in the
/// SELECT list. Before the fix the parser rejected it outright with
/// `Parse error: Expected expression` — the bare-identifier path never
/// accepted a Dot chain long enough for `d2.t2.id`.
#[test]
fn issue_5137_three_segment_column_reference() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    engine
        .execute("INSERT INTO t2 VALUES (42)")
        .expect("insert into t2");

    engine.execute("USE d1").expect("use d1");
    let rows = engine
        .execute("SELECT d2.t2.id FROM d2.t2")
        .expect("three-segment column reference must parse and evaluate");
    let text = format!("{rows:?}");
    assert!(
        text.contains("42"),
        "d2.t2.id must evaluate to 42, got: {text}"
    );
}

/// #5137 AC4f (阶段 3 后半): three-segment column in the WHERE clause
/// AND in a multi-table join predicate. The WHERE path parses through
/// `parse_expression`'s postfix-dot loop (SubqueryField form) and the
/// join-key path splits qualifiers at the FIRST dot — both assumed at
/// most two segments.
#[test]
fn issue_5137_three_segment_where_and_join_predicate() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    engine
        .execute("INSERT INTO t1 VALUES (1)")
        .expect("insert into t1");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    engine
        .execute("INSERT INTO t2 VALUES (42)")
        .expect("insert into t2");

    // WHERE with a three-segment reference (single table).
    engine.execute("USE d1").expect("use d1");
    let rows = engine
        .execute("SELECT id FROM d2.t2 WHERE d2.t2.id = 42")
        .expect("WHERE db.table.col must parse and evaluate");
    let text = format!("{rows:?}");
    assert!(
        text.contains("42"),
        "WHERE d2.t2.id = 42 must match the row, got: {text}"
    );
    let none = engine
        .execute("SELECT id FROM d2.t2 WHERE d2.t2.id = 7")
        .expect("non-matching WHERE must execute");
    let none_text = format!("{none:?}");
    assert!(
        !none_text.contains("42"),
        "WHERE d2.t2.id = 7 must filter the row out, got: {none_text}"
    );

    // Join predicate with three-segment references on both sides
    // (cross-db comma join with WHERE).
    let joined = engine
        .execute("SELECT t1.id FROM d1.t1, d2.t2 WHERE d1.t1.id = d2.t2.id")
        .expect("cross-db join predicate with three-segment refs must execute");
    let joined_text = format!("{joined:?}");
    assert!(
        !joined_text.contains("42") && !joined_text.contains("Integer(1)"),
        "id=1 vs id=42 must join to nothing, got: {joined_text}"
    );
    let joined2 = engine
        .execute("SELECT t1.id FROM d1.t1, d2.t2 WHERE d1.t1.id + 41 = d2.t2.id")
        .expect("cross-db join predicate must execute");
    let joined2_text = format!("{joined2:?}");
    assert!(
        joined2_text.contains("Integer(1)") || joined2_text.contains("42"),
        "matching join predicate must return rows, got: {joined2_text}"
    );
}

/// #5137 A2: hand-written cross-db JOIN with three-segment ON predicate.
/// The join-key builder splits qualifiers at the FIRST dot — `d2.t2.id`
/// yielded qualifier `d2` (the database, not the table) and the lookup
/// missed both sides.
#[test]
fn issue_5137_handwritten_join_on_three_segment() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    engine
        .execute("INSERT INTO t1 VALUES (1)")
        .expect("insert into t1");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    engine
        .execute("INSERT INTO t2 VALUES (1)")
        .expect("insert into t2");

    engine.execute("USE d1").expect("use d1");
    let rows = engine
        .execute("SELECT t1.id FROM d1.t1 JOIN d2.t2 ON d1.t1.id = d2.t2.id")
        .expect("handwritten cross-db JOIN with three-segment ON must execute");
    let text = format!("{rows:?}");
    assert!(
        text.contains("Integer(1)"),
        "matching JOIN must return the joined row, got: {text}"
    );
}

/// #5137 B: cross-db derived subquery inside a hand-written JOIN.
///
/// `execute_joins` materialises derived tables while holding its storage
/// read guard, and the recursive `execute_select` entry validates `d2`
/// through `ensure_database_known` — a second read on the same RwLock.
/// Single-threaded that succeeds (parking_lot allows read reentrancy
/// when no writer is queued); the concurrent deadlock is pinned by
/// `issue_5137_no_deadlock_derived_materialisation_under_writes`.
#[test]
fn issue_5137_cross_db_derived_join_materializes() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");

    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    engine
        .execute("INSERT INTO t1 VALUES (1)")
        .expect("insert 1");
    engine
        .execute("INSERT INTO t1 VALUES (2)")
        .expect("insert 2");

    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    engine
        .execute("INSERT INTO t2 VALUES (2)")
        .expect("insert 2");

    engine.execute("USE d1").expect("use d1");
    // CROSS JOIN (not `JOIN ... ON`): the parser's subquery-join branch
    // consumes `AS x` for the synthetic name but never records it in
    // `JoinClause.alias` (pre-existing, unrelated to #5137), so an ON
    // referencing `x.col` cannot resolve. With no ON, the join key is
    // `JoinKey::All` (cartesian) and no alias is needed — while still
    // exercising the `execute_joins` held-guard materialisation path.
    let rows = engine
        .execute("SELECT t1.id FROM t1 CROSS JOIN (SELECT id FROM d2.t2) AS x")
        .expect("cross-db derived CROSS JOIN must materialise and join");
    let text = format!("{rows:?}");
    assert!(
        text.contains("Integer(2)"),
        "cartesian product must include t1.id=2 joined with d2.t2, got: {text}"
    );
    assert!(
        text.contains("Integer(1)"),
        "cartesian product must include t1.id=1, got: {text}"
    );
}

/// #5137 B (phase-4 lock audit): the derived-materialisation reentrancy
/// must not deadlock under a concurrent writer.
///
/// Reader thread: the cross-db derived-JOIN query in a loop. Its
/// `execute_joins` holds the storage read guard while the recursive
/// entry calls `ensure_database_known` (a blocking `storage.read()`).
/// Writer thread: a tight INSERT loop, so a write request queues up
/// during the reader's guard window. Pre-fix, parking_lot's fair queue
/// parks the writer, then blocks the reader's reentrant read behind it
/// → AB-BA deadlock → the reader never signals completion and
/// `recv_timeout` fires. Post-fix (materialisation happens before the
/// guard is taken) both threads finish quickly.
#[test]
fn issue_5137_no_deadlock_derived_materialisation_under_writes() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut setup = MemoryExecutionEngine::new(storage.clone());
    setup.execute("CREATE DATABASE d1").expect("create d1");
    setup.execute("CREATE DATABASE d2").expect("create d2");
    setup.execute("USE d1").expect("use d1");
    setup
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    // ~20k rows widen the reader's guard window (the base scan runs
    // under the read guard), so the writer's request reliably queues
    // up before the recursive materialisation re-enters the lock.
    for chunk in 0..20 {
        let mut vals = String::new();
        for i in 0..1000 {
            if i > 0 {
                vals.push(',');
            }
            vals.push_str(&format!("({})", chunk * 1000 + i));
        }
        setup
            .execute(&format!("INSERT INTO t1 VALUES {vals}"))
            .expect("bulk insert t1");
    }
    setup
        .execute("CREATE TABLE w (v INT)")
        .expect("writer scratch table");
    setup.execute("USE d2").expect("use d2");
    setup
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    setup
        .execute("INSERT INTO t2 VALUES (2)")
        .expect("insert t2");
    setup.execute("USE d1").expect("back to d1");
    drop(setup);

    let done = Arc::new(AtomicBool::new(false));
    let writer_inserts = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel();

    let reader_storage = storage.clone();
    thread::spawn(move || {
        let mut reader = MemoryExecutionEngine::new(reader_storage);
        // #5057: each engine owns its session database now; the
        // storage-level `current_db` no longer leaks across engines.
        reader.execute("USE d1").expect("reader: use d1");
        for _ in 0..50 {
            let rows = reader
                .execute("SELECT t1.id FROM t1 CROSS JOIN (SELECT id FROM d2.t2) AS x")
                .expect("reader: cross-db derived CROSS JOIN must succeed");
            let text = format!("{rows:?}");
            assert!(
                text.contains("Integer(2)"),
                "reader: expected the joined row, got: {text}"
            );
        }
        tx.send(()).expect("signal reader completion");
    });

    let writer_storage = storage.clone();
    let writer_done = done.clone();
    let writer_counter = writer_inserts.clone();
    thread::spawn(move || {
        let mut writer = MemoryExecutionEngine::new(writer_storage);
        // #5057: session db is per-engine; scratch table `w` lives in d1.
        writer.execute("USE d1").expect("writer: use d1");
        let start = Instant::now();
        while !writer_done.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(60) {
            writer
                .execute("INSERT INTO w VALUES (1)")
                .expect("writer: INSERT must succeed");
            writer_counter.fetch_add(1, Ordering::SeqCst);
        }
    });

    match rx.recv_timeout(Duration::from_secs(30)) {
        Ok(()) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            done.store(true, Ordering::SeqCst);
            panic!("reader thread panicked — see the thread panic log above");
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            done.store(true, Ordering::SeqCst);
            panic!(
                "DEADLOCK: reader stuck in execute_joins while holding the storage read \
                 guard and re-entering ensure_database_known (#5137 B)"
            );
        }
    }
    done.store(true, Ordering::SeqCst);

    let n = writer_inserts.load(Ordering::SeqCst);
    assert!(
        n >= 100,
        "writer made no progress ({n} inserts) — contention never exercised"
    );
}

// ---------------------------------------------------------------------------
// #5137 阶段 C: 有 WHERE 多表跨库路径深检。
//
// 审计结论（engine_select.rs）：hash chain 每步以
// `get_table_info_in(current_db, "d2.t2")` 精确 scoped_key 查询（不剥
// db 前缀），跨库步骤必 miss → `.ok()?` 落笛卡尔回退 + WHERE 后置过滤
// （COMMA_JOIN_WHERE_CONSUMED 只在 hash chain 成功时置位，跨库永不置位）。
// 因此下列五形态必须**结果正确**——任何一个错结果都意味着某条路径错误
// 消费了 WHERE 或读错了库。探针的泄漏断言（负值行不得出现）是敏感指标：
// WHERE 被丢弃时笛卡尔积必然带出负值行。
// ---------------------------------------------------------------------------

fn issue_5137_c_fixture() -> MemoryExecutionEngine {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");
    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create t1 in d1");
    engine
        .execute("INSERT INTO t1 VALUES (1), (2), (42)")
        .expect("fill t1");
    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t2 (id INT)")
        .expect("create t2 in d2");
    engine
        .execute("INSERT INTO t2 VALUES (1), (42)")
        .expect("fill t2");
    engine.execute("USE d1").expect("back to d1");
    engine
}

/// C1: 别名 comma 跨库 + 等值 WHERE。别名使 pair 提取 (a, b) 命中
/// join_tables 别名 → hash chain 会尝试构建，随后跨库步骤
/// `get_table_info_in(current_db, "d2.t2")` 必 miss → 回退。必须结果
/// 正确（1、42 连接；2 是单边行，出现即 WHERE 被错误消费）。
#[test]
fn issue_5137_c_aliased_comma_equi_where() {
    let mut engine = issue_5137_c_fixture();
    let rows = engine
        .execute("SELECT a.id FROM d1.t1 AS a, d2.t2 AS b WHERE a.id = b.id")
        .expect("aliased cross-db comma join with equi-WHERE must execute");
    let text = format!("{rows:?}");
    assert!(
        text.contains("Integer(1)") && text.contains("Integer(42)"),
        "equi-WHERE must join 1 and 42, got: {text}"
    );
    assert!(
        !text.contains("Integer(2)"),
        "id=2 exists only in d1.t1 — WHERE was consumed without filtering, got: {text}"
    );
}

/// C2: 裸限定（t1.id / t2.id，无 db 段）跨库 comma + 等值 WHERE。
/// pair 提取 (t1, t2) 与 join_tables 平名 (d1.t1, d2.t2) 失配 → 不建
/// chain → 回退 + 后置过滤。裸限定求值必须两侧都命中（侧列无前缀时
/// qualifier 对 table_info.name）。
#[test]
fn issue_5137_c_bare_qualifier_comma_equi_where() {
    let mut engine = issue_5137_c_fixture();
    let rows = engine
        .execute("SELECT t1.id FROM d1.t1, d2.t2 WHERE t1.id = t2.id")
        .expect("bare-qualifier cross-db comma join must execute");
    let text = format!("{rows:?}");
    assert!(
        text.contains("Integer(1)") && text.contains("Integer(42)"),
        "equi-WHERE must join 1 and 42, got: {text}"
    );
    assert!(
        !text.contains("Integer(2)"),
        "id=2 must not survive the WHERE, got: {text}"
    );
}

/// C3: 同名表跨库碰撞（d1.t1 与 d2.t1）。若求值把两侧 db 段剥错成
/// 同一张表，则谓词退化为 t1.id = t1.id 恒真 → 全行泄漏（1/3 出现）。
/// 只有 2 是合法交集。
#[test]
fn issue_5137_c_same_name_collision_equi_where() {
    let mut engine = make_engine();
    engine.execute("CREATE DATABASE d1").expect("create d1");
    engine.execute("CREATE DATABASE d2").expect("create d2");
    engine.execute("USE d1").expect("use d1");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create d1.t1");
    engine
        .execute("INSERT INTO t1 VALUES (1), (2)")
        .expect("fill d1.t1");
    engine.execute("USE d2").expect("use d2");
    engine
        .execute("CREATE TABLE t1 (id INT)")
        .expect("create d2.t1");
    engine
        .execute("INSERT INTO t1 VALUES (2), (3)")
        .expect("fill d2.t1");
    engine.execute("USE d1").expect("back to d1");

    let rows = engine
        .execute("SELECT d1.t1.id FROM d1.t1, d2.t1 WHERE d1.t1.id = d2.t1.id")
        .expect("same-name cross-db comma join must execute");
    let text = format!("{rows:?}");
    assert!(
        text.contains("Integer(2)"),
        "intersection id=2 must join, got: {text}"
    );
    assert!(
        !text.contains("Integer(1)") && !text.contains("Integer(3)"),
        "non-intersecting ids leaked — sides resolved to the same table, got: {text}"
    );
}

/// C4: 手写 JOIN 后的后置 WHERE（跨库右表谓词）。ON 消费连接键后，
/// `WHERE d2.t2.id = 42` 必须仍逐行过滤——若 JOIN 路径误置
/// WHERE_CONSUMED 或三段求值读错库，42 之外的行会泄漏或全空。
#[test]
fn issue_5137_c_handwritten_join_post_where() {
    let mut engine = issue_5137_c_fixture();
    let rows = engine
        .execute("SELECT t1.id FROM d1.t1 JOIN d2.t2 ON d1.t1.id = d2.t2.id WHERE d2.t2.id = 42")
        .expect("handwritten cross-db JOIN with post-WHERE must execute");
    let text = format!("{rows:?}");
    assert!(
        text.contains("Integer(42)"),
        "joined row id=42 must pass WHERE, got: {text}"
    );
    assert!(
        !text.contains("Integer(1)"),
        "joined row id=1 must be filtered by WHERE d2.t2.id=42, got: {text}"
    );

    let empty = engine
        .execute("SELECT t1.id FROM d1.t1 JOIN d2.t2 ON d1.t1.id = d2.t2.id WHERE d2.t2.id = 7")
        .expect("non-matching post-WHERE must execute");
    let empty_text = format!("{empty:?}");
    assert!(
        !empty_text.contains("Integer("),
        "WHERE d2.t2.id=7 matches nothing, got: {empty_text}"
    );
}

/// C5: 右侧单表谓词参与 pushdown 提取。`d2.t2.id = 1` 若被错误提取到
/// 左表、或错误消费导致后置过滤跳过，42 会泄漏。
#[test]
fn issue_5137_c_right_table_pushdown_residual() {
    let mut engine = issue_5137_c_fixture();
    let rows = engine
        .execute("SELECT t1.id FROM d1.t1, d2.t2 WHERE d1.t1.id = d2.t2.id AND d2.t2.id = 1")
        .expect("cross-db comma join with right-side residual must execute");
    let text = format!("{rows:?}");
    assert!(
        text.contains("Integer(1)"),
        "id=1 must pass both conjuncts, got: {text}"
    );
    assert!(
        !text.contains("Integer(42)"),
        "residual d2.t2.id=1 must filter out 42, got: {text}"
    );
}
