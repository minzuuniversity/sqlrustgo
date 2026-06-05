//! TPC-H 22 audit on canonical SF=0.01 fixture
//! (`/home/openclaw/sqlrustgo-tpch/data/`).
//!
//! **Replaces** the previous `tests/eval_22_vs_sqlite.rs` audit which used
//! the corrupt `tests/data/tpch-sf001/*.tbl` (15/614/150 rows). See
//! `docs/discovery/2026-06-05-tpch-22-sf001-corrupt.md` for why that fixture
//! was wrong.
//!
//! ## What this measures
//!
//! For each of the 22 standard TPC-H queries (from `queries/q*.sql`),
//! - Run the same SQL against an in-process sqlrustgo `ExecutionEngine`
//!   with the SF=0.01 fixture loaded.
//! - Run the same SQL against a vanilla SQLite database with the same fixture
//!   loaded.
//! - Compare `row_count` between the two.
//! - Report the result category: MATCHED / MISMATCHED / ERR (engine error).
//!
//! ## What to expect
//!
//! Macmini's PR #3124 reported 6/22 PASS on this fixture via their
//! `tpch_value_test_v2` gate. Their "6/22" used a slightly different test
//! (per-row value comparison, not just row_count) so the row_count number may
//! be higher. The real baseline will be produced by this test.

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const FIXTURE_DIR: &str = "/home/openclaw/sqlrustgo-tpch/data";
const QUERIES_DIR: &str = "queries";
const TMP_DIR: &str = "/tmp/tpch_22_sf01_audit";

/// DDL matches the **standard TPC-H column order** in the canonical
/// `/home/openclaw/sqlrustgo-tpch/data/*.tbl` files (not the sqlrustgo
/// internal order used elsewhere in the engine). When the engine has a
/// canonical loader, this DDL should be replaced with the engine's
/// `Storage::default_schema_for(tpch_sf01)`.
const DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal INTEGER, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER, c_name TEXT, c_address TEXT, c_nationkey INTEGER, c_phone TEXT, c_acctbal INTEGER, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice INTEGER, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost INTEGER, ps_comment TEXT)",
    "CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, o_totalprice INTEGER, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity INTEGER, l_extendedprice INTEGER, l_discount INTEGER, l_tax INTEGER, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
];

const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

fn lookup_col_types(table: &str) -> Vec<&'static str> {
    for ddl in DDL {
        if let Some(rest) = ddl.strip_prefix("CREATE TABLE ") {
            if let Some(open) = rest.find('(') {
                let cols_str = &rest[open + 1..rest.len() - 1];
                if rest[..open].trim() != table {
                    continue;
                }
                return cols_str
                    .split(',')
                    .map(|c| {
                        let p: Vec<&str> = c.trim().split_whitespace().collect();
                        if p.len() >= 2 {
                            p[1]
                        } else {
                            "TEXT"
                        }
                    })
                    .collect();
            }
        }
    }
    vec![]
}

fn load_engine_with_sf01() -> ExecutionEngine<MemoryStorage> {
    let storage = std::sync::Arc::new(std::sync::RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for d in DDL {
        engine.execute(d).expect("DDL");
    }
    let base = PathBuf::from(FIXTURE_DIR);
    let mut skipped = 0usize;
    for tbl in TABLES {
        let path = base.join(format!("{}.tbl", tbl));
        let content = std::fs::read_to_string(&path).expect("read fixture");
        let col_types = lookup_col_types(tbl);
        // Build one multi-row INSERT to amortize per-statement overhead.
        const CHUNK: usize = 500;
        let mut chunk_rows: Vec<String> = Vec::with_capacity(CHUNK);
        let mut n_rows = 0usize;
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let trimmed = line.trim_end_matches('|');
            let cols: Vec<&str> = trimmed.split('|').collect();
            if cols.len() != col_types.len() {
                skipped += 1;
                continue;
            }
            let vals: Vec<String> = cols
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let ty = col_types.get(i).copied().unwrap_or("TEXT");
                    if ty == "INTEGER" || ty == "REAL" {
                        s.to_string()
                    } else {
                        format!("'{}'", s.replace('\'', "''"))
                    }
                })
                .collect();
            chunk_rows.push(format!("({})", vals.join(",")));
            n_rows += 1;
            if chunk_rows.len() >= CHUNK {
                let sql = format!(
                    "INSERT INTO {} VALUES {}",
                    tbl,
                    chunk_rows.join(",")
                );
                if let Err(e) = engine.execute(&sql) {
                    eprintln!("WARN: engine INSERT chunk failed on {tbl}: {e}");
                }
                chunk_rows.clear();
            }
        }
        if !chunk_rows.is_empty() {
            let sql = format!("INSERT INTO {} VALUES {}", tbl, chunk_rows.join(","));
            if let Err(e) = engine.execute(&sql) {
                eprintln!("WARN: engine INSERT final chunk failed on {tbl}: {e}");
            }
        }
        eprintln!("[load] {tbl}: {n_rows} rows inserted (chunks of {})", CHUNK);
    }
    if skipped > 0 {
        eprintln!("WARN: {skipped} fixture rows skipped due to column-count mismatch");
    }
    engine
}

