//! V312-21 / Issue #4181: Multi-way Join Planner Regression Test
//!
//! Covers join topologies that the previous single-direction greedy
//! `try_comma_join_hash_chain` could not handle:
//!   1. **Star with two hubs** (TPC-H Q7-like): 6-way `n1,n2,s,l,o,c`
//!   2. **Bridge**           (TPC-H Q8-like): 7-way `r,n1,n2,c,o,l,s`
//!
//! (TPC-H Q9-shape is a chain-leaf on a star; its join graph admits
//! no Hamiltonian path, so the linear-chain executor cannot run it
//! through the fast path even with multi-start. Q9 is documented as
//! out-of-scope for this fix and remains on the cartesian fallback.)
//!
//! Failure mode: the greedy grew the chain linearly from a fixed
//! start point, picking the first available neighbor each step. On
//! the star and bridge topologies it dead-ended inside a leaf
//! sub-tree, leaving `chain_order.len() < join_tables.len()`.
//! `try_comma_join_hash_chain` then returned `None`, silently
//! falling back to the per-clause cartesian path. The result was
//! still correct but 5-10x slower on TPC-H SF=1 and infeasible on
//! SF=10.
//!
//! The fix (`build_chain_from_start` + multi-start outer loop)
//! ensures a complete chain is found for any join graph that
//! admits a Hamiltonian path. Each test asserts:
//!   - the row count is correct (sanity check), and
//!   - `engine.last_query_used_comma_join_fast_path()` is `true`
//!     (strict regression check: with the buggy code, the flag
//!     would be `false`).
//!
//! Run:
//!   cargo test --test planner_multi_way_join_test -- --nocapture
//!
//! Source plan: see issue #4181 on the Gitea side. AFP evidence:
//!   source_agent: minimax-m2.7
//!   source_run:   cargo test --test planner_multi_way_join_test
//!   commit:       see fix/v313-4181-multi-way-join-planner

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Build the 6-way star fixture used by `test_6way_star_chain_build`
/// (Q7-shape: `n1,n2,s,l,o,c`).
///
/// Two hubs (lineitem and orders) joined by `l.l_orderkey = o.o_orderkey`,
/// with two nation aliases as leaves joined by `s.s_nationkey` and
/// `c.c_nationkey` respectively.
fn build_6way_star_fixture() -> ExecutionEngine<MemoryStorage> {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, \
         n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    )
    .expect("create nation");
    e.execute(
        "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, \
         s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, \
         s_acctbal REAL NOT NULL, s_comment TEXT)",
    )
    .expect("create supplier");
    e.execute(
        "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, \
         c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, \
         c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT)",
    )
    .expect("create customer");
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, \
         o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, \
         o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, \
         o_comment TEXT)",
    )
    .expect("create orders");
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
         l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, \
         l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, \
         l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, \
         l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, \
         l_shipmode TEXT NOT NULL, l_comment TEXT)",
    )
    .expect("create lineitem");

    // Minimal data set: 1 row per table forms a complete chain when
    // joined on the equality predicates below.
    e.execute("INSERT INTO nation VALUES (1, 'FRANCE', 0, 'comment')")
        .unwrap();
    e.execute("INSERT INTO nation VALUES (2, 'GERMANY', 0, 'comment')")
        .unwrap();
    e.execute("INSERT INTO supplier VALUES (100, 'S#100', 'addr', 1, 'phone', 100.0, 'c')")
        .unwrap();
    e.execute("INSERT INTO customer VALUES (1, 'C#1', 'addr', 2, 'phone', 100.0, 'BUILDING', 'c')")
        .unwrap();
    e.execute(
        "INSERT INTO orders VALUES (1, 1, 'O', 100.0, '1995-03-01', '1-URGENT', \
         'Clerk#000000001', 0, 'comment')",
    )
    .unwrap();
    e.execute(
        "INSERT INTO lineitem VALUES (1, 1, 100, 1, 1.0, 100.0, 0.0, 0.0, 'N', 'O', \
         '1995-03-15', '1995-03-10', '1995-03-20', 'NONE', 'TRUCK', 'comment')",
    )
    .unwrap();
    e
}

/// Build the 6-way chain-leaf fixture (Q9-shape: `p,s,l,ps,o,n`).
///
/// Note: this fixture is currently unused because Q9's join graph
/// admits no Hamiltonian path - `multi_way_hash_chain` cannot run
/// it through the fast path. The fixture remains in place for
/// potential future tree-join executor work (see issue #4181 follow-up).
#[allow(dead_code)]
fn build_6way_chain_leaf_fixture() -> ExecutionEngine<MemoryStorage> {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, \
         n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, \
         p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, \
         p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, \
         p_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, \
         s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, \
         s_acctbal REAL NOT NULL, s_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, \
         ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, \
         PRIMARY KEY (ps_partkey, ps_suppkey))",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, \
         o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, \
         o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, \
         o_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
         l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, \
         l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, \
         l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, \
         l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, \
         l_shipmode TEXT NOT NULL, l_comment TEXT)",
    )
    .unwrap();

    e.execute("INSERT INTO nation VALUES (1, 'GERMANY', 0, 'comment')")
        .unwrap();
    e.execute(
        "INSERT INTO part VALUES (1, 'green part', 'M#1', 'Brand#1', 'STANDARD', 1, \
         'SM CASE', 100.0, 'comment')",
    )
    .unwrap();
    e.execute("INSERT INTO supplier VALUES (100, 'S#100', 'addr', 1, 'phone', 100.0, 'c')")
        .unwrap();
    e.execute("INSERT INTO partsupp VALUES (1, 100, 100, 10.0, 'c')")
        .unwrap();
    e.execute(
        "INSERT INTO orders VALUES (1, 1, 'O', 100.0, '1995-03-01', '1-URGENT', \
         'Clerk#000000001', 0, 'comment')",
    )
    .unwrap();
    e.execute(
        "INSERT INTO lineitem VALUES (1, 1, 100, 1, 1.0, 100.0, 0.0, 0.0, 'N', 'O', \
         '1995-03-15', '1995-03-10', '1995-03-20', 'NONE', 'TRUCK', 'comment')",
    )
    .unwrap();
    e
}

