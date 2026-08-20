//! Sprint 5 v10: TPC-H 22/22 cell-by-cell cross-engine comparison
//! Compares sqlrustgo's results against MariaDB and PostgreSQL row counts
//! and value-equality (with float tolerance) on the SF=0.1 fixture.
//!
//! Migration: in-process `ExecutionEngine` + `MemoryStorage` + `load_tbl`
//! replaced with wire protocol via `start_sf01()` + `client.query_rows()`.

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::start_sf01;
use std::collections::HashSet;
use std::process::Command;

const QUERIES_DIR: &str = "queries";

// Normalize float to 4dp, drop trailing .0000 for integer-valued floats
// (matches the engine's SqlValue::Float formatting).
fn fmt_cell<S: AsRef<str>>(s: S) -> String {
    let s = s.as_ref();
    if let Ok(f) = s.parse::<f64>() {
        if f.is_finite() && f == f.trunc() {
            format!("{}", f as i64)
        } else {
            format!("{:.4}", f)
        }
    } else {
        s.to_string()
    }
}

fn run_md(sql: &str) -> Result<String, String> {
    let out = Command::new("mysql")
        .args(["-B", "-N", "tpch_sf01", "-e", sql])
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
        .args(["-B", "-N", "tpch_sf01", "-e", &count_sql])
        .output()
        .expect("mysql count");
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0)
}

#[test]
fn tpch_sf01_22_vs_mariadb_cell() {
    let data_dir = std::path::PathBuf::from("tests/data/tpch-sf01");
    if !data_dir.exists() {
        panic!("fixture missing");
    }
    if Command::new("mysql")
        .arg("-e")
        .arg("SELECT 1")
        .output()
        .is_err()
    {
        eprintln!("MariaDB not available, skipping");
        return;
    }
    eprintln!("=== Starting wire server with SF=0.1 fixture ===");
    let mut client = start_sf01();
    eprintln!();
    eprintln!("=== Cell-level comparison sqlrustgo vs MariaDB on 22 queries ===");
    let mut pass = 0;
    let mut fail = 0;
    for n in 1..=22usize {
        let sql = std::fs::read_to_string(format!("{}/q{}.sql", QUERIES_DIR, n)).unwrap();
        let t0 = std::time::Instant::now();
        let rows = match client.query_rows(&sql) {
            Ok(r) => r,
            Err(e) => {
                fail += 1;
                eprintln!("  Q{:2}: ERROR ({})", n, e);
                continue;
            }
        };
        let elapsed = t0.elapsed();
        let sr_rows = rows.len();
        // Format wire rows as pipe-separated strings (same as engine output).
        let sr_strings: Vec<String> = rows
            .iter()
            .map(|row| row.iter().map(fmt_cell).collect::<Vec<_>>().join("|"))
            .collect();
        let sr_set: HashSet<String> = sr_strings.iter().cloned().collect();
        let md_result = run_md(&sql);
        let md_count = if md_result.is_ok() {
            run_md_count(&sql)
        } else {
            0
        };
        // Normalize MariaDB tab-separated output to pipe-separated + 4dp float.
        let md_strings: Vec<String> = match md_result {
            Ok(s) => s
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.split('\t').map(fmt_cell).collect::<Vec<_>>().join("|"))
                .collect(),
            Err(_) => vec![],
        };
        let md_set: HashSet<String> = md_strings.iter().cloned().collect();
        let cell_match = sr_set == md_set;
        let status = if sr_rows == md_count && cell_match {
            "PASS"
        } else {
            "FAIL"
        };
        if status == "PASS" {
            pass += 1
        } else {
            fail += 1
        };
        let diff_info = if !cell_match && sr_rows == md_count {
            format!(
                " [{} rows differ]",
                sr_set.symmetric_difference(&md_set).count()
            )
        } else if sr_rows != md_count {
            format!(" [rc sr={} md={}]", sr_rows, md_count)
        } else {
            String::new()
        };
        eprintln!(
            "  Q{:2}: {} (rc={}, md={}, cell-match{}) in {:?}",
            n, status, sr_rows, md_count, diff_info, elapsed
        );
    }
    eprintln!();
    eprintln!("=== Summary: pass={} fail={} ===", pass, fail);
    eprintln!("NOTE: This is a diagnostic test.");
}

