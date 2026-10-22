//! G17 4-Way TPC-H Horizontal Comparison — Main Test
//!
//! Runs all 22 TPC-H queries on 4 engines (sqlrustgo, SQLite, MariaDB, PostgreSQL)
//! with the same data set, then compares row_count and sample rows.
//!
//! Skip with `SKIP_FOUR_WAY=1` (e.g. when external DBs are not available).
//!
//! Refs: V390_TEST_PLAN_ROUND2_REVIEW §G17
//!       docs/discovery/2026-06-05-tpch-22-mysql-server-comprehensive-report.md
//!
//! Output: docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md (regenerated each run)

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod four_way_harness;

// The shared TPC-H fixture loader (issue #5379). Loading through the
// storage layer in batches instead of executing one INSERT statement per
// row is ~2 orders of magnitude faster; see the comment at the load site.
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;
use four_way_harness::{setup_external_db, Engine, QueryResult, TABLE_COLS};

/// Cargo target name — used in the NOT-RUN marker.
const TARGET: &str = "four_way_compare_test";

/// One entry per (engine, query) produced by the harness.
#[derive(Debug, Default)]
struct FourWayResults {
    by_engine: BTreeMap<Engine, Vec<QueryResult>>, // 22 queries per engine
    durations_ms: BTreeMap<Engine, u128>,
}

#[test]
fn test_four_way_tpch_22() {
    // #5500: an explicit skip must be **disclosed**, not silently green.
    if std::env::var("SKIP_FOUR_WAY").is_ok() {
        tpch_fixture::report_not_run(
            TARGET,
            tpch_fixture::reason::EXTERNAL_SERVICE_UNAVAILABLE,
            "SKIP_FOUR_WAY=1 is set: the 4-way comparison was NOT executed",
        );
        return;
    }

    // The fixture is resolved through the shared resolver (#5379). It used
    // to fall back to a hardcoded `/tmp/tpch_sf01`, a directory that exists
    // only on the machine where dbgen happened to run — on every other
    // machine the target skipped itself and reported success.
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        return;
    };
    let data_dir = fixture.dir.clone();

    eprintln!("\n=== G17 4-Way TPC-H Horizontal Comparison ===");
    eprintln!("Data dir: {:?}", data_dir);
    eprintln!("Comparing: sqlrustgo, SQLite, MariaDB, PostgreSQL\n");

    let mut results = FourWayResults::default();

    // ---- 1. Setup external DBs (MariaDB + PostgreSQL) ----
    // An external client that is **not installed** is an unavailable
    // service, not a failed comparison: the engine is disclosed as
    // NOT-RUN and simply does not take part.
    let mariadb_available = four_way_harness::external_client_available(Engine::MariaDb);
    let postgres_available = four_way_harness::external_client_available(Engine::PostgreSql);
    for (engine, available, env_var) in [
        (Engine::MariaDb, mariadb_available, "SQLRUSTGO_MYSQL_BIN"),
        (Engine::PostgreSql, postgres_available, "SQLRUSTGO_PSQL_BIN"),
    ] {
        if available {
            continue;
        }
        tpch_fixture::report_not_run(
            TARGET,
            tpch_fixture::reason::EXTERNAL_SERVICE_UNAVAILABLE,
            &format!(
                "the {} client is not installed / not on PATH — set {env_var} to its \
                 absolute path to include this engine in the comparison",
                engine.name()
            ),
        );
    }

    if mariadb_available {
        eprintln!("[setup] MariaDB...");
        if let Err(e) = setup_external_db(Engine::MariaDb, &data_dir) {
            eprintln!("[setup] MariaDB error: {}", e);
        }
    }
    if postgres_available {
        eprintln!("[setup] PostgreSQL...");
        if let Err(e) = setup_external_db(Engine::PostgreSql, &data_dir) {
            eprintln!("[setup] PostgreSQL error: {}", e);
        }
    }

    // ---- 2. Read 22 query SQL files ----
    let query_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("queries");
    let mut queries: Vec<(u8, String)> = Vec::new();
    for q in 1..=22u8 {
        let path = query_dir.join(format!("q{}.sql", q));
        if let Ok(s) = std::fs::read_to_string(&path) {
            queries.push((q, s.trim().trim_end_matches(';').to_string()));
        }
    }
    eprintln!(
        "[queries] Loaded {} queries from {:?}",
        queries.len(),
        query_dir
    );

    // ---- 3. Run on sqlrustgo (in-process) ----
    eprintln!("\n[sqlrustgo] Running 22 queries...");
    let start = Instant::now();
    results
        .by_engine
        .insert(Engine::SqlRustGo, run_sqlrustgo(&queries, &data_dir));
    results
        .durations_ms
        .insert(Engine::SqlRustGo, start.elapsed().as_millis());

    // ---- 4. Run on SQLite (in-process rusqlite) ----
    eprintln!("[sqlite] Running 22 queries...");
    let start = Instant::now();
    results
        .by_engine
        .insert(Engine::Sqlite, run_sqlite(&queries, &data_dir));
    results
        .durations_ms
        .insert(Engine::Sqlite, start.elapsed().as_millis());

    // ---- 5. Run on MariaDB (subprocess mysql) ----
    if mariadb_available {
        eprintln!("[mariadb] Running 22 queries...");
        let start = Instant::now();
        results
            .by_engine
            .insert(Engine::MariaDb, run_mariadb(&queries));
        results
            .durations_ms
            .insert(Engine::MariaDb, start.elapsed().as_millis());
    }

    // ---- 6. Run on PostgreSQL (subprocess psql) ----
    if postgres_available {
        eprintln!("[postgresql] Running 22 queries...");
        let start = Instant::now();
        results
            .by_engine
            .insert(Engine::PostgreSql, run_postgresql(&queries));
        results
            .durations_ms
            .insert(Engine::PostgreSql, start.elapsed().as_millis());
    }

    // ---- 7. Compare + report ----
    print_summary(&results);
    write_report(&results);

    // ---- 8. Assert: 22/22 PASS for sqlrustgo (regression check) ----
    let sqlrustgo = results.by_engine.get(&Engine::SqlRustGo).unwrap();
    let pass = sqlrustgo.iter().filter(|r| r.passed()).count();
    assert_eq!(pass, 22, "sqlrustgo should pass 22/22 (got {})", pass);
}

