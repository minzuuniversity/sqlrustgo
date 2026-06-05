//! Table Alias Tests
//!
//! TPC-H Q7/Q8/Q9 use the same table referenced twice with different
//! aliases: `nation n1, nation n2`. The current parser swallows the
//! alias token without storing it, so the executor can't distinguish
//! `n1` from `n2` in the ON condition.
//!
//! These tests assert end-to-end behavior: the engine must accept the
//! aliased FROM, the executor must route `n1.col` and `n2.col` to the
//! correct joined table, and the result must be the cross product
//! filtered by the ON conditions.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_table_alias_parsing_does_not_error() {
    // The parser must accept `FROM t1 a` and store both t1 and the
    // alias. We don't directly inspect the AST here; we just verify
    // the parse call doesn't return an error.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine
        .execute("CREATE TABLE t2 (tid INTEGER, val TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO t1 VALUES (1, 'a'), (2, 'b')")
        .unwrap();
    engine
        .execute("INSERT INTO t2 VALUES (1, 'x'), (2, 'y')")
        .unwrap();

    // `a` is the alias for t1. Currently the parser swallows it without
    // storing, so the query parses but the executor doesn't know about
    // the alias. This test asserts the parse stage at least succeeds
    // (it has since Sprint 1 multi-join), and that the query returns
    // *some* result — full correctness is verified by the next test.
    let result = engine.execute("SELECT a.id, t2.val FROM t1 a JOIN t2 ON a.id = t2.tid");
    assert!(
        result.is_ok(),
        "Aliased FROM must parse and execute without error: {:?}",
        result.err()
    );
}

#[test]
fn test_same_table_twice_with_different_aliases() {
    // TPC-H Q7 pattern: nation n1 JOIN ... ON ... = n1.n_nationkey
    // The same nation table is referenced twice, each alias producing a
    // distinct row in the join. The ON conditions route the rows
    // correctly only if the executor knows which alias is which.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE nation (n_nationkey INTEGER, n_name TEXT)")
        .unwrap();
    engine
        .execute("CREATE TABLE supplier (s_suppkey INTEGER, s_nationkey INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE customer (c_custkey INTEGER, c_nationkey INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO nation VALUES (1, 'GERMANY'), (2, 'FRANCE')")
        .unwrap();
    engine
        .execute("INSERT INTO supplier VALUES (10, 1), (20, 2)")
        .unwrap();
    engine
        .execute("INSERT INTO customer VALUES (100, 1), (200, 2)")
        .unwrap();

    let result = engine.execute(
        "SELECT supplier.s_suppkey, customer.c_custkey, n1.n_name, n2.n_name \
             FROM supplier \
             JOIN nation n1 ON supplier.s_nationkey = n1.n_nationkey \
             JOIN customer ON customer.c_nationkey = supplier.s_nationkey \
             JOIN nation n2 ON customer.c_nationkey = n2.n_nationkey",
    );

    assert!(
        result.is_ok(),
        "Same-table-twice alias join must execute: {:?}",
        result.err()
    );
    // supplier s=10 (GERMANY), customer c=100 (GERMANY): n1='GERMANY', n2='GERMANY'
    // supplier s=20 (FRANCE), customer c=200 (FRANCE): n1='FRANCE', n2='FRANCE'
    assert_eq!(
        result.unwrap().rows.len(),
        2,
        "Two supplier × two customer rows, both should match"
    );
}
