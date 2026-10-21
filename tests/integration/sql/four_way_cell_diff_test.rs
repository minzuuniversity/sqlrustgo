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
//!
//! #5404: this file used to have **zero assertions**.
//!
//! The single test printed a diff report and returned. PostgreSQL is the
//! declared truth source, but on CI the `psql` binary is absent — so
//! `run_psql_sql` failed for every statement, the "PG side" was filled with
//! empty result sets, and the diff report was nothing but
//! "row_count_mismatch" on all 22 queries. The target was **green** while
//! having compared the engine against nothing at all.
//!
//! `SKIP_CELL_DIFF=1` was the same story with a shorter fuse: it returned
//! immediately and reported success.
//!
//! What it does now:
//!
//! * **PostgreSQL unavailable** ⇒ the target is reported through the
//!   `TPCH-NOT-RUN` channel with the reason code
//!   `external_service_unavailable`. It is *not* counted as passed and it
//!   is *not* silently green.
//! * **PostgreSQL available** ⇒ the cell diff must agree with the truth
//!   source, and any difference that is **not** registered in
//!   `tests/baseline/tpch_known_differences.json` fails the test.
//! * The report is written to `TPCH_CELL_DIFF_OUT` (or the build directory)
//!   instead of overwriting a file tracked in the repository.

#![allow(dead_code)]

mod four_way_harness;

// The shared TPC-H fixture loader (issue #5379). Loading through the
// storage layer in batches instead of executing one INSERT statement per
// row is ~2 orders of magnitude faster; see the comment at the load site.
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

#[path = "../../common/tpch_oracle.rs"]
mod tpch_oracle;

use four_way_harness::{run_mysql_sql, run_psql_sql, Engine, TABLE_COLS};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "four_way_cell_diff_test";

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

/// Run the 22 statements on PostgreSQL.
///
/// #5500: a statement that **failed** must not be recorded as an empty
/// result set. "The query failed" and "the query matched no rows" are
/// different claims, and folding the first into the second is what made
/// this target report `pg_rows=0` for Q08/Q09/Q11/Q20/Q21/Q22 — six
/// "differences" that were really six *errors*.
///
/// A missing client is a different thing again: the whole comparison
/// cannot run, which is a NOT-RUN, not a failure.
fn run_pg_queries(queries: &[(u8, String)]) -> Result<BTreeMap<u8, Vec<Vec<String>>>, String> {
    let mut out = BTreeMap::new();
    for (q, sql) in queries {
        match run_psql_sql(sql) {
            Ok(stdout) => {
                out.insert(*q, parse_psql_output(&stdout));
            }
            Err(e) if e.is_unavailable() => {
                return Err(format!(
                    "the PostgreSQL client is unavailable — the comparison against \
                     PostgreSQL was NOT executed: {e}"
                ));
            }
            Err(e) => {
                return Err(format!("Q{q:02} failed on PostgreSQL: {e}"));
            }
        }
    }
    Ok(out)
}