// ============================================================================
// Per-engine runners
// ============================================================================

fn run_sqlrustgo(queries: &[(u8, String)], data_dir: &Path) -> Vec<QueryResult> {
    use parking_lot::RwLock;
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
    use std::sync::Arc;

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(Arc::clone(&storage));

    // Apply DDL via engine.execute (same as tpch_full_22_test)
    for ddl in four_way_harness::tpc_h_schema(Engine::SqlRustGo) {
        if let Err(e) = engine.execute(&ddl) {
            eprintln!("[sqlrustgo] DDL failed: {} - {}", ddl, e);
        }
    }

    // Load data via batch insert
    for (table, cols) in TABLE_COLS {
        let path = data_dir.join(format!("{}.tbl", table));
        // Batch-load through the storage layer instead of executing one
        // INSERT statement per row: the statement path issues ~86,000
        // INSERTs per load and costs ~1 hour per load in a debug build.
        // Before issue #5379 the fixture was a dangling symlink, so this
        // target skipped the load and finished in milliseconds — the cost
        // was invisible. With the fixture now generated at run time and
        // reachable, the statement path would need ~2 hours of CI time and
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

    let mut out = Vec::new();
    for (q, sql) in queries {
        let start = Instant::now();
        let res = engine.execute(sql);
        let dur = start.elapsed().as_millis();
        let mut qr = QueryResult {
            engine: Engine::SqlRustGo,
            query: *q,
            row_count: 0,
            duration_ms: dur,
            error: None,
            sample_rows: vec![],
        };
        match res {
            Ok(r) => {
                qr.row_count = r.rows.len();
                let mut rows: Vec<String> = r
                    .rows
                    .iter()
                    .take(5)
                    .map(|row| {
                        row.iter()
                            .map(|v| format!("{:?}", v))
                            .collect::<Vec<_>>()
                            .join("|")
                    })
                    .collect();
                rows.sort();
                qr.sample_rows = rows;
            }
            Err(e) => qr.error = Some(format!("{}", e)),
        }
        out.push(qr);
    }
    out
}

fn run_sqlite(queries: &[(u8, String)], data_dir: &Path) -> Vec<QueryResult> {
    use rusqlite::Connection;
    let conn = Connection::open_in_memory().expect("sqlite open");
    for ddl in four_way_harness::tpc_h_schema(Engine::Sqlite) {
        let _ = conn.execute(&ddl, []);
    }
    for (table, cols) in TABLE_COLS {
        let path = data_dir.join(format!("{}.tbl", table));
        let inserts = four_way_harness::load_tbl_inserts(table, *cols, &path, Engine::Sqlite);
        for ins in &inserts {
            // Replace INSERT with sqlite-compatible (the harness already uses single-quoted strings)
            let _ = conn.execute(ins, []);
        }
    }
    let mut out = Vec::new();
    for (q, sql) in queries {
        let start = Instant::now();
        let mut stmt = conn.prepare(sql);
        let res = stmt.as_mut().map(|s| s.query([]));
        let dur = start.elapsed().as_millis();
        let mut qr = QueryResult {
            engine: Engine::Sqlite,
            query: *q,
            row_count: 0,
            duration_ms: dur,
            error: None,
            sample_rows: vec![],
        };
        match res {
            Ok(Ok(mut rows)) => {
                let mut all_rows: Vec<Vec<String>> = Vec::new();
                while let Some(row) = rows.next().unwrap_or(None) {
                    let mut vals: Vec<String> = Vec::new();
                    for i in 0.. {
                        let v: rusqlite::types::Value = match row.get(i) {
                            Ok(v) => v,
                            Err(_) => break,
                        };
                        vals.push(format!("{:?}", v));
                    }
                    all_rows.push(vals);
                }
                qr.row_count = all_rows.len();
                let mut sample: Vec<String> =
                    all_rows.into_iter().take(5).map(|r| r.join("|")).collect();
                sample.sort();
                qr.sample_rows = sample;
            }
            Ok(Err(e)) => qr.error = Some(format!("{}", e)),
            Err(e) => qr.error = Some(format!("prepare: {}", e)),
        }
        out.push(qr);
    }
    out
}

fn run_mariadb(queries: &[(u8, String)]) -> Vec<QueryResult> {
    let mut out = Vec::new();
    for (q, sql) in queries {
        let start = Instant::now();
        let res = four_way_harness::run_mysql_sql(sql);
        let dur = start.elapsed().as_millis();
        let mut qr = QueryResult {
            engine: Engine::MariaDb,
            query: *q,
            row_count: 0,
            duration_ms: dur,
            error: None,
            sample_rows: vec![],
        };
        match res {
            Ok(stdout) => {
                let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
                qr.row_count = lines.len();
                let mut sample: Vec<String> = lines.into_iter().take(5).map(String::from).collect();
                sample.sort();
                qr.sample_rows = sample;
            }
            // #5500: a failed query is a **failure**, not an empty result
            // set. It is recorded as an error so the summary shows ERR
            // instead of a plausible "0 rows".
            Err(e) => qr.error = Some(e.to_string()),
        }
        out.push(qr);
    }
    out
}

fn run_postgresql(queries: &[(u8, String)]) -> Vec<QueryResult> {
    let mut out = Vec::new();
    for (q, sql) in queries {
        let start = Instant::now();
        let res = four_way_harness::run_psql_sql(sql);
        let dur = start.elapsed().as_millis();
        let mut qr = QueryResult {
            engine: Engine::PostgreSql,
            query: *q,
            row_count: 0,
            duration_ms: dur,
            error: None,
            sample_rows: vec![],
        };
        match res {
            Ok(stdout) => {
                let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
                qr.row_count = lines.len();
                let mut sample: Vec<String> = lines.into_iter().take(5).map(String::from).collect();
                sample.sort();
                qr.sample_rows = sample;
            }
            // #5500: see `run_mariadb`.
            Err(e) => qr.error = Some(e.to_string()),
        }
        out.push(qr);
    }
    out
}

// ============================================================================
// Reporting
// ============================================================================

fn print_summary(results: &FourWayResults) {
    eprintln!("\n=== 4-Way TPC-H Summary ===\n");
    eprintln!(
        "{:>4} | {:>10} {:>10} {:>10} {:>10} | row_count_match",
        "Q", "sqlrustgo", "sqlite", "mariadb", "postgresql"
    );
    eprintln!("{}", "-".repeat(80));
    for q in 1..=22u8 {
        let row = |e: Engine| {
            results
                .by_engine
                .get(&e)
                .and_then(|v| v.iter().find(|r| r.query == q))
        };
        let cell = |e: Engine| -> String {
            match row(e) {
                Some(r) if r.passed() => format!("{} ({:.0}ms)", r.row_count, r.duration_ms as f64),
                Some(r) => format!(
                    "ERR: {}",
                    r.error
                        .as_deref()
                        .unwrap_or("?")
                        .chars()
                        .take(30)
                        .collect::<String>()
                ),
                None => "?".to_string(),
            }
        };
        let counts: Vec<usize> = Engine::all()
            .iter()
            .filter_map(|e| row(*e).and_then(|r| if r.passed() { Some(r.row_count) } else { None }))
            .collect();
        let match_str = if counts.is_empty() {
            "—"
        } else {
            let first = counts[0];
            if counts.iter().all(|c| *c == first) {
                "✓"
            } else {
                "✗ MISMATCH"
            }
        };
        eprintln!(
            "{:>4} | {:>10} {:>10} {:>10} {:>10} | {}",
            q,
            cell(Engine::SqlRustGo),
            cell(Engine::Sqlite),
            cell(Engine::MariaDb),
            cell(Engine::PostgreSql),
            match_str
        );
    }
    eprintln!("\n=== Per-engine total time ===");
    for e in Engine::all() {
        if let Some(d) = results.durations_ms.get(&e) {
            eprintln!("  {}: {:.2}s", e, *d as f64 / 1000.0);
        }
    }
}

/// Where the comparison report is written.
///
/// It used to overwrite `docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md`,
/// a file **tracked in the repository** — so every run of this target
/// dirtied the working tree (the same family as issue #5389, where the A04
/// matrix test rewrote 81 evidence files, and as #5404, where the cell-diff
/// target did the same). The report is an artefact of a run, not a source
/// file, so it now goes to the build directory unless `TPCH_FOUR_WAY_OUT`
/// says otherwise.
/// The cargo target directory, resolved at **runtime**.
///
/// `env!("CARGO_TARGET_DIR")` is a *compile-time* macro. When cargo is not
/// invoked with `CARGO_TARGET_DIR` set — which is the case on CI — the
/// macro expands to a compile error ("environment variable
/// `CARGO_TARGET_DIR` not defined at compile time") and the **whole test
/// target fails to build**. That is what made the CI `lint-build` job red:
/// `cargo clippy --all-targets` compiles this test target, and the error
/// is a hard build failure, not a lint.
///
/// The path has to be resolved when the test runs, not when it is compiled.
fn cargo_target_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    // `option_env!` is the compile-time-optional variant: it yields the
    // value when cargo *was* invoked with the variable set.
    if let Some(dir) = option_env!("CARGO_TARGET_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    // The default: cargo puts the artifacts in `<package root>/target`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")
}