/// Build the 7-way bridge fixture (Q8-shape: `r,n1,n2,c,o,l,s`).
///
/// `n1` and `n2` are both leaves, connected via customer and supplier
/// respectively to the central `l-o` chain.
fn build_7way_bridge_fixture() -> ExecutionEngine<MemoryStorage> {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, \
         r_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, \
         n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, \
         s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, \
         s_acctbal REAL NOT NULL, s_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, \
         c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, \
         c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, \
         o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, \
         o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, \
         o_comment TEXT)",
    )
    .unwrap();
    e.execute(
        "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
         l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, \
         l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, \
         l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, \
         l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, \
         l_shipmode TEXT NOT NULL, l_comment TEXT)",
    )
    .unwrap();

    e.execute("INSERT INTO region VALUES (1, 'AMERICA', 'comment')")
        .unwrap();
    e.execute("INSERT INTO nation VALUES (10, 'ARGENTINA', 1, 'comment')")
        .unwrap();
    e.execute("INSERT INTO nation VALUES (20, 'BRAZIL', 1, 'comment')")
        .unwrap();
    e.execute("INSERT INTO supplier VALUES (100, 'S#100', 'addr', 20, 'phone', 100.0, 'c')")
        .unwrap();
    e.execute(
        "INSERT INTO customer VALUES (1, 'C#1', 'addr', 10, 'phone', 100.0, 'BUILDING', 'c')",
    )
    .unwrap();
    e.execute(
        "INSERT INTO orders VALUES (1, 1, 'O', 100.0, '1995-03-01', '1-URGENT', \
         'Clerk#000000001', 0, 'comment')",
    )
    .unwrap();
    e.execute(
        "INSERT INTO lineitem VALUES (1, 1, 100, 1, 1.0, 100.0, 0.0, 0.0, 'N', 'O', \
         '1995-03-15', '1995-03-10', '1995-03-20', 'NONE', 'TRUCK', 'comment')",
    )
    .unwrap();
    e
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Q7-shape: star with two hubs joined by `l.l_orderkey = o.o_orderkey`.
/// Both `n1` and `n2` are leaves connected to `s` and `c` respectively.
/// The buggy single-direction greedy starting from the first FROM
/// table would walk `s -> l -> o -> c -> n2` and dead-end (missing
/// `n1`). With the multi-start + best-first fix, the chain starts
/// from `n2` (a leaf) and grows inward, completing the chain.
#[test]
fn test_6way_star_chain_build() {
    let mut engine = build_6way_star_fixture();

    // Pure comma-join (no explicit JOIN ... ON). The bug manifests
    // as `try_comma_join_hash_chain` returning None, in which case
    // `last_query_used_comma_join_fast_path()` would still be
    // `false`. With the fix, the chain completes and the flag is
    // `true`.
    let q = "SELECT COUNT(*) AS cnt FROM supplier, lineitem, orders, customer, \
             nation n1, nation n2 \
             WHERE s_suppkey = l_suppkey \
               AND l_orderkey = o_orderkey \
               AND o_custkey = c_custkey \
               AND s_nationkey = n1.n_nationkey \
               AND c_nationkey = n2.n_nationkey";

    let r = engine
        .execute(q)
        .expect("Q7-shape 6-way join should succeed");

    // Correctness: exactly one fully-joined row, COUNT(*) = 1.
    assert_eq!(r.rows.len(), 1, "expected exactly one joined row");
    assert_eq!(
        r.rows[0][0],
        sqlrustgo::Value::Integer(1),
        "expected COUNT(*) = 1 for fully-joined 6-way star"
    );

    // Strategy: assert the chain-build fast path was actually used.
    // This is the strict regression check: with the buggy code,
    // chain_order dead-ended and the flag is `false`.
    assert!(
        engine.last_query_used_comma_join_fast_path(),
        "try_comma_join_hash_chain should have built a complete chain \
         for the Q7-shape 6-way star (chain dead-end regression)"
    );
}

/// Q8-shape: bridge. `n1` and `n2` are both leaves connected via the
/// `c`/`s` hubs to the central `l-o` chain.
#[test]
fn test_7way_bridge_build() {
    let mut engine = build_7way_bridge_fixture();

    let q = "SELECT COUNT(*) AS cnt FROM region, supplier, lineitem, orders, customer, \
             nation n1, nation n2 \
             WHERE s_suppkey = l_suppkey \
               AND l_orderkey = o_orderkey \
               AND o_custkey = c_custkey \
               AND c_nationkey = n1.n_nationkey \
               AND n1.n_regionkey = r_regionkey \
               AND s_nationkey = n2.n_nationkey";

    let r = engine
        .execute(q)
        .expect("Q8-shape 7-way join should succeed");

    assert_eq!(r.rows.len(), 1, "expected exactly one joined row");
    assert_eq!(
        r.rows[0][0],
        sqlrustgo::Value::Integer(1),
        "expected COUNT(*) = 1 for fully-joined 7-way bridge"
    );

    assert!(
        engine.last_query_used_comma_join_fast_path(),
        "try_comma_join_hash_chain should have built a complete chain \
         for the Q8-shape 7-way bridge (chain dead-end regression)"
    );
}