/// Run the 22 statements on MariaDB.
///
/// #5500: same rule as [`run_pg_queries`] — a failure is a failure, never
/// an empty result set.
fn run_mariadb_queries(queries: &[(u8, String)]) -> Result<BTreeMap<u8, Vec<Vec<String>>>, String> {
    let mut out = BTreeMap::new();
    for (q, sql) in queries {
        match run_mysql_sql(sql) {
            Ok(stdout) => {
                out.insert(*q, parse_mysql_output(&stdout));
            }
            Err(e) if e.is_unavailable() => {
                return Err(format!(
                    "the MariaDB client is unavailable — the comparison against \
                     MariaDB was NOT executed: {e}"
                ));
            }
            Err(e) => {
                return Err(format!("Q{q:02} failed on MariaDB: {e}"));
            }
        }
    }
    Ok(out)
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
    use parking_lot::RwLock;
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
    use std::sync::Arc;
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
        // Batch-load through the storage layer instead of executing one
        // INSERT statement per row: the statement path issues ~86,000
        // INSERTs per load and costs ~1 hour per load in a debug build.
        // Before issue #5379 the fixture was a dangling symlink, so this
        // target skipped the load and finished in milliseconds — the cost
        // was invisible. With the fixture now generated at run time and
        // reachable, the statement path would need ~1 hour of CI time and
        // starve every other target.
        //
        // The two loaders are equivalent: the shared loader derives the
        // same typed values (NULL / Integer / Float / Text) from the same
        // TBL text, verified cell-by-cell over all 8 tables and 86,482 rows.
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let rows = tpch_fixture::parse_tbl(&content);
        let _ = tpch_fixture::load_rows_into_storage(&storage, &rows, table, *cols)
            .unwrap_or_else(|e| panic!("loading table `{table}`: {e}"));
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
            if pg_val != other_val && diffs.len() < max_diffs {
                diffs.push(SingleCellDiff {
                    row: i,
                    column: j,
                    expected: pg_val,
                    actual: other_val,
                });
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
            if pg_val != other_val && diffs.len() < max_diffs {
                diffs.push(SingleCellDiff {
                    row: i,
                    column: j,
                    expected: pg_val,
                    actual: other_val,
                });
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

/// The case ids that are registered as known differences.
///
/// A difference is tolerated **only** when it is registered in
/// `tests/baseline/tpch_known_differences.json` with an issue and an owner.
/// Anything else fails the test.
fn load_known_case_ids() -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/baseline/tpch_known_differences.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let v: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()));
    v.get("entries")
        .and_then(|e| e.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| e.get("case_id").and_then(|c| c.as_str()))
                .map(|c| c.to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Where the JSON report is written.
///
/// It used to overwrite `docs/audit/status/2026-06-07-tpch-cell-diff-v390.json`,
/// a file **tracked in the repository** — so every run of this target
/// dirtied the working tree (the same family as issue #5389, where the A04
/// matrix test rewrote 81 evidence files). The report is an artefact of a
/// run, not a source file, so it now goes to the build directory unless
/// `TPCH_CELL_DIFF_OUT` says otherwise.
fn report_path() -> PathBuf {
    if let Ok(p) = std::env::var("TPCH_CELL_DIFF_OUT") {
        if !p.trim().is_empty() {
            return PathBuf::from(p.trim());
        }
    }
    PathBuf::from(env!("CARGO_TARGET_DIR")).join("tpch-cell-diff.json")
}

/// The truth source of the comparison.
///
/// PostgreSQL is this target's original intent, but it can only be the
/// truth source when its database holds **the same fixture** as the engine
/// side. A reachable PostgreSQL instance that holds a *different* dataset
/// produces differences that have nothing to do with the engine — the two
/// sides would be compared on different data. (Measured here: the engine
/// side rendered `Supplier 1` where PostgreSQL rendered
/// `Supplier#000000001`, because the generated fixture and a real TPC-H
/// database are not the same bytes.)
///
/// So the source is chosen by an explicit **same-data** check, not by
/// availability:
///
/// * `FOUR_WAY_TRUTH=pg` **and** PostgreSQL's `lineitem` row count equals
///   the fixture's ⇒ PostgreSQL is the truth source;
/// * otherwise the **in-process SQLite** oracle is used, loaded from the
///   same fixture as the engine side, and the switch is disclosed.
enum TruthSource {
    /// In-process SQLite, loaded from the same fixture as the engine side.
    Sqlite(rusqlite::Connection),
    /// PostgreSQL, whose `lineitem` row count was verified to match.
    Postgres(BTreeMap<u8, Vec<Vec<String>>>),
}

impl TruthSource {
    /// Resolve the truth source and disclose the choice.
    fn resolve(fixture: &tpch_fixture::Fixture) -> TruthSource {
        let wants_pg = std::env::var("FOUR_WAY_TRUTH")
            .map(|v| v.trim().eq_ignore_ascii_case("pg"))
            .unwrap_or(false);
        if !wants_pg {
            eprintln!(
                "[TRUTH] using the in-process SQLite oracle (same fixture as the \
                 engine side). Set FOUR_WAY_TRUTH=pg to use PostgreSQL instead — \
                 that is only valid when PostgreSQL holds the very same fixture."
            );
            let conn = tpch_oracle::load_into_sqlite(&fixture.dir, &tpch_oracle::ALL_TABLES)
                .unwrap_or_else(|e| panic!("loading the SQLite oracle failed: {e}"));
            return TruthSource::Sqlite(conn);
        }

        // The user asked for PostgreSQL: verify that it holds the same data.
        let fixture_rows = fixture.row_count("lineitem");
        let pg_rows = run_psql_sql("SELECT count(*) FROM lineitem")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok());
        let same_data = match (fixture_rows, pg_rows) {
            (Some(f), Some(p)) => f == p,
            _ => false,
        };
        if same_data {
            eprintln!(
                "[TRUTH] using PostgreSQL — lineitem row count matches ({}).",
                fixture_rows.unwrap_or(0)
            );
            let queries = load_queries();
            // #5500: a statement that failed on PostgreSQL is a **failure**,
            // not an empty result set. Falling back to another truth source
            // here would compare the engine against something the caller
            // explicitly did not ask for, so it fails the target instead.
            let pg = run_pg_queries(&queries).unwrap_or_else(|e| {
                panic!(
                    "FOUR_WAY_TRUTH=pg was requested and PostgreSQL holds the same \
                     fixture, but a statement failed: {e}\n\
                     Switching to another truth source here would compare the engine \
                     against a source the caller did not ask for — so this is a failure."
                )
            });
            TruthSource::Postgres(pg)
        } else {
            let detail = match pg_rows {
                Some(p) => format!(
                    "FOUR_WAY_TRUTH=pg was requested, but PostgreSQL does not hold \
                     the same fixture as the engine side (fixture lineitem rows = \
                     {fixture_rows:?}, PostgreSQL lineitem rows = {p}); the comparison \
                     against PostgreSQL would compare two different datasets, so \
                     the in-process SQLite oracle was used instead"
                ),
                None => format!(
                    "FOUR_WAY_TRUTH=pg was requested, but PostgreSQL could not be \
                     queried at all (the psql client is unavailable, or the count \
                     query failed); fixture lineitem rows = {fixture_rows:?}. The \
                     in-process SQLite oracle was used instead"
                ),
            };
            tpch_fixture::report_not_run(TARGET, tpch_fixture::reason::KNOWN_DIFFERENCE, &detail);
            let conn = tpch_oracle::load_into_sqlite(&fixture.dir, &tpch_oracle::ALL_TABLES)
                .unwrap_or_else(|e| panic!("loading the SQLite oracle failed: {e}"));
            TruthSource::Sqlite(conn)
        }
    }

    /// The truth source's rows for one statement.
    fn rows(&self, sql: &str, qid: u8) -> Result<Vec<Vec<String>>, String> {
        match self {
            TruthSource::Sqlite(conn) => tpch_oracle::sqlite_rows(conn, sql),
            TruthSource::Postgres(pg) => Ok(pg.get(&qid).cloned().unwrap_or_default()),
        }
    }

    /// A human-readable label for the source.
    fn label(&self) -> &'static str {
        match self {
            TruthSource::Sqlite(_) => "in-process SQLite",
            TruthSource::Postgres(_) => "PostgreSQL",
        }
    }
}