/// Report file location. `TPCH_FOUR_WAY_OUT` overrides it; otherwise the
/// report lands next to the build artifacts.
fn report_path() -> PathBuf {
    if let Ok(p) = std::env::var("TPCH_FOUR_WAY_OUT") {
        if !p.trim().is_empty() {
            return PathBuf::from(p.trim());
        }
    }
    cargo_target_dir().join("four-way-tpch-report.md")
}

fn write_report(results: &FourWayResults) {
    let report_path = report_path();
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut out = String::new();
    out.push_str("# G17 4-Way TPC-H Comparison Report (v3.9.0)\n\n");
    out.push_str("> **Date**: 2026-06-06  \n");
    out.push_str("> **Test**: tests/four_way_compare_test.rs  \n");
    out.push_str("> **Data**: SF=1 (simplified — 1500/15000/60000 + 5/25/100/2000/8000)  \n");
    out.push_str("> **Engines**: sqlrustgo, SQLite, MariaDB, PostgreSQL  \n\n");
    out.push_str("## Per-Query Results\n\n");
    out.push_str("| Q | sqlrustgo (rows,ms) | SQLite (rows,ms) | MariaDB (rows,ms) | PostgreSQL (rows,ms) | Row count match |\n");
    out.push_str("|---|---------------------|------------------|-------------------|----------------------|------------------|\n");
    for q in 1..=22u8 {
        let row = |e: Engine| {
            results
                .by_engine
                .get(&e)
                .and_then(|v| v.iter().find(|r| r.query == q))
        };
        let cell = |e: Engine| -> String {
            match row(e) {
                Some(r) if r.passed() => format!("{} / {}", r.row_count, r.duration_ms),
                Some(r) => format!(
                    "ERR: {}",
                    r.error
                        .as_deref()
                        .unwrap_or("?")
                        .chars()
                        .take(50)
                        .collect::<String>()
                ),
                None => "?".to_string(),
            }
        };
        let counts: Vec<usize> = Engine::all()
            .iter()
            .filter_map(|e| row(*e).and_then(|r| if r.passed() { Some(r.row_count) } else { None }))
            .collect();
        let match_str = if counts.is_empty() {
            "—"
        } else {
            let first = counts[0];
            if counts.iter().all(|c| *c == first) {
                "✓ all match"
            } else {
                "✗ MISMATCH"
            }
        };
        out.push_str(&format!(
            "| Q{:02} | {} | {} | {} | {} | {} |\n",
            q,
            cell(Engine::SqlRustGo),
            cell(Engine::Sqlite),
            cell(Engine::MariaDb),
            cell(Engine::PostgreSql),
            match_str,
        ));
    }
    out.push_str("\n## Per-Engine Total Time\n\n");
    for e in Engine::all() {
        if let Some(d) = results.durations_ms.get(&e) {
            let pass = results
                .by_engine
                .get(&e)
                .map(|v| v.iter().filter(|r| r.passed()).count())
                .unwrap_or(0);
            out.push_str(&format!(
                "- **{}**: {}/22 PASS, {:.2}s total\n",
                e,
                pass,
                *d as f64 / 1000.0
            ));
        }
    }

    if let Err(e) = std::fs::write(&report_path, out) {
        eprintln!("[report] write failed: {}", e);
    } else {
        eprintln!("\n[report] Written to {:?}", report_path);
    }
}
