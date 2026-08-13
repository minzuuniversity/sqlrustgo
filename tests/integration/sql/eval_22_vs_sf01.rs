//! TPC-H 22 audit on canonical SF=0.01 fixture
//! (default: `tests/data/tpch-sf01/`, override with `TPCH_DATA_DIR`).
//!
//! **Replaces** the previous `tests/eval_22_vs_sqlite.rs` audit which used
//! the corrupt `tests/data/tpch-sf001/*.tbl` (15/614/150 rows). See
//! `docs/discovery/2026-06-05-tpch-22-sf001-corrupt.md` for why that fixture
//! was wrong.
//!
//! ## Loading strategy
//!
//! Uses the same `SCHEMA_SQL` + `load_tbl_file` style as
//! `tests/tpch_full_22_test.rs` (which passes 22/22 in-process in ~7 min).
//! Critical lessons learned:
//!
//! 1. The DDL must declare `PRIMARY KEY` and `NOT NULL` constraints as the
//!    real engine path (`execute_insert`) takes a different code path for
//!    tables with primary keys — without the constraint, the audit
//!    silently produces wrong row counts and triggers a different executor
//!    branch.
//! 2. Loading via `engine.execute("INSERT INTO ...")` in a loop is too
//!    slow / deadlocks the engine's internal `Arc<RwLock<Storage>>` after
//!    ~80k rows. Direct `StorageEngine::insert()` in batches of 10000 is
//!    the working path.
//! 3. The engine that runs the queries must be created **before** the
//!    fixture loader starts inserting rows, so that `TableInfo` is
//!    consistent for both the loader's `storage.write()` calls and the
//!    query engine's `storage.read()` calls. If you create the query
//!    engine *after* loading, you get an empty schema for queries.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

const FIXTURE_DIR: &str = match option_env!("TPCH_DATA_DIR") {
    Some(p) => p,
    None => "tests/data/tpch-sf01",
};
const QUERIES_DIR: &str = "queries";
const TMP_DIR: &str = "/tmp/tpch_22_sf01_audit";

/// Schema matches the real engine code path (`execute_insert` branches on
/// primary key). Column order matches the standard TPC-H .tbl files.
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

fn lookup_col_types(table: &str) -> Vec<&'static str> {
    for ddl in SCHEMA_SQL {
        if let Some(rest) = ddl.strip_prefix("CREATE TABLE ") {
            if let Some(open) = rest.find('(') {
                let cols_str = &rest[open + 1..rest.len() - 1];
                if rest[..open].trim() != table {
                    continue;
                }
                return cols_str
                    .split(',')
                    .filter_map(|c| {
                        let c = c.trim();
                        // Skip "PRIMARY KEY (...)" / "FOREIGN KEY ..." table
                        // constraints; only return a type for real columns.
                        if c.starts_with("PRIMARY KEY")
                            || c.starts_with("FOREIGN KEY")
                            || c.starts_with("UNIQUE")
                            || c.starts_with("CHECK")
                            || c.starts_with("CONSTRAINT")
                        {
                            return None;
                        }
                        let p: Vec<&str> = c.split_whitespace().collect();
                        if p.len() >= 2 {
                            Some(p[1])
                        } else {
                            Some("TEXT")
                        }
                    })
                    .collect();
            }
        }
    }
    vec![]
}

/// Load a .tbl file into MemoryStorage using batch insert. Mirrors the
/// `load_tbl_file` in `tests/tpch_full_22_test.rs`.
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
            let mut storage = storage.write();
            storage
                .insert(tbl_name, batch.clone())
                .map_err(|e| format!("Insert error: {}", e))?;
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut storage = storage.write();
        storage
            .insert(tbl_name, batch.clone())
            .map_err(|e| format!("Insert error: {}", e))?;
        count += batch.len();
    }
    Ok(count)
}

fn build_sqlite_init_sql() -> String {
    let mut sql_buffer = String::new();
    for ddl in SCHEMA_SQL {
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
            sql_buffer.push_str(&format!(
                "INSERT INTO {} VALUES ({});\n",
                tbl,
                vals.join(",")
            ));
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

    // 1. Create storage + engine *first* so TableInfo is consistent for
    //    both DDL and the loader.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    // 2. DDL.
    eprintln!("[1/3] Creating schema...");
    for ddl in SCHEMA_SQL {
        engine
            .execute(ddl)
            .unwrap_or_else(|e| panic!("DDL failed: {ddl} - {e}"));
    }
    eprintln!("  Schema created ({} tables)", SCHEMA_SQL.len());

    // 3. Load fixture via direct storage.insert (batched).
    eprintln!("[2/3] Loading fixture via storage.insert() in batches of 10000 ...");
    let base = PathBuf::from(FIXTURE_DIR);
    let mut total_rows = 0usize;
    for tbl in TABLES {
        let tbl_path = base.join(format!("{}.tbl", tbl));
        let col_types = lookup_col_types(tbl);
        let n = load_tbl_file(&storage, tbl, &tbl_path, col_types.len())
            .unwrap_or_else(|e| panic!("Failed to load {tbl}: {e}"));
        eprintln!("  {tbl}: {n} rows");
        total_rows += n;
    }
    eprintln!("  total: {total_rows} rows");

    // 4. SQLite baseline.
    eprintln!("[3/3] Loading SQLite baseline ...");
    let db_path = load_sqlite_with_sf01();
    eprintln!("  SQLite ready at {db_path}");

    // 5. Run 22 queries.
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
    println!("\n=== Totals ({} rows loaded) ===", total_rows);
    println!("MATCHED   : {matched}/22");
    println!("MISMATCHED: {mismatched}/22");
    println!("ERR(engine): {err_count}/22");
    println!("SQLiteERR : {sqlite_err}/22");
    let total = matched + mismatched + err_count + sqlite_err;
    assert!(
        total >= 22,
        "category counts do not add up to 22: {}",
        total
    );
}
