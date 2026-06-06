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
use std::path::PathBuf;
use std::time::Instant;

mod four_way_harness;
use four_way_harness::{compare_row_counts, setup_external_db, Engine, QueryResult, TABLE_COLS, default_data_dir};

/// One entry per (engine, query) produced by the harness.
#[derive(Debug, Default)]
struct FourWayResults {
    by_engine: BTreeMap<Engine, Vec<QueryResult>>, // 22 queries per engine
    durations_ms: BTreeMap<Engine, u128>,
}

#[test]
fn test_four_way_tpch_22() {
    if std::env::var("SKIP_FOUR_WAY").is_ok() {
        eprintln!("\n=== G17 4-Way TPC-H [SKIPPED via SKIP_FOUR_WAY=1] ===");
        return;
    }

    let data_dir = default_data_dir();
    if !data_dir.exists() {
        eprintln!("\n=== G17 4-Way TPC-H [SKIPPED — TPCH_DATA_DIR={:?} not found] ===", data_dir);
        return;
    }

    eprintln!("\n=== G17 4-Way TPC-H Horizontal Comparison ===");
    eprintln!("Data dir: {:?}", data_dir);
    eprintln!("Comparing: sqlrustgo, SQLite, MariaDB, PostgreSQL\n");

    let mut results = FourWayResults::default();

    // ---- 1. Setup external DBs (MariaDB + PostgreSQL) ----
    eprintln!("[setup] MariaDB...");
    if let Err(e) = setup_external_db(Engine::MariaDb, &data_dir) {
        eprintln!("[setup] MariaDB error: {}", e);
    }
    eprintln!("[setup] PostgreSQL...");
    if let Err(e) = setup_external_db(Engine::PostgreSql, &data_dir) {
        eprintln!("[setup] PostgreSQL error: {}", e);
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
    eprintln!("[queries] Loaded {} queries from {:?}", queries.len(), query_dir);

    // ---- 3. Run on sqlrustgo (in-process) ----
    eprintln!("\n[sqlrustgo] Running 22 queries...");
    let start = Instant::now();
    results.by_engine.insert(Engine::SqlRustGo, run_sqlrustgo(&queries, &data_dir));
    results.durations_ms.insert(Engine::SqlRustGo, start.elapsed().as_millis());

    // ---- 4. Run on SQLite (in-process rusqlite) ----
    eprintln!("[sqlite] Running 22 queries...");
    let start = Instant::now();
    results.by_engine.insert(Engine::Sqlite, run_sqlite(&queries, &data_dir));
    results.durations_ms.insert(Engine::Sqlite, start.elapsed().as_millis());

    // ---- 5. Run on MariaDB (subprocess mysql) ----
    eprintln!("[mariadb] Running 22 queries...");
    let start = Instant::now();
    results.by_engine.insert(Engine::MariaDb, run_mariadb(&queries));
    results.durations_ms.insert(Engine::MariaDb, start.elapsed().as_millis());

    // ---- 6. Run on PostgreSQL (subprocess psql) ----
    eprintln!("[postgresql] Running 22 queries...");
    let start = Instant::now();
    results.by_engine.insert(Engine::PostgreSql, run_postgresql(&queries));
    results.durations_ms.insert(Engine::PostgreSql, start.elapsed().as_millis());

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

fn run_sqlrustgo(queries: &[(u8, String)], data_dir: &PathBuf) -> Vec<QueryResult> {
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
    use std::sync::{Arc, RwLock};

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
        let inserts = four_way_harness::load_tbl_inserts(table, *cols, &path, Engine::SqlRustGo);
        for ins in &inserts {
            let _ = engine.execute(ins);
        }
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

fn run_sqlite(queries: &[(u8, String)], data_dir: &PathBuf) -> Vec<QueryResult> {
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
                let mut sample: Vec<String> = all_rows
                    .into_iter()
                    .take(5)
                    .map(|r| r.join("|"))
                    .collect();
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
            Err(e) => qr.error = Some(e),
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
            Err(e) => qr.error = Some(e),
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
                Some(r) => format!("ERR: {}", r.error.as_deref().unwrap_or("?").chars().take(30).collect::<String>()),
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

fn write_report(results: &FourWayResults) {
    // Resolve the actual workspace root (CARGO_MANIFEST_DIR for the
    // workspace root crate is the current dir; for a sub-crate it's
    // the sub-crate dir, hence the .. walk).
    let report_path = if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = std::path::PathBuf::from(manifest);
        // Walk up until we find the workspace root (has Cargo.toml with [workspace])
        let mut cur = p.as_path();
        loop {
            if cur.join("Cargo.toml").exists()
                && std::fs::read_to_string(cur.join("Cargo.toml"))
                    .map(|s| s.contains("[workspace]"))
                    .unwrap_or(false)
            {
                break;
            }
            match cur.parent() {
                Some(p) => cur = p,
                None => break,
            }
        }
        cur.join("docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md")
    } else {
        std::path::PathBuf::from("docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md")
    };
    std::fs::create_dir_all(report_path.parent().unwrap()).ok();

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
            results.by_engine.get(&e).and_then(|v| v.iter().find(|r| r.query == q))
        };
        let cell = |e: Engine| -> String {
            match row(e) {
                Some(r) if r.passed() => format!("{} / {}", r.row_count, r.duration_ms),
                Some(r) => format!("ERR: {}", r.error.as_deref().unwrap_or("?").chars().take(50).collect::<String>()),
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
            out.push_str(&format!("- **{}**: {}/22 PASS, {:.2}s total\n", e, pass, *d as f64 / 1000.0));
        }
    }

    if let Err(e) = std::fs::write(&report_path, out) {
        eprintln!("[report] write failed: {}", e);
    } else {
        eprintln!("\n[report] Written to {:?}", report_path);
    }
}
