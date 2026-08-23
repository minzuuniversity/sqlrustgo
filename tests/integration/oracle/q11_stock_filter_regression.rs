//! Regression test for V312-58 / Issue #4377: TPC-H Q11 stock-level filter.
//!
//! Q11 joins `partsupp, supplier, nation`, filters `n_name = 'GERMANY'`,
//! groups by `ps_partkey`, applies `HAVING SUM(ps_supplycost*ps_availqty) > 10000`,
//! and orders by the aggregated sum descending.
//!
//! Pre-#4410, the comma-join hash-chain build resolved single-table
//! predicates by table alias first, then bare table name. Q11 references
//! `n_name` without an alias (`FROM partsupp, supplier, nation`), so the
//! `n_name = 'GERMANY'` predicate was silently dropped during the build
//! and `COMMA_JOIN_WHERE_CONSUMED` was set on the post-join WHERE.
//! Result: the full 800K partsupp × 25 nation cross-product leaked into
//! GROUP BY, producing 200,000 distinct ps_partkey groups instead of the
// SQLite-oracle ~29,636 (the count of GERMANY-only parts with stock
//! value > 10000).
//!
//! Fix commit: 0e9e32a4d (fix(v312-58 / #4377): TPC-H Q11 pushdown predicate —
//!              3rd lookup by tpch column prefix)
//!
//! Verification (post-#4410):
//!   row_count          = 29,636    ✅
//!   partkey SET        = identical ✅
//!   per-partkey value  = identical ✅
//!   ordering           = 4 of 29,636 are ties (SQL standard allows arbitrary
//!                        tie-breaking; the divergent pairs have equal sums)
//!
//! Run:
//!   cargo test --test q11_stock_filter_regression --all-features \
//!     -- --ignored --nocapture q11_stock_filter_sf1

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::collections::BTreeMap;
use std::sync::Arc;

/// SF=1 fixture directory. Override with TPCH_SF1_DIR env var.
const DATA_DIR: &str = "/tmp/tpch-sf1";

const SCHEMAS: &[&str] = &[
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
];

const TBL_FILES: &[&str] = &["nation", "supplier", "partsupp"];

fn resolve_data_dir() -> String {
    std::env::var("TPCH_SF1_DIR").unwrap_or_else(|_| DATA_DIR.to_string())
}

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    let data_dir = resolve_data_dir();
    for s in SCHEMAS {
        engine.execute(s).unwrap();
    }
    {
        let mut st = storage.write();
        for t in TBL_FILES {
            let path = format!("{}/{}.tbl", data_dir, t);
            st.bulk_load_tbl_file(t, &path).unwrap();
        }
    }
    engine
}

/// Q11 canonical SQL — copy of `queries/q11.sql`.
const Q11_SQL: &str = "SELECT ps_partkey, SUM(ps_supplycost * ps_availqty) AS part_value \
                       FROM partsupp, supplier, nation \
                       WHERE ps_suppkey = s_suppkey \
                         AND s_nationkey = n_nationkey \
                         AND n_name = 'GERMANY' \
                       GROUP BY ps_partkey \
                       HAVING SUM(ps_supplycost * ps_availqty) > 10000 \
                       ORDER BY part_value DESC";

/// Expected row count from the SQLite oracle at SF=1 (queries/q11.sql).
const EXPECTED_ROW_COUNT: usize = 29_636;

/// Absolute path (relative to repo root) to the SQLite oracle TSV file.
const ORACLE_PATH: &str =
    "docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/sqlite/q11.tsv";

/// Float comparison tolerance — IEEE-754 f64 summation order may differ
/// slightly across engines (Rust row-by-row sum vs SQLite's per-group sum)
/// but TPC-H 2.18.0 §6.3.3 allows ±1e-3 epsilon for aggregate queries.
const FLOAT_TOL: f64 = 1e-3;

/// Maximum number of ordering tie-break mismatches allowed. SQL standard
/// permits arbitrary tie-breaking for equal `ORDER BY` keys; we observed
/// 4 such ties in the SF=1 oracle (2 pairs of parts with identical stock
/// value). We allow up to 100 to leave headroom for future data
/// regenerations while still flagging gross ordering bugs.
const MAX_TIE_MISMATCHES: usize = 100;

