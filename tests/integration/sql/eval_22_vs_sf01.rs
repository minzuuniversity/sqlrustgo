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
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

#[path = "../../common/mod.rs"]
mod common;
#[path = "../../common/sqlite_oracle.rs"]
mod sqlite_oracle;
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;
use common::tpch_wire_harness::{report_not_run, resolve_fixture_dir};
use sqlite_oracle::adapt_sql;

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

/// Split a DDL column list at **top-level** commas only.
///
/// A naive `split(',')` breaks on table-level constraints such as
/// `PRIMARY KEY (l_orderkey, l_linenumber)`: the comma inside the
/// parentheses splits one constraint into two fragments, and the second
/// fragment (`l_linenumber)`) no longer starts with a constraint keyword,
/// so it is mistaken for a column. The column count then comes out one
/// too high, every `.tbl` row fails the `cols.len() != n_cols` check, and
/// the table is loaded with **zero** rows — silently, because the mismatch
/// is only ever skipped, never reported.
///
/// That is what happened to `lineitem` (17 vs 16) and `partsupp` (6 vs 5)
/// here: the SQLite side of this audit was empty for the two largest
/// tables, which turned Q1/Q3/Q4/Q12/Q15 into false MISMATCHED verdicts
/// against a perfectly correct engine.
fn split_top_level_commas(cols_str: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, ch) in cols_str.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(&cols_str[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&cols_str[start..]);
    out
}