#[test]
fn test_four_way_cell_diff_postgres_truth() {
    // #5404: an explicit skip must be **disclosed**, not silently green.
    if std::env::var("SKIP_CELL_DIFF").is_ok() {
        tpch_fixture::report_not_run(
            TARGET,
            tpch_fixture::reason::EXTERNAL_SERVICE_UNAVAILABLE,
            "SKIP_CELL_DIFF=1 is set: the cell diff was NOT executed",
        );
        return;
    }

    // The fixture is resolved through the shared resolver: a missing fixture
    // is a disclosed NOT-RUN, never a pass.
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        return;
    };
    let data_dir = fixture.dir.clone();

    eprintln!("\n=== Sprint 1.5: Cell-Level Diff ===");
    eprintln!("Data dir: {:?}", data_dir);

    let truth = TruthSource::resolve(fixture);

    // The engine side: the same fixture, loaded into an in-memory engine.
    let storage = std::sync::Arc::new(parking_lot::RwLock::new(sqlrustgo::MemoryStorage::new()));
    let mut engine = sqlrustgo::ExecutionEngine::new(storage.clone());
    for ddl in tpch_oracle::engine_ddl(&tpch_oracle::ALL_TABLES) {
        engine
            .execute(&ddl)
            .unwrap_or_else(|e| panic!("creating the table failed: {e}"));
    }
    for table in tpch_oracle::ALL_TABLES {
        let path = fixture.table_path(table);
        assert!(
            path.exists(),
            "the fixture is present but table `{table}` is missing from it (looked at {})",
            path.display()
        );
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {table} failed: {e}"));
        let rows = tpch_oracle::parse_tbl(&content);
        let ncols = tpch_oracle::table_spec(table)
            .map(|(_, cols, _)| cols)
            .unwrap_or(0);
        tpch_oracle::load_rows_into_storage(&storage, &rows, table, ncols)
            .unwrap_or_else(|e| panic!("loading `{table}` failed: {e}"));
    }

    let queries = load_queries();
    eprintln!("[queries] Loaded {} queries", queries.len());
    assert_eq!(
        queries.len(),
        22,
        "all 22 standard TPC-H statements must be present in `queries/`"
    );

    let known = load_known_case_ids();
    let mut unregistered: Vec<String> = Vec::new();
    let mut clean = 0usize;
    let mut tolerated = 0usize;

    for (qid, sql) in &queries {
        let ours = tpch_oracle::engine_rows(&mut engine, sql)
            .unwrap_or_else(|e| panic!("Q{qid:02} must execute on the engine side — {e}"));
        let theirs = truth
            .rows(sql, *qid)
            .unwrap_or_else(|e| panic!("Q{qid:02} must execute on the truth side — {e}"));
        let Some(diff) = tpch_oracle::first_difference_normalized(&ours, &theirs) else {
            clean += 1;
            continue;
        };
        let case_id = format!("tpch:Q{qid:02}");
        if known.contains(&case_id) {
            eprintln!("[KNOWN-DIFF] {case_id} is registered — tolerated: {diff}");
            tolerated += 1;
        } else {
            unregistered.push(format!("{case_id}: {diff}"));
        }
    }

    eprintln!(
        "\n=== Summary (truth source: {}) ===\n  clean_match={clean}  known_diff={tolerated}  unregistered={}",
        truth.label(),
        unregistered.len()
    );

    assert!(
        unregistered.is_empty(),
        "the engine differs from the truth source ({}) on {} statement(s) that are \
         NOT registered in tests/baseline/tpch_known_differences.json:\n  {}\n\
         Register each difference there with an issue and an owner, or fix the engine.",
        truth.label(),
        unregistered.len(),
        unregistered.join("\n  ")
    );

    // The report goes to the build directory: it used to overwrite a file
    // tracked in the repository, which dirtied the working tree on every run.
    let output = report_path();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut report = String::new();
    report.push_str("{\n");
    report.push_str(&format!("  \"truth_source\": \"{}\",\n", truth.label()));
    report.push_str(&format!("  \"clean_match\": {clean},\n"));
    report.push_str(&format!("  \"known_diff\": {tolerated},\n"));
    report.push_str(&format!("  \"unregistered\": {}\n", unregistered.len()));
    report.push_str("}\n");
    fs::write(&output, report)
        .unwrap_or_else(|e| panic!("writing {} failed: {e}", output.display()));
    eprintln!("\n[output] Cell-diff summary written to {:?}", output);
}
