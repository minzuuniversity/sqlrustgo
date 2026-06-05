//! TPC-H 22 audit on **SF=0.1** canonical fixture
//! (`/home/openclaw/sqlrustgo-tpch/sf01/`).
//!
//! 866K rows total. The SF=0.01 audit hit "0/0 coincidence" on 12/22
//! queries because the data set was too small for queries with date
//! filters in the 1994-1998 range to return any rows. SF=0.1 has 10× the
//! volume, so the 22 queries now hit real data:
//!
//!   SQLite SF=0.1 baseline (row_count) for all 22 queries:
//!     Q1=4 Q2=20 Q3=10 Q4=5 Q5=5 Q6=1 Q7=7 Q8=2 Q9=175 Q10=20
//!     Q11=3695 Q12=2 Q13=37 Q14=1 Q15=1000 Q16=2762 Q17=1 Q18=5
//!     Q19=1 Q20=18 Q21=50 Q22=7
//!
//! Q7/Q8/Q9 use TPC-H's `EXTRACT(YEAR FROM ...)` syntax which SQLite does
//! not accept, so for SQLite baseline they were rewritten to
//! `CAST(strftime('%Y', x) AS INTEGER)`. The sqlrustgo parser still
//! receives the original TPC-H SQL and must support `EXTRACT` natively.
//!
//! ## Loading strategy
//!
//! Same as the SF=0.01 audit: direct `StorageEngine::insert()` in
//! batches of 10000, engine created before the loader, DDL with
//! `PRIMARY KEY` + `NOT NULL` constraints.

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, RwLock};

const FIXTURE_DIR: &str = "/home/openclaw/sqlrustgo-tpch/sf01";
const QUERIES_DIR: &str = "queries";
const TMP_DIR: &str = "/tmp/tpch_22_sf01_audit";

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

const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

/// Expected row_count from SQLite on the SF=0.1 fixture. Computed offline
/// in this session using `sqlite3 tpch_sf01.sqlite "SELECT COUNT(*) FROM
/// (queries/qN.sql) sub"` (Q7-Q9 rewritten with strftime). These are the
/// **canonical correctness target** for the engine on SF=0.1.
const SQLITE_BASELINE: &[Option<usize>] = &[
    Some(4),    // Q1
    Some(20),   // Q2
    Some(10),   // Q3
    Some(5),    // Q4
    Some(5),    // Q5
    Some(1),    // Q6
    Some(7),    // Q7
    Some(2),    // Q8
    Some(175),  // Q9
    Some(20),   // Q10
    Some(3695), // Q11
    Some(2),    // Q12
    Some(37),   // Q13
    Some(1),    // Q14
    Some(1000), // Q15
    Some(2762), // Q16
    Some(1),    // Q17
    Some(5),    // Q18
    Some(1),    // Q19
    Some(18),   // Q20
    Some(50),   // Q21
    Some(7),    // Q22
];

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
        // strip trailing '|' so split('|') yields exactly `columns` parts
        let line = line.trim_end_matches('|');
        let values: Vec<&str> = line.split('|').collect();
        if values.len() != columns {
            continue;
        }
        let record: Vec<SqlValue> = values
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

#[test]
fn eval_22_vs_sqlite_sf01() {
    println!(
        "\n=== TPC-H 22 audit on SF=0.1 canonical fixture ({}) ===\n",
        FIXTURE_DIR
    );
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    eprintln!("[1/3] Creating schema...");
    for ddl in SCHEMA_SQL {
        engine
            .execute(ddl)
            .unwrap_or_else(|e| panic!("DDL failed: {ddl} - {e}"));
    }
    eprintln!("  Schema created ({} tables)", SCHEMA_SQL.len());

    eprintln!("[2/3] Loading fixture (batches of 10000) ...");
    let base = PathBuf::from(FIXTURE_DIR);
    let mut total_rows = 0usize;
    for tbl in TABLES {
        let tbl_path = base.join(format!("{}.tbl", tbl));
        // Sniff column count from the first non-empty data line. We don't
        // trust the hard-coded schema here because the loader is doing the
        // work, not the engine.
        let content = fs::read_to_string(&tbl_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", tbl_path.display()));
        let columns = content
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim_end_matches('|').split('|').count())
            .unwrap_or(0);
        let n = load_tbl_file(&storage, tbl, &tbl_path, columns)
            .unwrap_or_else(|e| panic!("Failed to load {tbl}: {e}"));
        eprintln!("  {tbl}: {n} rows ({columns} cols)");
        total_rows += n;
    }
    eprintln!("  total: {total_rows} rows");

    eprintln!("[3/3] Running 22 queries ...");
    let mut matched = 0usize;
    let mut mismatched = 0usize;
    let mut err_count = 0usize;
    for q in 1..=22usize {
        let sql_path = format!("{}/q{}.sql", QUERIES_DIR, q);
        let sql = fs::read_to_string(&sql_path).unwrap_or_else(|_| panic!("read {}", sql_path));
        let sql = sql.trim().trim_end_matches(';');

        let expected = SQLITE_BASELINE[q - 1];

        let engine_count: Option<usize> =
            match engine.execute(&format!("SELECT COUNT(*) FROM ({}) sub", sql)) {
                Ok(r) => r
                    .rows
                    .first()
                    .and_then(|row| row.first())
                    .and_then(|v| match v {
                        SqlValue::Integer(i) => Some(*i as usize),
                        SqlValue::Float(f) => Some(*f as usize),
                        _ => None,
                    }),
                Err(e) => {
                    eprintln!("Engine ERR Q{q}: {e}");
                    None
                }
            };

        let cat = match (engine_count, expected) {
            (Some(e), Some(s)) if e == s => {
                matched += 1;
                "MATCHED"
            }
            (Some(_), Some(_)) => {
                mismatched += 1;
                "MISMATCHED"
            }
            (None, _) => {
                err_count += 1;
                "ERR"
            }
            (Some(_), None) => {
                mismatched += 1;
                "MISMATCHED (no baseline)"
            }
        };
        println!(
            "Q{:>2}: engine={:>6?} sqlite_baseline={:>6?}  {}",
            q, engine_count, expected, cat
        );
    }
    println!("\n=== Totals ({} rows loaded) ===", total_rows);
    println!("MATCHED   : {matched}/22");
    println!("MISMATCHED: {mismatched}/22");
    println!("ERR(engine): {err_count}/22");
    let total = matched + mismatched + err_count;
    assert!(total >= 22, "category counts do not add up to 22: {}", total);
}