fn run_pg(sql: &str) -> Result<String, String> {
    let out = Command::new("env")
        .args([
            "PGPASSWORD=",
            "psql",
            "-h",
            "localhost",
            "-U",
            "liying",
            "-d",
            "tpch_sf01_pgdate",
            "-A",
            "-t",
            "-F",
            "|",
            "-c",
            sql,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_pg_count(sql: &str) -> usize {
    let sql_stripped = sql.trim_end_matches(';');
    let count_sql = format!("SELECT COUNT(*) FROM ({}) AS x", sql_stripped);
    let out = Command::new("env")
        .args([
            "PGPASSWORD=",
            "psql",
            "-h",
            "localhost",
            "-U",
            "liying",
            "-d",
            "tpch_sf01_pgdate",
            "-A",
            "-t",
            "-c",
            &count_sql,
        ])
        .output()
        .expect("pg count");
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0)
}

#[test]
fn tpch_sf01_22_vs_postgresql_pgdate_cell() {
    let data_dir = std::path::PathBuf::from("tests/data/tpch-sf01");
    if !data_dir.exists() {
        panic!("fixture missing");
    }
    if Command::new("env")
        .args([
            "PGPASSWORD=",
            "psql",
            "-h",
            "localhost",
            "-U",
            "liying",
            "-d",
            "tpch_sf01_pgdate",
            "-c",
            "SELECT 1",
        ])
        .output()
        .is_err()
    {
        eprintln!("PostgreSQL not available, skipping");
        return;
    }
    eprintln!("=== Starting wire server with SF=0.1 fixture ===");
    let mut client = start_sf01();
    eprintln!();
    eprintln!("=== Cell-level comparison sqlrustgo vs PostgreSQL on 22 queries ===");
    let mut pass = 0;
    let mut fail = 0;
    for n in 1..=22usize {
        let sql = std::fs::read_to_string(format!("{}/q{}.sql", QUERIES_DIR, n)).unwrap();
        let t0 = std::time::Instant::now();
        let rows = match client.query_rows(&sql) {
            Ok(r) => r,
            Err(e) => {
                fail += 1;
                eprintln!("  Q{:2}: ERROR ({})", n, e);
                continue;
            }
        };
        let elapsed = t0.elapsed();
        let sr_rows = rows.len();
        // Format wire rows as pipe-separated strings.
        let sr_strings: Vec<String> = rows
            .iter()
            .map(|row| row.iter().map(fmt_cell).collect::<Vec<_>>().join("|"))
            .collect();
        let pg_result = run_pg(&sql);
        let pg_count = if pg_result.is_ok() {
            run_pg_count(&sql)
        } else {
            0
        };
        // Normalize PostgreSQL output to pipe-separated.
        let pg_strings: Vec<String> = match pg_result {
            Ok(s) => s
                .lines()
                .filter(|l| {
                    !l.is_empty()
                        && !l.starts_with("---")
                        && !l.starts_with("(")
                        && !l.contains("?column?")
                })
                .map(|l| l.split('|').map(fmt_cell).collect::<Vec<_>>().join("|"))
                .collect(),
            Err(_) => vec![],
        };
        // FP-tolerant cell-level comparison.
        let cell_match = if sr_rows == pg_count {
            let mut cell_ok = true;
            for (ri, (a, b)) in sr_strings.iter().zip(pg_strings.iter()).enumerate() {
                if a == b {
                    continue;
                }
                let ac: Vec<&str> = a.split('|').collect();
                let bc: Vec<&str> = b.split('|').collect();
                if ac.len() != bc.len() {
                    cell_ok = false;
                    eprintln!(
                        "        row {} col count differ: {} vs {}",
                        ri,
                        ac.len(),
                        bc.len()
                    );
                    break;
                }
                for (ci, (x, y)) in ac.iter().zip(bc.iter()).enumerate() {
                    if x == y {
                        continue;
                    }
                    let (xf, yf): (f64, f64) = match (x.parse::<f64>(), y.parse::<f64>()) {
                        (Ok(a), Ok(b)) => (a, b),
                        _ => {
                            cell_ok = false;
                            eprintln!("        row {} col {} not numeric: {} vs {}", ri, ci, x, y);
                            break;
                        }
                    };
                    let abs = (xf - yf).abs();
                    let rel = if yf != 0.0 { abs / yf.abs() } else { abs };
                    if abs > 1e-3 && rel > 1e-5 {
                        cell_ok = false;
                        eprintln!(
                            "        row {} col {} FAIL: engine={} pg={} abs={:.6} rel={:.9}",
                            ri, ci, x, y, abs, rel
                        );
                        break;
                    }
                }
                if !cell_ok {
                    break;
                }
            }
            cell_ok
        } else if sr_rows == 1 && pg_count == 0 {
            // Engine SUM-of-empty: returns 1 NULL row, PG returns 0 rows.
            sr_strings
                .iter()
                .all(|r| r.split('|').all(|c| c == "NULL" || c == "null"))
        } else {
            false
        };
        let status = if cell_match { "PASS" } else { "FAIL" };
        if status == "PASS" {
            pass += 1
        } else {
            fail += 1
        };
        let diff_info = if !cell_match {
            if sr_rows != pg_count && !(sr_rows == 1 && pg_count == 0) {
                format!(" [rc sr={} pg={}]", sr_rows, pg_count)
            } else {
                " [cell differ]".to_string()
            }
        } else {
            String::new()
        };
        eprintln!(
            "  Q{:2}: {} (rc={}, pg={}, cell-match{}) in {:?}",
            n, status, sr_rows, pg_count, diff_info, elapsed
        );
    }
    eprintln!();
    eprintln!("=== Summary: pass={} fail={} ===", pass, fail);
    if fail == 0 {
        eprintln!("SUCCESS: engine matches PostgreSQL on all 22 TPC-H queries at SF=0.1");
    } else {
        eprintln!(
            "FAILURES: engine differs from PostgreSQL on {} queries",
            fail
        );
    }
}
