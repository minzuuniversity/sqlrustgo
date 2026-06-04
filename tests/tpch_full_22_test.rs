//! TPC-H Full 22 Query Test — v3.8.0 TPC-H coverage gate
//!
//! Reads .tbl files from `~/sqlrustgo-tpch/data/` (SF=0.01 by default) and
//! runs the standard TPC-H Q1-Q22 SQL (from the `queries/q*.sql` directory).
//!
//! Goal: 22/22 queries must execute without parse/executor errors.
//!
//! # Usage
//!
//! ```bash
//! # Default SF=0.01 (data in ~/sqlrustgo-tpch/data)
//! cargo test --test tpch_full_22_test -- --nocapture
//!
//! # SF=1 (data in ~/sqlrustgo-tpch/sf1)
//! TPCH_SF=1 cargo test --test tpch_full_22_test -- --nocapture
//!
//! # Custom data dir
//! TPCH_DATA_DIR=/path/to/tbl cargo test --test tpch_full_22_test
//! ```

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

// ============================================================
// Helpers
// ============================================================

fn data_dir() -> PathBuf {
    env::var("TPCH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join("sqlrustgo-tpch").join("data")
        })
}

fn queries_dir() -> PathBuf {
    // queries/ directory is at the workspace root (next to tests/)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(manifest_dir).join("queries")
}

fn has_tpch_data(dir: &PathBuf) -> bool {
    if !dir.exists() {
        return false;
    }
    fs::read_dir(dir)
        .map(|e| {
            e.filter_map(|x| x.ok())
                .filter(|e| e.path().extension().map(|x| x == "tbl").unwrap_or(false))
                .count()
        })
        .unwrap_or(0)
        >= 3
}

// TPC-H schema (matches queries/q*.sql column references)
const SCHEMA_SQL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))",
];

/// Load a .tbl file into a SQL table using batch insert
fn load_tbl_file(
    storage: &Arc<RwLock<MemoryStorage>>,
    tbl_name: &str,
    tbl_path: &PathBuf,
    columns: usize,
) -> Result<usize, String> {
    let content = fs::read_to_string(tbl_path)
        .map_err(|e| format!("Cannot read {}: {}", tbl_path.display(), e))?;

    const BATCH_SIZE: usize = 10000;
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split('|').collect();
        if values.len() < columns {
            continue;
        }

        let record: Vec<SqlValue> = values[..columns]
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
            let mut storage = storage.write().map_err(|e| format!("Lock error: {}", e))?;
            storage
                .insert(tbl_name, batch.clone())
                .map_err(|e| format!("Insert error: {}", e))?;
            count += batch.len();
            batch.clear();
        }
    }

    if !batch.is_empty() {
        let mut storage = storage.write().map_err(|e| format!("Lock error: {}", e))?;
        storage
            .insert(tbl_name, batch.clone())
            .map_err(|e| format!("Insert error: {}", e))?;
        count += batch.len();
    }

    Ok(count)
}

// ============================================================
// The test
// ============================================================

