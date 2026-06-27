//! Sprint 1.5: Cell-level Differential Test (PostgreSQL as Canonical Truth)
//!
//! Extends the 4-way harness with byte-level cell comparison. PostgreSQL
//! is the canonical truth source; each other engine is compared cell-by-cell
//! against PG's results for the same query.
//!
//! This catches subtle correctness bugs that row_count-only comparison
//! misses: e.g. SUM values off by 0.01, DATE filter missing one row,
//! aggregate AVG off by 1 ULP.
//!
//! # Output
//!
//! Writes a JSON diff report to
//! `docs/audit/status/2026-06-07-tpch-cell-diff-v390.json` listing every
//! row/column/value that differs from PostgreSQL.
//!
//! # Skip
//!
//! Skips with `SKIP_CELL_DIFF=1` (e.g. when PG is not available).
//!
//! Refs:
//! - docs/audit/status/2026-06-07-tpch-failure-matrix-v390.md
//! - docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md

mod four_way_harness;

use four_way_harness::{
    default_data_dir, run_mysql_sql, run_psql_sql, Engine, QueryResult, TABLE_COLS,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

const SCHEMA_SQL_FOR_SQLITE: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER, n_nationkey_alt INTEGER, n_regionkey INTEGER, n_name TEXT, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER, c_name TEXT, c_address TEXT, c_nationkey INTEGER, s_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT)",
    "CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
];

fn parse_psql_output(stdout: &str) -> Vec<Vec<String>> {
    stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split('|').map(String::from).collect())
        .collect()
}

fn normalize_sqlrustgo_cell(s: &str) -> String {
    if s == "Null" || s == "Null()" {
        return String::new();
    }
    if let Some(inner) = s
        .strip_prefix("Text(\"")
        .and_then(|x| x.strip_suffix("\")"))
    {
        return inner.replace("\\\"", "\"").replace("\\\\", "\\");
    }
    if let Some(inner) = s.strip_prefix("Integer(").and_then(|x| x.strip_suffix(")")) {
        return inner.to_string();
    }
    if let Some(inner) = s.strip_prefix("Float(").and_then(|x| x.strip_suffix(")")) {
        return inner.to_string();
    }
    s.to_string()
}

fn normalize_sqlrustgo_rows(rows: &[Vec<String>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|r| r.iter().map(|c| normalize_sqlrustgo_cell(c)).collect())
        .collect()
}

fn parse_mysql_output(stdout: &str) -> Vec<Vec<String>> {
    stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split('\t').map(String::from).collect())
        .collect()
}

fn load_queries() -> Vec<(u8, String)> {
    let mut queries = Vec::new();
    for q in 1..=22u8 {
        let path = PathBuf::from("queries").join(format!("q{}.sql", q));
        if let Ok(s) = fs::read_to_string(&path) {
            queries.push((q, s.trim().trim_end_matches(';').to_string()));
        }
    }
    queries
}

fn run_pg_queries(queries: &[(u8, String)]) -> BTreeMap<u8, Vec<Vec<String>>> {
    let mut out = BTreeMap::new();
    for (q, sql) in queries {
        match run_psql_sql(sql) {
            Ok(stdout) => {
                out.insert(*q, parse_psql_output(&stdout));
            }
            Err(e) => {
                eprintln!("[PG] Q{} error: {}", q, e);
                out.insert(*q, vec![]);
            }
        }
    }
    out
}

fn run_mariadb_queries(queries: &[(u8, String)]) -> BTreeMap<u8, Vec<Vec<String>>> {
    let mut out = BTreeMap::new();
    for (q, sql) in queries {
        match run_mysql_sql(sql) {
            Ok(stdout) => {
                out.insert(*q, parse_mysql_output(&stdout));
            }
            Err(e) => {
                eprintln!("[MariaDB] Q{} error: {}", q, e);
                out.insert(*q, vec![]);
            }
        }
    }
    out
}