#[test]
#[ignore] // Heavy: SF=1 partsupp has 800K rows
fn q11_stock_filter_sf1() {
    let mut engine = setup();
    let r = engine
        .execute(Q11_SQL)
        .unwrap_or_else(|e| panic!("Q11 failed: {}", e));
    eprintln!("Q11 returned {} rows", r.rows.len());
    for (i, row) in r.rows.iter().take(3).enumerate() {
        eprintln!("  row[{}] = {:?}", i, row);
    }
    if let Some(row) = r.rows.last() {
        eprintln!("  last row = {:?}", row);
    }

    // Parse engine output into a (partkey → value) map.
    let engine_map: BTreeMap<i64, f64> = r
        .rows
        .iter()
        .map(|row| {
            let k = match &row[0] {
                sqlrustgo::Value::Integer(i) => *i,
                other => panic!("unexpected partkey type: {:?}", other),
            };
            let v = match &row[1] {
                sqlrustgo::Value::Float(f) => *f,
                other => panic!("unexpected part_value type: {:?}", other),
            };
            (k, v)
        })
        .collect();

    // Load SQLite oracle.
    let oracle_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ORACLE_PATH);
    let oracle_text = std::fs::read_to_string(&oracle_path)
        .unwrap_or_else(|e| panic!("missing oracle {}: {}", oracle_path.display(), e));
    let oracle_map: BTreeMap<i64, f64> = oracle_text
        .lines()
        .map(|l| {
            let mut parts = l.split('\t');
            (
                parts.next().unwrap().parse::<i64>().unwrap(),
                parts.next().unwrap().parse::<f64>().unwrap(),
            )
        })
        .collect();

    // 1) Row-count parity — the regression symptom (#4377) was
    //    200,000 rows instead of 29,636.
    assert_eq!(
        r.rows.len(),
        EXPECTED_ROW_COUNT,
        "Q11 row count must match SQLite oracle (expected {}, got {})",
        EXPECTED_ROW_COUNT,
        r.rows.len()
    );
    assert_eq!(
        engine_map.len(),
        oracle_map.len(),
        "engine map size {} != oracle map size {}",
        engine_map.len(),
        oracle_map.len()
    );

    // 2) Partkey SET identity — same 29,636 parts on both sides.
    let engine_keys: std::collections::BTreeSet<i64> = engine_map.keys().copied().collect();
    let oracle_keys: std::collections::BTreeSet<i64> = oracle_map.keys().copied().collect();
    assert_eq!(
        engine_keys, oracle_keys,
        "Q11 partkey SET must match SQLite oracle"
    );

    // 3) Per-partkey value parity within ±epsilon — verify the actual
    //    aggregates, not just the count.
    let mut max_abs_diff = 0f64;
    let mut max_diff_partkey = 0i64;
    for (k, engine_v) in &engine_map {
        let oracle_v = oracle_map.get(k).unwrap();
        let diff = (engine_v - oracle_v).abs();
        if diff > max_abs_diff {
            max_abs_diff = diff;
            max_diff_partkey = *k;
        }
        assert!(
            diff <= FLOAT_TOL,
            "Q11 aggregate mismatch for partkey {}: engine={}, oracle={}, diff={}",
            k, engine_v, oracle_v, diff
        );
    }
    eprintln!(
        "max |engine - oracle| value diff: {} (at partkey {})",
        max_abs_diff, max_diff_partkey
    );

    // 4) Ordering check — sort oracle by value DESC and compare. SQL
    //    standard allows arbitrary tie-breaking; we tolerate up to
    //    MAX_TIE_MISMATCHES equal-value reorderings.
    let mut oracle_sorted: Vec<(i64, f64)> = oracle_map.iter().map(|(k, v)| (*k, *v)).collect();
    oracle_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Engine rows are already ordered (SQL specifies ORDER BY part_value DESC).
    let engine_ordered: Vec<(i64, f64)> = r
        .rows
        .iter()
        .map(|row| {
            let k = match &row[0] {
                sqlrustgo::Value::Integer(i) => *i,
                _ => unreachable!(),
            };
            let v = match &row[1] {
                sqlrustgo::Value::Float(f) => *f,
                _ => unreachable!(),
            };
            (k, v)
        })
        .collect();

    let mut tie_mismatches = 0usize;
    for (i, ((ek, ev), (ok, ov))) in
        engine_ordered.iter().zip(oracle_sorted.iter()).enumerate()
    {
        if ek != ok {
            // Equal-valued tie reordering is allowed; different value is not.
            assert!(
                (ev - ov).abs() <= FLOAT_TOL,
                "Q11 ordering mismatch at row {} (not a tie): \
                 engine=(partkey={}, value={}), oracle=(partkey={}, value={})",
                i, ek, ev, ok, ov
            );
            tie_mismatches += 1;
        }
    }
    eprintln!(
        "ordering tie-break mismatches (equal values, different partkey): {} / {}",
        tie_mismatches, engine_ordered.len()
    );
    assert!(
        tie_mismatches <= MAX_TIE_MISMATCHES,
        "Q11 ordering has {} tie-break mismatches (limit {}) — possible \
         non-stable sort bug",
        tie_mismatches, MAX_TIE_MISMATCHES
    );
}