//! TPC-H 22 audit on **SF=0.01** canonical fixture (in-process).
//! Compare sqlrustgo row counts to SQLite ground truth.

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, RwLock};

const FIXTURE_DIR: &str = "/tmp/tpch_3way_sf001/clean";
const QUERIES_DIR: &str = "/home/ai/sqlrustgo/queries";

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
fn test_tpch_22_inprocess_sf001() {
    eprintln!("\n=== TPC-H 22 in-process audit (SF=0.01, AFTER Q9 O(1) hash fix) ===\n");
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    for ddl in SCHEMA_SQL {
        engine
            .execute(ddl)
            .unwrap_or_else(|e| panic!("DDL failed: {ddl} - {e}"));
    }
    eprintln!("  Schema created ({} tables)", SCHEMA_SQL.len());

    let base = PathBuf::from(FIXTURE_DIR);
    let mut total_rows = 0usize;
    for tbl in TABLES {
        let tbl_path = base.join(format!("{}.tbl", tbl));
        let content = fs::read_to_string(&tbl_path).expect("read");
        let columns = content
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim_end_matches('|').split('|').count())
            .unwrap_or(0);
        let n = load_tbl_file(&storage, tbl, &tbl_path, columns)
            .unwrap_or_else(|e| panic!("Failed to load {tbl}: {e}"));
        eprintln!("  {tbl}: {n} rows");
        total_rows += n;
    }
    eprintln!("  total: {total_rows} rows");

    let mut matched = 0;
    let mut mismatched = 0;
    let mut err_count = 0;
    for q in 1..=22 {
        let sql_path = format!("{}/q{}.sql", QUERIES_DIR, q);
        let sql = fs::read_to_string(&sql_path).unwrap();
        let sql = sql.trim().trim_end_matches(';');

        // SQLite ground truth
        let sqlite_sql = if q == 7 || q == 8 || q == 9 {
            sql.replace("EXTRACT(YEAR FROM ", "CAST(strftime('%Y', ")
                .replace(") AS o_year", ") AS INTEGER) AS o_year")
                .replace(") AS l_year", ") AS INTEGER) AS l_year")
        } else {
            sql.to_string()
        };
        let sqlite_out = Command::new("sqlite3")
            .args(["/tmp/tpch_3way_sf001.db", &format!("SELECT COUNT(*) FROM ({}) sub", sqlite_sql)])
            .output()
            .ok();
        let s_rc: Option<i64> = sqlite_out
            .as_ref()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok());

        // Engine result: wrap in COUNT(*) so we get the row count of
        // the result set, not a result column.
        let wrap = format!("SELECT COUNT(*) AS c FROM ({}) sub", sql);
        let engine_count: Option<i64> = match engine.execute(&wrap) {
            Ok(r) => r.rows.first()
                .and_then(|row| row.first())
                .and_then(|v| match v {
                    SqlValue::Integer(i) => Some(*i),
                    SqlValue::Float(f) => Some(*f as i64),
                    _ => None,
                }),
            Err(e) => {
                eprintln!("Engine ERR Q{q}: {e}");
                None
            }
        };

        let cat = match (engine_count, s_rc) {
            (Some(e), Some(s)) if e == s => { matched += 1; "MATCH" }
            (Some(_), Some(_)) => { mismatched += 1; "MISMATCH" }
            (None, _) => { err_count += 1; "ERR" }
            (Some(_), None) => { mismatched += 1; "MISMATCH (no baseline)" }
        };
        eprintln!("Q{:>2}: engine={:>6?} sqlite={:>6?}  {}", q, engine_count, s_rc, cat);
    }
    eprintln!("\n=== Totals ({} rows loaded) ===", total_rows);
    eprintln!("MATCHED   : {matched}/22");
    eprintln!("MISMATCHED: {mismatched}/22");
    eprintln!("ERR       : {err_count}/22");
    assert!(mismatched == 0 && err_count == 0,
        "TPC-H gate failure: {mismatched} mismatched, {err_count} errors");
}