fn run_sqlite_queries(queries: &[(u8, String)], data_dir: &Path) -> BTreeMap<u8, Vec<Vec<String>>> {
    use rusqlite::Connection;
    let mut out = BTreeMap::new();
    let conn = Connection::open_in_memory().expect("sqlite open");
    for ddl in SCHEMA_SQL_FOR_SQLITE {
        let _ = conn.execute(ddl, []);
    }
    for (table, cols) in TABLE_COLS {
        let path = data_dir.join(format!("{}.tbl", table));
        if !path.exists() {
            continue;
        }
        let inserts = four_way_harness::load_tbl_inserts(table, *cols, &path, Engine::Sqlite);
        for ins in &inserts {
            let _ = conn.execute(ins, []);
        }
    }
    for (q, sql) in queries {
        let mut stmt = conn.prepare(sql);
        let mut cells: Vec<Vec<String>> = Vec::new();
        if let Ok(ref mut s) = stmt {
            if let Ok(mut rows) = s.query([]) {
                while let Some(row) = rows.next().unwrap_or(None) {
                    let mut vals: Vec<String> = Vec::new();
                    let mut i = 0;
                    loop {
                        let v: rusqlite::types::Value = match row.get(i) {
                            Ok(v) => v,
                            Err(_) => break,
                        };
                        vals.push(format!("{:?}", v));
                        i += 1;
                    }
                    cells.push(vals);
                }
            }
        }
        out.insert(*q, cells);
    }
    out
}

fn run_sqlrustgo_queries(
    queries: &[(u8, String)],
    data_dir: &Path,
) -> BTreeMap<u8, Vec<Vec<String>>> {
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
    use std::sync::{Arc, RwLock};
    let mut out = BTreeMap::new();
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(Arc::clone(&storage));
    for ddl in four_way_harness::tpc_h_schema(Engine::SqlRustGo) {
        let _ = engine.execute(&ddl);
    }
    for (table, cols) in TABLE_COLS {
        let path = data_dir.join(format!("{}.tbl", table));
        if !path.exists() {
            continue;
        }
        let inserts = four_way_harness::load_tbl_inserts(table, *cols, &path, Engine::SqlRustGo);
        for ins in &inserts {
            let _ = engine.execute(ins);
        }
    }
    for (q, sql) in queries {
        let cells: Vec<Vec<String>> = match engine.execute(sql) {
            Ok(r) => r
                .rows
                .iter()
                .map(|row| row.iter().map(|v| format!("{:?}", v)).collect())
                .collect(),
            Err(_) => Vec::new(),
        };
        out.insert(*q, cells);
    }
    out
}

#[derive(Debug, Default)]
struct CellDiff {
    query: u8,
    engine: String,
    pg_row_count: usize,
    other_row_count: usize,
    first_mismatches: Vec<SingleCellDiff>,
    status: String,
}

#[derive(Debug)]
struct SingleCellDiff {
    row: usize,
    column: usize,
    expected: String,
    actual: String,
}

fn diff_rows(pg: &[Vec<String>], other: &[Vec<String>]) -> (usize, Vec<SingleCellDiff>) {
    let mut sorted_pg = pg.to_vec();
    sorted_pg.sort();
    let mut sorted_other = other.to_vec();
    sorted_other.sort();

    let mut diffs: Vec<SingleCellDiff> = Vec::new();
    let max_diffs = 10;
    for i in 0..sorted_pg.len().min(sorted_other.len()) {
        let pg_row = &sorted_pg[i];
        let other_row = &sorted_other[i];
        let max_cols = pg_row.len().max(other_row.len());
        for j in 0..max_cols {
            let pg_val = pg_row.get(j).cloned().unwrap_or_default();
            let other_val = other_row.get(j).cloned().unwrap_or_default();
            if pg_val != other_val {
                if diffs.len() < max_diffs {
                    diffs.push(SingleCellDiff {
                        row: i,
                        column: j,
                        expected: pg_val,
                        actual: other_val,
                    });
                }
            }
        }
    }
    (
        max_diffs
            .saturating_sub(diffs.len())
            .min(sorted_pg.len().min(sorted_other.len())),
        diffs,
    )
}

fn diff_rows_normalized(
    pg: &[Vec<String>],
    other_norm: &[Vec<String>],
) -> (usize, Vec<SingleCellDiff>) {
    let mut sorted_pg = pg.to_vec();
    sorted_pg.sort();
    let mut sorted_other = other_norm.to_vec();
    sorted_other.sort();

    let mut diffs: Vec<SingleCellDiff> = Vec::new();
    let max_diffs = 10;
    for i in 0..sorted_pg.len().min(sorted_other.len()) {
        let pg_row = &sorted_pg[i];
        let other_row = &sorted_other[i];
        let max_cols = pg_row.len().max(other_row.len());
        for j in 0..max_cols {
            let pg_val = pg_row.get(j).cloned().unwrap_or_default();
            let other_val = other_row.get(j).cloned().unwrap_or_default();
            if pg_val != other_val {
                if diffs.len() < max_diffs {
                    diffs.push(SingleCellDiff {
                        row: i,
                        column: j,
                        expected: pg_val,
                        actual: other_val,
                    });
                }
            }
        }
    }
    (
        max_diffs
            .saturating_sub(diffs.len())
            .min(sorted_pg.len().min(sorted_other.len())),
        diffs,
    )
}

