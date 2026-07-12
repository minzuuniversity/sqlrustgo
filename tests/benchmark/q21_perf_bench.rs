use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine, Value};
use std::sync::Arc;
use std::time::Instant;

type Engine = ExecutionEngine<MemoryStorage>;

const Q21_SQL: &str = include_str!("../queries/q21.sql");
const MAX_SECS: f64 = 5.0;
const BATCH_SIZE: usize = 2000;

fn fresh_engine() -> (Engine, Arc<RwLock<MemoryStorage>>) {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(Arc::clone(&storage));
    (engine, storage)
}

fn create_tables(engine: &mut Engine) {
    engine
        .execute(
            "\
CREATE TABLE region (\
  r_regionkey INTEGER, r_name TEXT, r_comment TEXT\
)",
        )
        .unwrap();
    engine
        .execute(
            "\
CREATE TABLE nation (\
  n_nationkey INTEGER, n_regionkey INTEGER, n_name TEXT, n_comment TEXT\
)",
        )
        .unwrap();
    engine
        .execute(
            "\
CREATE TABLE supplier (\
  s_suppkey INTEGER, s_nationkey INTEGER, s_name TEXT, \
  s_address TEXT, s_phone TEXT, s_acctbal REAL, s_comment TEXT\
)",
        )
        .unwrap();
    engine
        .execute(
            "\
CREATE TABLE customer (\
  c_custkey INTEGER, c_nationkey INTEGER, c_name TEXT, \
  c_address TEXT, c_phone TEXT, c_acctbal REAL, \
  c_mktsegment TEXT, c_comment TEXT\
)",
        )
        .unwrap();
    engine
        .execute(
            "\
CREATE TABLE part (\
  p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, \
  p_type TEXT, p_size INTEGER, p_container TEXT, \
  p_retailprice REAL, p_comment TEXT\
)",
        )
        .unwrap();
    engine
        .execute(
            "\
CREATE TABLE partsupp (\
  ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, \
  ps_supplycost REAL, ps_comment TEXT\
)",
        )
        .unwrap();
    engine
        .execute(
            "\
CREATE TABLE orders (\
  o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, \
  o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, \
  o_shippriority INTEGER, o_comment TEXT\
)",
        )
        .unwrap();
    engine
        .execute(
            "\
CREATE TABLE lineitem (\
  l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, \
  l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, \
  l_discount REAL, l_tax REAL, l_returnflag TEXT, \
  l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, \
  l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, \
  l_comment TEXT\
)",
        )
        .unwrap();
}

fn parse_tpch_value(s: &str) -> Value {
    let s = s.trim();
    if s.is_empty() || s == "null" || s == "NULL" {
        return Value::Null;
    }
    if let Ok(n) = s.parse::<i64>() {
        return Value::Integer(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Value::Float(f);
    }
    Value::Text(s.to_string())
}

fn load_tpch_table(
    storage: &Arc<RwLock<MemoryStorage>>,
    tbl: &str,
    ncols: usize,
    dir: &std::path::Path,
    totals: &mut Vec<(String, usize)>,
) {
    let path = dir.join(format!("{tbl}.tbl"));
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0usize;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let raw: Vec<&str> = line.split('|').collect();
        let mut row = Vec::with_capacity(ncols);
        for i in 0..ncols {
            let val = if i < raw.len() {
                parse_tpch_value(raw[i])
            } else {
                Value::Null
            };
            row.push(val);
        }
        batch.push(row);
        count += 1;
        if batch.len() >= BATCH_SIZE {
            let mut st = storage.write().unwrap();
            st.insert(tbl, batch).unwrap();
            batch = Vec::with_capacity(BATCH_SIZE);
        }
    }
    if !batch.is_empty() {
        let mut st = storage.write().unwrap();
        st.insert(tbl, batch).unwrap();
    }
    totals.push((tbl.to_string(), count));
}

fn load_tpch_sf01(storage: &Arc<RwLock<MemoryStorage>>, data_dir: &std::path::Path) {
    let tables: &[(&str, usize)] = &[
        ("region", 3),
        ("nation", 4),
        ("supplier", 7),
        ("customer", 8),
        ("part", 9),
        ("partsupp", 6),
        ("orders", 8),
        ("lineitem", 16),
    ];
    let mut totals = Vec::new();
    for (tbl, ncols) in tables {
        load_tpch_table(storage, tbl, *ncols, data_dir, &mut totals);
    }
    for (tbl, n) in &totals {
        eprintln!("  loaded {tbl}: {n} rows");
    }
}

#[test]
fn q21_perf_bench_sf01() {
    let data_dir = std::path::PathBuf::from("tests/data/tpch-sf01");
    if !data_dir.exists() {
        panic!("SF=0.1 fixture missing at {}", data_dir.display());
    }

    eprintln!("=== Creating tables and loading data ===");
    let (mut engine, storage) = fresh_engine();
    create_tables(&mut engine);

    let t0 = Instant::now();
    load_tpch_sf01(&storage, &data_dir);
    let load_secs = t0.elapsed().as_secs_f64();
    eprintln!("Data loaded in {:.3}s", load_secs);

    eprintln!("=== Running Q21 ===");
    let start = Instant::now();
    let result = engine
        .execute(Q21_SQL)
        .unwrap_or_else(|e| panic!("Q21 failed: {e}"));

    let elapsed = start.elapsed();
    let secs = elapsed.as_secs_f64();
    let rows = result.rows.len();

    eprintln!("Q21 completed in {:.3}s, returned {} rows", secs, rows);

    let q21_sf01_baseline_known_zero_rows: usize = 0;
    assert_eq!(
        rows, q21_sf01_baseline_known_zero_rows,
        "Q21 row count mismatch: expected {} got {}",
        q21_sf01_baseline_known_zero_rows, rows
    );

    assert!(
        secs < MAX_SECS,
        "Q21 perf regression: {:.3}s >= {:.1}s threshold",
        secs,
        MAX_SECS
    );

    eprintln!(
        "PASS: Q21 SF=0.1 {:.3}s < {:.1}s ({} rows)",
        secs, MAX_SECS, rows
    );
}