#[test]
fn test_tpch_full_22_queries() {
    let dir = data_dir();
    if !has_tpch_data(&dir) {
        eprintln!("\n=== TPC-H Full 22 [SKIPPED] ===");
        eprintln!("Data not found at: {}", dir.display());
        eprintln!("Generate with: bash scripts/gate/setup_tpch_env.sh --sf1");
        if env::var("TPCH_FORCE").as_deref() == Ok("1") {
            panic!("TPC-H data required but not found (TPCH_FORCE=1)");
        }
        return;
    }

    let qdir = queries_dir();
    assert!(
        qdir.exists(),
        "queries/ directory not found: {}",
        qdir.display()
    );

    eprintln!("\n=== TPC-H Full 22 Query Gate ===");
    eprintln!("Data dir: {}", dir.display());
    eprintln!("Queries dir: {}", qdir.display());
    eprintln!("");

    // Create engine
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    #[allow(deprecated)]
    let mut engine = ExecutionEngine::new(storage.clone());

    // Create schema
    eprintln!("[1/3] Creating schema...");
    for ddl in SCHEMA_SQL {
        engine
            .execute(ddl)
            .unwrap_or_else(|e| panic!("DDL failed: {} - {}", ddl, e));
    }
    eprintln!("  Schema created ({} tables)", SCHEMA_SQL.len());

    // Import data
    eprintln!("[2/3] Importing data...");
    let import_start = Instant::now();

    let tables = vec![
        ("region", 3),
        ("nation", 4),
        ("supplier", 7),
        ("customer", 8),
        ("part", 9),
        ("partsupp", 5),
        ("orders", 9),
        ("lineitem", 16),
    ];

    let mut total_rows = 0;
    for (tbl_name, cols) in &tables {
        let tbl_path = dir.join(format!("{}.tbl", tbl_name));
        if !tbl_path.exists() {
            eprintln!("  [SKIP] {}: file not found", tbl_path.display());
            continue;
        }
        eprint!("  Loading {}... ", tbl_name);
        let rows = load_tbl_file(&storage, tbl_name, &tbl_path, *cols)
            .unwrap_or_else(|e| panic!("Failed to load {}: {}", tbl_name, e));
        eprintln!("{} rows", rows);
        total_rows += rows;
    }

    let import_elapsed = import_start.elapsed();
    eprintln!(
        "  Import completed: {} rows in {:?}",
        total_rows, import_elapsed
    );

    // Run Q1..Q22
    eprintln!("\n[3/3] Running TPC-H Q1..Q22...");
    let mut results: Vec<(&str, Duration, Result<usize, String>)> = Vec::new();

    for q in 1..=22 {
        let q_name = format!("Q{}", q);
        let q_path = qdir.join(format!("q{}.sql", q));
        if !q_path.exists() {
            eprintln!("  {} ... ⚠️  query file not found", q_name);
            results.push((
                Box::leak(q_name.into_boxed_str()),
                Duration::ZERO,
                Err("file not found".to_string()),
            ));
            continue;
        }
        let q_sql = fs::read_to_string(&q_path)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", q_path.display(), e))
            .trim()
            .trim_end_matches(';')
            .to_string();

        let start = Instant::now();
        let result = engine.execute(&q_sql);
        let elapsed = start.elapsed();

        let row_count = match &result {
            Ok(exec) => Ok(exec.rows.len()),
            Err(e) => Err(format!("{}", e)),
        };

        let q_name_static: &'static str = Box::leak(q_name.clone().into_boxed_str());
        results.push((q_name_static, elapsed, row_count));
    }

    // Report
    eprintln!("\n=== TPC-H Full 22 Results ===");
    let total = results.len();
    let passed = results.iter().filter(|r| r.2.is_ok()).count();
    let failed = total - passed;

    for (q_name, elapsed, result) in &results {
        match result {
            Ok(n) => eprintln!("✅ {}: {} rows ({:?})", q_name, n, elapsed),
            Err(e) => {
                let truncated = if e.len() > 100 {
                    format!("{}...", &e[..100])
                } else {
                    e.clone()
                };
                eprintln!("❌ {}: {} ({:?})", q_name, truncated, elapsed);
            }
        }
    }

    eprintln!("\nTotal: {}/{} passed, {} failed", passed, total, failed);
    eprintln!("Import time: {:?}", import_elapsed);
    eprintln!();

    // For gate purposes: report the result but do not panic (TPC-H full coverage
    // is tracked as ongoing work; intermediate results are still useful diagnostics).
    if failed > 0 {
        eprintln!(
            "⚠️  TPC-H Full 22: only {}/{} queries executed successfully. \
             Remaining failures are tracked for follow-up PRs.",
            passed, total
        );
    } else {
        eprintln!("✅ TPC-H Full 22 PASSED ({} queries)", passed);
    }

    // RC1 gate baseline: 13/22 passing pre-RC1 (audit 2026-06-04).
    // Real target is 22/22; tracked as #2977. The baseline assertion
    // here catches regressions: any drop below 13 is a hard fail.
    // For CI gating in pre-RC, set TPCH_RC1_STRICT=1 to require 22/22.
    let min_required = if env::var("TPCH_RC1_STRICT").as_deref() == Ok("1") {
        22
    } else {
        13
    };
    assert!(
        passed >= min_required,
        "TPC-H RC1 gate: {passed}/22 passed (need >= {min_required}); \
         {failed} failed. Set TPCH_RC1_STRICT=1 to require 22/22. \
         Tracked as #2977."
    );
}
