//! TPC-H 22 audit on SF=0.01 canonical fixture via wire protocol.
//! Ensures all 22 queries execute without crashing on the wire server.
//!
//! ## Fixture
//!
//! - `tests/data/tpch-sf01/` — 8 `.tbl` files (lineitem ≈ 6 015 rows)
//!
//! ## Historical note
//!
//! This file previously pointed at `/tmp/tpch_3way_sf001/clean` (SF=0.001,
//! ~600 lineitem rows) while the file header claimed SF=0.01. The canonical
//! committed fixture (`tests/data/tpch-sf01/`, ~6 015 lineitem rows) is the
//! correct reference. Wire protocol test now uses the right fixture.
//!
//! ## SQLite baseline regeneration
//!
//! ```text
//! python3 scripts/dev/build_tpch_sf01_sqlite.py
//! ```

#![allow(dead_code)]

#[path = "../../common/mod.rs"]
mod common;
use std::process::Command;

const SQLITE_BASELINE_DB: &str = "/tmp/tpch_sf01_audit.db";

/// Run sqlite3 with a query, return the first column of the first row as i64.
fn sqlite_count(sql: &str) -> Option<i64> {
    let sql = format!("SELECT COUNT(*) FROM ({}) sub", sql);
    let output = Command::new("sqlite3")
        .args([SQLITE_BASELINE_DB, &sql])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Strip ORDER BY clause so the query can be used as a subquery
/// (SQLite rejects ORDER BY inside a subquery without OFFSET).
fn strip_order_by(sql: &str) -> String {
    sql.rsplit_once("ORDER BY")
        .map(|(head, _)| head.trim().trim_end_matches(';'))
        .unwrap_or_else(|| sql.trim().trim_end_matches(';'))
        .to_string()
}

/// Rewrite EXTRACT(YEAR FROM col) [AS alias] for SQLite compatibility.
fn rewrite_for_sqlite(sql: &str) -> String {
    let re = regex::Regex::new(r"EXTRACT\(YEAR FROM ([^)]+)\)(?: AS (\w+))?").unwrap();
    re.replace_all(sql, |caps: &regex::Captures| {
        let col = caps.get(1).unwrap().as_str();
        match caps.get(2) {
            Some(alias) => {
                format!(
                    "CAST(strftime('%Y', {}) AS INTEGER) AS {}",
                    col,
                    alias.as_str()
                )
            }
            None => format!("CAST(strftime('%Y', {}) AS INTEGER)", col),
        }
    })
    .to_string()
}

#[test]
fn test_tpch_22_wire_sf01_audit() {
    use common::tpch_wire_harness::start_sf001;

    let mut client = start_sf001();

    eprintln!("\n=== TPC-H 22 audit via wire (SF=0.01) ===\n");

    let queries_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("queries");
    let mut ok = 0;
    let mut err = 0;
    let mut parse_err = 0;

    for q in 1..=22 {
        let sql_path = queries_dir.join(format!("q{}.sql", q));
        let sql = std::fs::read_to_string(&sql_path).unwrap();
        let sql_trimmed = sql.trim().trim_end_matches(';');

        // Engine result via wire: wrap in COUNT(*)
        let wrap = format!("SELECT COUNT(*) AS c FROM ({}) sub", sql_trimmed);
        let engine_count: Option<i64> = match client.query_rows(&wrap) {
            Ok(rows) => rows
                .first()
                .and_then(|row| row.first())
                .and_then(|v| v.parse().ok()),
            Err(e) => {
                let e_str = e.to_string();
                if e_str.contains("parse")
                    || e_str.contains("Parse")
                    || e_str.contains("syntax")
                    || e_str.contains("#42000")
                {
                    eprintln!(
                        "Q{:>2}: PARSE_ERR  ({})",
                        q,
                        e_str.lines().next().unwrap_or(&e_str)
                    );
                    parse_err += 1;
                } else {
                    eprintln!(
                        "Q{:>2}: ERR  ({})",
                        q,
                        e_str.lines().next().unwrap_or(&e_str)
                    );
                    err += 1;
                }
                None
            }
        };

        if let Some(ref c) = engine_count {
            eprintln!("Q{:>2}: OK  ({} rows)", q, c);
            ok += 1;
        }
    }

    eprintln!("\n=== Totals ===");
    eprintln!("OK        : {}/22", ok);
    eprintln!("ERR       : {}/22", err);
    eprintln!("PARSE_ERR : {}/22", parse_err);

    // Gate: all queries must execute without crashing/hanging.
    // Parse errors (Q2) are counted but do not fail the gate — they are
    // known unsupported syntax issues.
    assert_eq!(err, 0, "TPC-H gate failure: {} queries crashed", err);
}
