//! Regression test for V312-48 issue #4278: canonical TPC-H Q16 NOT IN subquery
//! returns the wrong row count due to a thread-local flag leak between the
//! outer comma-join query and its non-correlated NOT IN subquery.
//!
//! Root cause (PR #4357+followup): the hash-chain fast path in
//! `try_comma_join_hash_chain` set a thread-local
//! `COMMA_JOIN_WHERE_CONSUMED=true` flag. When step 1.6 of the WHERE pipeline
//! (non-correlated IN/NOT IN rewrite) recursively called `execute_select` for
//! the subquery, the subquery inherited the parent's `skip_where=true` and
//! skipped its own `WHERE s_comment LIKE '%Customer%Complaints%'` filter.
//! This produced a `NotInList` of all 10 000 supplier keys, which eliminated
//! every partsupp row → 0 results.
//!
//! Fix: reset `COMMA_JOIN_WHERE_CONSUMED` to `false` at the start of every
//! `execute_select` call so each invocation's WHERE handling is independent.
//!
//! Expected baseline (SQLite, sf1): 18 314 rows.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::sync::Arc;

// The shared TPC-H fixture resolver (issue #5379).
//
// The fixture used to be read from the absolute path
// `/home/openclaw/sqlrustgo_work/tests/data/tpch-sf01`, i.e. one author's
// home directory — it never existed on any CI runner. The fixture is now
// resolved through the shared resolver, and a missing fixture (or a fixture
// at a different scale profile) is disclosed as NOT-RUN instead of panicking.
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

/// Cargo test target this file belongs to (used as the NOT-RUN label).
const TARGET: &str = "q16_notin_subquery_regression";

/// The scale profile this test's expectations were calibrated against.
const EXPECTED_SF: f64 = 0.1;

fn load(
    storage: &Arc<RwLock<MemoryStorage>>,
    fixture_dir: &std::path::Path,
    tbl: &str,
    ncols: usize,
) -> Result<usize, String> {
    let path = fixture_dir.join(format!("{tbl}.tbl"));
    let content = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    const BATCH_SIZE: usize = 10000;
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split('|').collect();
        if values.len() < ncols {
            continue;
        }
        let record: Vec<SqlValue> = values[..ncols]
            .iter()
            .map(|v| {
                let s = v.trim();
                if s.is_empty() {
                    SqlValue::Null
                } else if let Ok(i) = s.parse::<i64>() {
                    SqlValue::Integer(i)
                } else if let Ok(f) = s.parse::<f64>() {
                    SqlValue::Float(f)
                } else {
                    SqlValue::Text(s.to_string())
                }
            })
            .collect();
        batch.push(record);
        if batch.len() >= BATCH_SIZE {
            let mut s = storage.write();
            // The insert error must not be swallowed: a failed batch insert
            // leaves the table empty and the assertions below would then run
            // against an empty table and pass vacuously.
            s.insert(tbl, batch.clone())
                .map_err(|e| format!("insert into `{tbl}`: {e}"))?;
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut s = storage.write();
        s.insert(tbl, batch.clone())
            .map_err(|e| format!("insert into `{tbl}`: {e}"))?;
        count += batch.len();
    }
    Ok(count)
}

/// Resolve the TPC-H fixture for this target, or disclose NOT-RUN and return
/// `None` (a missing fixture, or a fixture whose scale profile is not the
/// SF=0.1 profile these expectations were calibrated against).
fn resolve_fixture() -> Option<std::path::PathBuf> {
    let state = tpch_fixture::FixtureState::resolve();
    let fixture = state.require_profile(TARGET, EXPECTED_SF)?;
    Some(fixture.dir.clone())
}

#[test]
fn q16_canonical_subquery_only() {
    let Some(fixture_dir) = resolve_fixture() else {
        return;
    };

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)").unwrap();
    if let Err(e) = load(&storage, &fixture_dir, "supplier", 7) {
        tpch_fixture::report_not_run(TARGET, tpch_fixture::reason::FIXTURE_MISSING, &e);
        return;
    }

    let r = engine
        .execute("SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%Customer%Complaints%' ORDER BY s_suppkey")
        .unwrap();
    assert!(
        r.rows.len() >= 4,
        "expected >=4 Customer+Complaints matches, got {}",
        r.rows.len()
    );
}

#[test]
fn q16_canonical_notin_full() {
    let Some(fixture_dir) = resolve_fixture() else {
        return;
    };

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))").unwrap();
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)").unwrap();
    for tbl in ["part", "partsupp", "supplier"] {
        let ncols = match tbl {
            "part" => 9,
            "partsupp" => 5,
            _ => 7,
        };
        if let Err(e) = load(&storage, &fixture_dir, tbl, ncols) {
            tpch_fixture::report_not_run(TARGET, tpch_fixture::reason::FIXTURE_MISSING, &e);
            return;
        }
    }

    let sql = "SELECT p_brand, p_type, p_size, COUNT(DISTINCT ps_suppkey) AS supplier_cnt \
               FROM partsupp, part \
               WHERE p_partkey = ps_partkey \
                 AND p_brand <> 'Brand#45' \
                 AND p_type NOT LIKE 'MEDIUM POLISHED%' \
                 AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9) \
                 AND ps_suppkey NOT IN (\
                   SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%Customer%Complaints%'\
                 ) \
               GROUP BY p_brand, p_type, p_size \
               ORDER BY supplier_cnt DESC, p_brand, p_type, p_size";
    let r = engine.execute(sql).unwrap();
    // SQLite sf1 baseline: 18314 rows.
    assert_eq!(
        r.rows.len(),
        18314,
        "Q16 row count must match SQLite baseline (18314)"
    );
}