fn build_sqlite_init_sql() -> String {
    let mut sql_buffer = String::new();
    for ddl in DDL {
        sql_buffer.push_str(ddl);
        sql_buffer.push(';');
        sql_buffer.push('\n');
    }
    let base = PathBuf::from(FIXTURE_DIR);
    for tbl in TABLES {
        let path = base.join(format!("{}.tbl", tbl));
        let content = std::fs::read_to_string(&path).expect("read fixture");
        let col_types = lookup_col_types(tbl);
        let n_cols = col_types.len();
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let trimmed = line.trim_end_matches('|');
            let cols: Vec<&str> = trimmed.split('|').collect();
            if cols.len() != n_cols {
                continue;
            }
            let vals: Vec<String> = cols
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let ty = col_types.get(i).copied().unwrap_or("TEXT");
                    if ty == "INTEGER" || ty == "REAL" {
                        s.to_string()
                    } else {
                        format!("'{}'", s.replace('\'', "''"))
                    }
                })
                .collect();
            sql_buffer.push_str(&format!("INSERT INTO {} VALUES ({});\n", tbl, vals.join(",")));
        }
    }
    sql_buffer
}

fn load_sqlite_with_sf01() -> String {
    let _ = fs::create_dir_all(TMP_DIR);
    let db_path = format!("{}/tpch_sf01.sqlite", TMP_DIR);
    let _ = fs::remove_file(&db_path);
    let sql = build_sqlite_init_sql();
    let sql_file = format!("{}/init.sql", TMP_DIR);
    fs::write(&sql_file, &sql).expect("write init.sql");
    let status = Command::new("sqlite3")
        .arg(&db_path)
        .arg(format!(".read {}", sql_file))
        .status()
        .expect("sqlite3 CLI must be available");
    assert!(status.success(), "sqlite3 .read failed: {:?}", status);
    db_path
}

fn sqlite_count(sql: &str, db_path: &str) -> Option<usize> {
    let wrapped = format!("SELECT COUNT(*) FROM ({}) sub", sql);
    let out = Command::new("sqlite3")
        .arg(db_path)
        .arg(&wrapped)
        .output()
        .expect("sqlite3 exec");
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        eprintln!("SQLite ERR: {}", err.lines().next().unwrap_or(""));
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.trim().parse::<usize>().ok()
}

#[test]
fn eval_22_vs_sqlite_sf01_canonical() {
    println!(
        "\n=== TPC-H 22 audit on CANONICAL SF=0.01 fixture ({}) ===\n",
        FIXTURE_DIR
    );
    let mut engine = load_engine_with_sf01();
    let db_path = load_sqlite_with_sf01();

    let mut matched = 0usize;
    let mut mismatched = 0usize;
    let mut err_count = 0usize;
    let mut sqlite_err = 0usize;
    for q in 1..=22u8 {
        let sql_path = format!("{}/q{}.sql", QUERIES_DIR, q);
        let sql = fs::read_to_string(&sql_path).unwrap_or_else(|_| panic!("read {}", sql_path));
        let sql = sql.trim().trim_end_matches(';');

        let sqlite_count = sqlite_count(sql, &db_path);
        if sqlite_count.is_none() {
            sqlite_err += 1;
        }

        let engine_count: Option<usize> = match engine.execute(&format!(
            "SELECT COUNT(*) FROM ({}) sub",
            sql
        )) {
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

        let cat = match (engine_count, sqlite_count) {
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
            (_, None) => {
                sqlite_err += 1;
                "SQLITE_ERR"
            }
        };
        println!(
            "Q{:>2}: engine={:>6?} sqlite={:>6?}  {}",
            q, engine_count, sqlite_count, cat
        );
    }
    println!("\n=== Totals ===");
    println!("MATCHED   : {matched}/22");
    println!("MISMATCHED: {mismatched}/22");
    println!("ERR(engine): {err_count}/22");
    println!("SQLiteERR : {sqlite_err}/22");
    let total = matched + mismatched + err_count;
    assert!(total >= 22, "category counts do not add up to 22: {}", total);
}
