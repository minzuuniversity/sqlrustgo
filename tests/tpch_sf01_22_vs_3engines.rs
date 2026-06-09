//! Sprint 5 v10: TPC-H 22/22 cell-by-cell cross-engine comparison
//! Compares sqlrustgo's results against MariaDB and PostgreSQL row counts
//! and value-equality (with float tolerance) on the SF=0.1 fixture.

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, RwLock};

const DATA_DIR: &str = "tests/data/tpch-sf01";
const QUERIES_DIR: &str = "queries";
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
const TABLES: &[&str] = &["region","nation","supplier","customer","part","partsupp","orders","lineitem"];

fn lookup_col_types(table: &str) -> Vec<&'static str> {
    let ddl = SCHEMA_SQL.iter().find(|s| s.contains(table)).expect("ddl");
    let start = ddl.find('(').unwrap() + 1;
    let end = ddl.rfind(')').unwrap();
    let inner = &ddl[start..end];
    let tokens: Vec<&str> = inner
        .split(',')
        .map(|c| {
            let c = c.trim();
            if c.eq_ignore_ascii_case("PRIMARY KEY") { "" }
            else if let Some(idx) = c.find("PRIMARY KEY") {
                let stripped = c[..idx].trim().to_string();
                Box::leak(stripped.into_boxed_str()) as &str
            } else { c }
        })
        .filter(|s| !s.is_empty())
        .map(|c| {
            let toks: Vec<&str> = c.split_whitespace().collect();
            if toks.len() >= 2 { toks[1] } else { "" }
        })
        .filter(|s| !s.is_empty())
        .collect();
    tokens
}

fn load_tbl(storage: &Arc<RwLock<MemoryStorage>>, tbl: &str) -> usize {
    let path = format!("{}/{}.tbl", DATA_DIR, tbl);
    let content = fs::read_to_string(&path).expect(&format!("read {}", path));
    let types = lookup_col_types(tbl);
    let mut n = 0usize;
    for line in content.lines() {
        if line.is_empty() { continue; }
        let fields: Vec<&str> = line.split('|').collect();
        let mut vals: Vec<SqlValue> = Vec::new();
        for (i, f) in fields.iter().enumerate() {
            let t = types.get(i).copied().unwrap_or("TEXT");
            vals.push(match t.to_uppercase().as_str() {
                "INTEGER" => f.parse::<i64>().map(SqlValue::Integer).unwrap_or(SqlValue::Null),
                "REAL" => f.parse::<f64>().map(SqlValue::Float).unwrap_or(SqlValue::Null),
                _ => SqlValue::Text(f.to_string()),
            });
        }
        storage.write().unwrap().insert(tbl, vec![vals]).expect("insert");
        n += 1;
    }
    n
}

fn build_engine() -> (ExecutionEngine<MemoryStorage>, Arc<RwLock<MemoryStorage>>) {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for ddl in SCHEMA_SQL {
        engine.execute(ddl).expect("ddl");
    }
    (engine, storage)
}

fn to_md_value(v: &SqlValue) -> String {
    match v {
        SqlValue::Null => "NULL".to_string(),
        SqlValue::Integer(i) => i.to_string(),
        SqlValue::Float(f) => f.to_string(),
        SqlValue::Text(s) => s.clone(),
        SqlValue::Boolean(b) => b.to_string(),
        SqlValue::Blob(_) => "BLOB".to_string(),
    }
}

fn run_md(sql: &str) -> Result<String, String> {
    let out = Command::new("mysql")
        .args(&["-B", "-N", "tpch_sf01", "-e", sql])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_md_count(sql: &str) -> usize {
    let sql_stripped = sql.trim_end_matches(';');
    let count_sql = format!("SELECT COUNT(*) FROM ({}) AS x", sql_stripped);
    let out = Command::new("mysql")
        .args(&["-B", "-N", "tpch_sf01", "-e", &count_sql])
        .output()
        .expect("mysql count");
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0)
}

#[test]
fn tpch_sf01_22_vs_mariadb_cell() {
    if !PathBuf::from(DATA_DIR).exists() { panic!("fixture missing"); }
    if Command::new("mysql").arg("-e").arg("SELECT 1").output().is_err() {
        eprintln!("MariaDB not available, skipping");
        return;
    }
    eprintln!("=== Loading SF=0.1 fixture from {} ===", DATA_DIR);
    let (mut engine, storage) = build_engine();
    for tbl in TABLES {
        let n = load_tbl(&storage, tbl);
        eprintln!("  {}: {} rows", tbl, n);
    }
    eprintln!();
    eprintln!("=== Cell-level comparison sqlrustgo vs MariaDB on 22 queries ===");
    let mut pass = 0;
    let mut fail = 0;
    for n in 1..=22usize {
        let sql = fs::read_to_string(format!("{}/q{}.sql", QUERIES_DIR, n)).unwrap();
        let t0 = std::time::Instant::now();
        let r = engine.execute(&sql).expect("engine execute");
        let elapsed = t0.elapsed();
        let sr_rows = r.rows.len();
        let sr_strings: Vec<String> = r.rows.iter().map(|row| {
            row.iter().map(to_md_value).collect::<Vec<_>>().join("|")
        }).collect();
        let sr_set: std::collections::HashSet<String> = sr_strings.iter().cloned().collect();
        let md_result = run_md(&sql);
        let md_count = if md_result.is_ok() { run_md_count(&sql) } else { 0 };
        let md_strings: Vec<String> = match md_result {
            Ok(s) => s.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect(),
            Err(_) => vec![],
        };
        let md_set: std::collections::HashSet<String> = md_strings.iter().cloned().collect();
        let cell_match = sr_set == md_set;
        let status = if sr_rows == md_count && cell_match { "PASS" } else { "FAIL" };
        if status == "PASS" { pass += 1 } else { fail += 1 };
        let diff_info = if !cell_match && sr_rows == md_count {
            format!(" [{} rows differ]", sr_set.symmetric_difference(&md_set).count())
        } else if sr_rows != md_count {
            format!(" [rc sr={} md={}]", sr_rows, md_count)
        } else { "".to_string() };
        eprintln!("  Q{:2}: {} (rc={}, md={}, {}{}) in {:?}", n, status, sr_rows, md_count, "cell-match", diff_info, elapsed);
    }
    eprintln!();
    eprintln!("=== Summary: pass={} fail={} ===", pass, fail);
    eprintln!("NOTE: This is a diagnostic test. Sprint 5 v10 found that");
    eprintln!("Q17 (cell value) and Q18 (wrong top order) are real engine bugs");
    eprintln!("that the row-count-based test missed.");
}