fn compute_diffs(
    pg: &BTreeMap<u8, Vec<Vec<String>>>,
    other: &BTreeMap<u8, Vec<Vec<String>>>,
    engine_name: &str,
    normalize_other: bool,
) -> Vec<CellDiff> {
    let mut diffs = Vec::new();
    for q in 1..=22u8 {
        let pg_rows = pg.get(&q).cloned().unwrap_or_default();
        let other_rows = other.get(&q).cloned().unwrap_or_default();
        let other_for_compare = if normalize_other {
            normalize_sqlrustgo_rows(&other_rows)
        } else {
            other_rows.clone()
        };
        let status = if pg_rows.len() != other_for_compare.len() {
            "row_count_mismatch".to_string()
        } else {
            "cell_diff".to_string()
        };
        let (_, first_diffs) = diff_rows_normalized(&pg_rows, &other_for_compare);
        diffs.push(CellDiff {
            query: q,
            engine: engine_name.to_string(),
            pg_row_count: pg_rows.len(),
            other_row_count: other_rows.len(),
            first_mismatches: first_diffs,
            status,
        });
    }
    diffs
}

fn write_json_report(
    sqlrustgo: &[CellDiff],
    sqlite: &[CellDiff],
    mariadb: &[CellDiff],
    output_path: &Path,
) {
    let mut file = fs::File::create(output_path).expect("create output json");
    writeln!(file, "{{").unwrap();
    writeln!(file, "  \"date\": \"2026-06-07\",").unwrap();
    writeln!(file, "  \"data\": \"SF=1 simplified (60K lineitem)\",").unwrap();
    writeln!(file, "  \"truth_source\": \"PostgreSQL\",").unwrap();
    writeln!(file, "  \"query_count\": 22,").unwrap();
    writeln!(file, "  \"summary\": {{").unwrap();

    let summary_entries: Vec<(&str, &CellDiff)> = vec![];
    let _ = summary_entries;
    for (i, (label, diffs)) in [
        ("sqlrustgo", sqlrustgo),
        ("sqlite", sqlite),
        ("mariadb", mariadb),
    ]
    .iter()
    .enumerate()
    {
        let row_mismatch: usize = diffs
            .iter()
            .filter(|d| d.status == "row_count_mismatch")
            .count();
        let cell_diff: usize = diffs
            .iter()
            .filter(|d| d.status == "cell_diff" && !d.first_mismatches.is_empty())
            .count();
        let clean: usize = diffs
            .iter()
            .filter(|d| d.status == "cell_diff" && d.first_mismatches.is_empty())
            .count();
        let comma = if i < 2 { "," } else { "" };
        writeln!(
            file,
            "    \"{lab}\": {{\"row_count_mismatch\": {rm}, \"cell_diff\": {cd}, \"clean_match\": {cm}}}{c}",
            lab = label,
            rm = row_mismatch,
            cd = cell_diff,
            cm = clean,
            c = comma
        )
        .unwrap();
    }
    writeln!(file, "  }},").unwrap();
    writeln!(file, "  \"details\": {{").unwrap();
    for (i, (label, diffs)) in [
        ("sqlrustgo", sqlrustgo),
        ("sqlite", sqlite),
        ("mariadb", mariadb),
    ]
    .iter()
    .enumerate()
    {
        writeln!(file, "    \"{lab}\": [", lab = label).unwrap();
        let mut first = true;
        for d in diffs.iter() {
            if d.status == "cell_diff" && d.first_mismatches.is_empty() {
                continue;
            }
            let comma = if first { "" } else { "," };
            writeln!(file, "      {c}{{", c = comma).unwrap();
            writeln!(file, "        \"query\": \"Q{:02}\",", d.query).unwrap();
            writeln!(file, "        \"status\": \"{}\",", d.status).unwrap();
            writeln!(file, "        \"pg_row_count\": {},", d.pg_row_count).unwrap();
            writeln!(file, "        \"other_row_count\": {},", d.other_row_count).unwrap();
            if !d.first_mismatches.is_empty() {
                writeln!(file, "        \"first_mismatches\": [").unwrap();
                for (j, m) in d.first_mismatches.iter().enumerate() {
                    let mcomma = if j < d.first_mismatches.len() - 1 {
                        ","
                    } else {
                        ""
                    };
                    writeln!(
                        file,
                        "          {{\"row\": {}, \"column\": {}, \"expected\": {:?}, \"actual\": {:?}}}{c}",
                        m.row,
                        m.column,
                        m.expected,
                        m.actual,
                        c = mcomma
                    )
                    .unwrap();
                }
                writeln!(file, "        ]").unwrap();
            } else {
                writeln!(file, "        \"first_mismatches\": []").unwrap();
            }
            writeln!(file, "      }}").unwrap();
            first = false;
        }
        let dcomma = if i < 2 { "," } else { "" };
        writeln!(file, "    ]{d}", d = dcomma).unwrap();
    }
    writeln!(file, "  }}").unwrap();
    writeln!(file, "}}").unwrap();
}