fn lookup_col_types(table: &str) -> Vec<&'static str> {
    for ddl in SCHEMA_SQL {
        if let Some(rest) = ddl.strip_prefix("CREATE TABLE ") {
            if let Some(open) = rest.find('(') {
                let cols_str = &rest[open + 1..rest.len() - 1];
                if rest[..open].trim() != table {
                    continue;
                }
                return split_top_level_commas(cols_str)
                    .into_iter()
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

fn build_sqlite_init_sql(base: &Path) -> String {
    let mut sql_buffer = String::new();
    for ddl in SCHEMA_SQL {
        sql_buffer.push_str(ddl);
        sql_buffer.push(';');
        sql_buffer.push('\n');
    }
    for tbl in TABLES {
        let path = base.join(format!("{}.tbl", tbl));
        let content = std::fs::read_to_string(&path).expect("read fixture");
        let col_types = lookup_col_types(tbl);
        let n_cols = col_types.len();
        let mut skipped = 0usize;
        let mut observed_fields: Option<usize> = None;
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let trimmed = line.trim_end_matches('|');
            let cols: Vec<&str> = trimmed.split('|').collect();
            if cols.len() != n_cols {
                skipped += 1;
                observed_fields = Some(cols.len());
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
        // A dropped row here empties the SQLite side of the audit while the
        // engine side stays correct, which surfaces as a false MISMATCHED
        // verdict against the engine. Refuse to continue instead.
        assert!(
            skipped == 0,
            "fixture loader dropped {skipped} row(s) of `{tbl}`: the .tbl rows have {} fields but \
             the DDL implies {n_cols} columns. Silently skipping rows makes the SQLite oracle \
             incomplete and turns correct engine results into false mismatches.",
            observed_fields.unwrap_or(0)
        );
    }
    sql_buffer
}

fn load_sqlite_with_sf01(base: &Path) -> String {
    let _ = fs::create_dir_all(TMP_DIR);
    let db_path = format!("{}/tpch_sf01.sqlite", TMP_DIR);
    let _ = fs::remove_file(&db_path);
    let sql = build_sqlite_init_sql(base);
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

/// What the SQLite oracle had to say about one query.
enum OracleVerdict {
    /// SQLite ran the query and produced this many rows.
    Count(usize),
    /// The query uses a construct SQLite cannot run, even after adaptation.
    ///
    /// This is kept apart from [`OracleVerdict::Error`] on purpose: "the
    /// oracle cannot express this" and "the oracle ran it and got a different
    /// answer" are different findings and must not be lumped together.
    Unsupported(String),
    /// SQLite was asked to run the query and failed.
    Error(String),
}

/// Row count of `sql` according to the SQLite oracle.
///
/// The query is first adapted to SQLite's dialect — `EXTRACT(YEAR FROM x)`
/// becomes `strftime('%Y', x)`, which is the only rewrite
/// [`adapt_sql`] performs. Without it, Q7/Q8/Q9 fail to parse in SQLite and
/// silently drop out of the comparison.
fn sqlite_count(sql: &str, db_path: &str) -> OracleVerdict {
    let adapted = adapt_sql(sql);
    if !adapted.unsupported.is_empty() {
        return OracleVerdict::Unsupported(adapted.unsupported.join("; "));
    }
    let wrapped = format!("SELECT COUNT(*) FROM ({}) sub", adapted.sql);
    let out = Command::new("sqlite3")
        .arg(db_path)
        .arg(&wrapped)
        .output()
        .expect("sqlite3 exec");
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return OracleVerdict::Error(err.lines().next().unwrap_or("").to_string());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    match stdout.trim().parse::<usize>() {
        Ok(n) => OracleVerdict::Count(n),
        Err(_) => OracleVerdict::Error(format!("unparsable count {:?}", stdout.trim())),
    }
}

#[test]
fn eval_22_vs_sqlite_sf01_canonical() {
    // The fixture path used to be baked in at compile time through
    // `option_env!("TPCH_DATA_DIR")`, which can never see a run-time
    // environment variable — the CI-provided fixture was therefore never
    // found. It is now resolved at run time; without it there is nothing
    // to audit, so the test is disclosed as *not run*.
    let Some(base) = resolve_fixture_dir() else {
        report_not_run(
            "eval_22_vs_sqlite_sf01_canonical",
            "fixture_missing",
            "no TPC-H fixture found (set TPCH_DATA_DIR to a directory with the 8 .tbl files)",
        );
        return;
    };

    println!(
        "\n=== TPC-H 22 audit on CANONICAL SF=0.01 fixture ({}) ===\n",
        base.display()
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
    let db_path = load_sqlite_with_sf01(&base);
    eprintln!("  SQLite ready at {db_path}");

    // 5. Run 22 queries.
    //
    //    Each query must land in exactly one category, and the categories must
    //    partition all 22. The previous version of this loop counted a query
    //    as `SQLITE_ERR` twice (once before the match, once inside it) and
    //    then asserted only `total >= 22` — a condition that is true by
    //    construction, so the double count and the 3 unadapted queries were
    //    both invisible. The counts are now disjoint and the totals are
    //    asserted to be exactly 22.
    let mut matched = 0usize;
    let mut mismatched: Vec<String> = Vec::new();
    let mut known_diffs: Vec<String> = Vec::new();
    let mut err_count: Vec<String> = Vec::new();
    let mut unsupported: Vec<String> = Vec::new();
    let mut oracle_err: Vec<String> = Vec::new();
    for q in 1..=22u8 {
        let sql_path = format!("{}/q{}.sql", QUERIES_DIR, q);
        let sql = fs::read_to_string(&sql_path).unwrap_or_else(|_| panic!("read {}", sql_path));
        let sql = sql.trim().trim_end_matches(';');

        let verdict = sqlite_count(sql, &db_path);

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

        let case_id = format!("eval_22_vs_sf01:Q{q}");
        let cat = match (&verdict, engine_count) {
            (OracleVerdict::Count(s), Some(e)) if e == *s => {
                matched += 1;
                "MATCHED"
            }
            (OracleVerdict::Count(s), Some(e)) => {
                let detail = format!("engine={e} sqlite={s}");
                match tpch_fixture::lookup_known_difference(&case_id) {
                    Some(entry) => {
                        tpch_fixture::report_known_difference(&entry, &detail);
                        known_diffs.push(case_id.clone());
                    }
                    None => mismatched.push(format!("Q{q}: {detail}")),
                }
                "MISMATCHED"
            }
            (OracleVerdict::Count(s), None) => {
                err_count.push(format!("Q{q}: engine error, sqlite={s}"));
                "ERR"
            }
            (OracleVerdict::Unsupported(why), _) => {
                unsupported.push(format!("Q{q}: {why}"));
                "UNSUPPORTED"
            }
            (OracleVerdict::Error(msg), _) => {
                oracle_err.push(format!("Q{q}: {msg}"));
                "SQLITE_ERR"
            }
        };
        let sqlite_n = match &verdict {
            OracleVerdict::Count(s) => Some(*s),
            _ => None,
        };
        println!(
            "Q{:>2}: engine={:>6?} sqlite={:>6?}  {}",
            q, engine_count, sqlite_n, cat
        );
    }
    println!("\n=== Totals ({} rows loaded) ===", total_rows);
    println!("MATCHED   : {matched}/22");
    println!("MISMATCHED: {}/22", mismatched.len());
    println!("KNOWN-DIFF: {}/22", known_diffs.len());
    println!("ERR(engine): {}/22", err_count.len());
    println!("UNSUPPORTED: {}/22", unsupported.len());
    println!("SQLiteERR : {}/22", oracle_err.len());

    // The categories must partition the 22 queries: no query may be counted
    // twice and none may be dropped.
    assert_eq!(
        matched
            + mismatched.len()
            + known_diffs.len()
            + err_count.len()
            + unsupported.len()
            + oracle_err.len(),
        22,
        "the 22 queries must fall into exactly one category each"
    );
    // A query the oracle cannot express is not a pass — it is an uncompared
    // query, and it must be registered instead of being quietly skipped.
    assert!(
        unsupported.is_empty(),
        "SQLite cannot run {} query/queries after adaptation: {unsupported:?} — \
         each one needs a dialect adaptation or a registered known difference",
        unsupported.len()
    );
    assert!(
        oracle_err.is_empty(),
        "the SQLite oracle failed on {} query/queries: {oracle_err:?}",
        oracle_err.len()
    );
    assert!(
        err_count.is_empty(),
        "the engine failed on {} query/queries: {err_count:?}",
        err_count.len()
    );
    assert!(
        mismatched.is_empty(),
        "engine and SQLite disagree on {} query/queries: {mismatched:?} — \
         register a known difference with an issue and an owner, or fix the engine",
        mismatched.len()
    );
    assert_eq!(
        matched + known_diffs.len(),
        22,
        "every query must be compared against the oracle and agree with it"
    );
}