#[test]
fn test_four_way_cell_diff_postgres_truth() {
    if std::env::var("SKIP_CELL_DIFF").is_ok() {
        eprintln!("\n=== Sprint 1.5 Cell-Diff [SKIPPED via SKIP_CELL_DIFF=1] ===");
        return;
    }

    let data_dir = default_data_dir();
    eprintln!("\n=== Sprint 1.5: Cell-Level Diff (PG as Truth) ===");
    eprintln!("Data dir: {:?}", data_dir);

    let queries = load_queries();
    eprintln!("[queries] Loaded {} queries", queries.len());

    eprintln!("[PG] Running 22 queries (canonical truth)...");
    let t = Instant::now();
    let pg = run_pg_queries(&queries);
    eprintln!("[PG] done in {:.1}s", t.elapsed().as_secs_f64());

    eprintln!("[sqlrustgo] Running 22 queries...");
    let t = Instant::now();
    let sr = run_sqlrustgo_queries(&queries, &data_dir);
    eprintln!("[sqlrustgo] done in {:.1}s", t.elapsed().as_secs_f64());

    eprintln!("[sqlite] Running 22 queries...");
    let t = Instant::now();
    let sq = run_sqlite_queries(&queries, &data_dir);
    eprintln!("[sqlite] done in {:.1}s", t.elapsed().as_secs_f64());

    eprintln!("[mariadb] Running 22 queries...");
    let t = Instant::now();
    let md = run_mariadb_queries(&queries);
    eprintln!("[mariadb] done in {:.1}s", t.elapsed().as_secs_f64());

    let sqlrustgo_diffs = compute_diffs(&pg, &sr, "sqlrustgo", true);
    let sqlite_diffs = compute_diffs(&pg, &sq, "sqlite", true);
    let mariadb_diffs = compute_diffs(&pg, &md, "mariadb", false);

    let output = PathBuf::from("docs/audit/status/2026-06-07-tpch-cell-diff-v390.json");
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).ok();
    }
    write_json_report(&sqlrustgo_diffs, &sqlite_diffs, &mariadb_diffs, &output);
    eprintln!("\n[output] Cell-diff JSON written to {:?}", output);

    eprintln!("\n=== Summary (PG as truth) ===");
    for (label, diffs) in [
        ("sqlrustgo", &sqlrustgo_diffs),
        ("sqlite", &sqlite_diffs),
        ("mariadb", &mariadb_diffs),
    ]
    .iter()
    {
        let rm: usize = diffs
            .iter()
            .filter(|d| d.status == "row_count_mismatch")
            .count();
        let cd: usize = diffs
            .iter()
            .filter(|d| d.status == "cell_diff" && !d.first_mismatches.is_empty())
            .count();
        let cl: usize = diffs
            .iter()
            .filter(|d| d.status == "cell_diff" && d.first_mismatches.is_empty())
            .count();
        eprintln!(
            "  {lab:10}: row_count_mismatch={rm:>2}  cell_diff={cd:>2}  clean_match={cl:>2}  (of 22)",
            lab = label,
            rm = rm,
            cd = cd,
            cl = cl
        );
    }

    let _: QueryResult = QueryResult {
        engine: Engine::SqlRustGo,
        query: 0,
        row_count: 0,
        duration_ms: 0,
        error: None,
        sample_rows: vec![],
    };
}
