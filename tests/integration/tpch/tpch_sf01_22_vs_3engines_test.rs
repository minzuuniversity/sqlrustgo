//! TPC-H 22/22 wire-protocol (sqlrustgo) vs MariaDB (mysql CLI) +
//! PostgreSQL (psql CLI) on the SF=0.01 fixture.

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::start_sf01;
use std::collections::HashSet;
use std::fs;
use std::process::Command;
use std::sync::Mutex;

static SERVER_LOCK: Mutex<()> = Mutex::new(());

const QUERIES_DIR: &str = "queries";

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

fn normalize_md_output(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            l.split('\t')
                .map(|c| {
                    if let Ok(f) = c.parse::<f64>() {
                        if f == f.trunc() && c.contains('.') {
                            format!("{}", f as i64)
                        } else {
                            format!("{:.4}", f)
                        }
                    } else {
                        c.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect()
}

#[test]
fn tpch_sf01_22_vs_mariadb_cell() {
    let _guard = SERVER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if Command::new("mysql")
        .arg("-e")
        .arg("SELECT 1")
        .output()
        .is_err()
    {
        eprintln!("MariaDB not available, skipping");
        return;
    }
    let mut client = start_sf01();
    eprintln!();
    eprintln!("=== Cell-level comparison sqlrustgo (wire) vs MariaDB on 22 queries ===");
    let mut pass = 0;
    let mut fail = 0;
    for n in 1..=22usize {
        let sql = fs::read_to_string(format!("{}/q{}.sql", QUERIES_DIR, n)).unwrap();
        let t0 = std::time::Instant::now();
        let sr_rows = client.query_rows(&sql).expect("query");
        let elapsed = t0.elapsed();
        let sr_strings: Vec<String> = sr_rows.iter().map(|row| row.join("|")).collect();
        let sr_set: HashSet<String> = sr_strings.iter().cloned().collect();
        let md_count = run_md_count(&sql);
        let md_result = run_md(&sql);
        let md_strings = match md_result {
            Ok(s) => normalize_md_output(&s),
            Err(_) => vec![],
        };
        let md_set: HashSet<String> = md_strings.iter().cloned().collect();
        let cell_match = sr_set == md_set;
        let sr_rc = sr_rows.len();
        let status = if sr_rc == md_count && cell_match {
            pass += 1;
            "PASS"
        } else {
            fail += 1;
            "FAIL"
        };
        let diff_info = if !cell_match && sr_rc == md_count {
            format!(
                " [{} rows differ]",
                sr_set.symmetric_difference(&md_set).count()
            )
        } else if sr_rc != md_count {
            format!(" [rc sr={} md={}]", sr_rc, md_count)
        } else {
            "".to_string()
        };
        eprintln!(
            "  Q{:2}: {} (rc={}, md={}{}) in {:?}",
            n, status, sr_rc, md_count, diff_info, elapsed
        );
    }
    eprintln!();
    eprintln!("=== Summary: pass={} fail={} ===", pass, fail);
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

fn normalize_pg_output(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| {
            !l.is_empty() && !l.starts_with("---") && !l.starts_with("(") && !l.contains("?column?")
        })
        .map(|l| {
            l.split('|')
                .map(|c| {
                    if let Ok(f) = c.parse::<f64>() {
                        if f == f.trunc() {
                            format!("{}", f as i64)
                        } else {
                            format!("{:.4}", f)
                        }
                    } else {
                        c.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect()
}

fn cell_match_with_fp_tol(a: &[String], b: &[String]) -> bool {
    for (ri, (ra, rb)) in a.iter().zip(b.iter()).enumerate() {
        let ac: Vec<&str> = ra.split('|').collect();
        let bc: Vec<&str> = rb.split('|').collect();
        if ac.len() != bc.len() {
            eprintln!(
                "        row {} col count differ: {} vs {}",
                ri,
                ac.len(),
                bc.len()
            );
            return false;
        }
        for (ci, (x, y)) in ac.iter().zip(bc.iter()).enumerate() {
            if x == y {
                continue;
            }
            let (xf, yf) = match (x.parse::<f64>(), y.parse::<f64>()) {
                (Ok(a), Ok(b)) => (a, b),
                _ => {
                    eprintln!("        row {} col {} not numeric: {} vs {}", ri, ci, x, y);
                    return false;
                }
            };
            let abs = (xf - yf).abs();
            let rel = if yf != 0.0 { abs / yf.abs() } else { abs };
            if abs > 1e-3 && rel > 1e-5 {
                eprintln!(
                    "        row {} col {} FAIL: engine={} pg={} abs={:.6} rel={:.9}",
                    ri, ci, x, y, abs, rel
                );
                return false;
            }
        }
    }
    true
}

#[test]
fn tpch_sf01_22_vs_postgresql_pgdate_cell() {
    let _guard = SERVER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let mut client = start_sf01();
    eprintln!();
    eprintln!("=== Cell-level comparison sqlrustgo (wire) vs PostgreSQL on 22 queries ===");
    let mut pass = 0;
    let mut fail = 0;
    for n in 1..=22usize {
        let sql = fs::read_to_string(format!("{}/q{}.sql", QUERIES_DIR, n)).unwrap();
        let t0 = std::time::Instant::now();
        let sr_rows = client.query_rows(&sql).expect("query");
        let elapsed = t0.elapsed();
        let sr_strings: Vec<String> = sr_rows.iter().map(|r| r.join("|")).collect();
        let sr_rc = sr_rows.len();
        let pg_count = run_pg_count(&sql);
        let pg_result = run_pg(&sql);
        let pg_strings = match pg_result {
            Ok(s) => normalize_pg_output(&s),
            Err(_) => vec![],
        };
        let cell_match = if sr_rc == pg_count {
            cell_match_with_fp_tol(&sr_strings, &pg_strings)
        } else if sr_rc == 1 && pg_count == 0 {
            sr_strings
                .iter()
                .all(|r| r.split('|').all(|c| c == "NULL" || c == "null"))
        } else {
            false
        };
        let status = if cell_match {
            pass += 1;
            "PASS"
        } else {
            fail += 1;
            "FAIL"
        };
        let diff_info = if !cell_match {
            if sr_rc != pg_count && !(sr_rc == 1 && pg_count == 0) {
                format!(" [rc sr={} pg={}]", sr_rc, pg_count)
            } else {
                " [cell differ]".to_string()
            }
        } else {
            "".to_string()
        };
        eprintln!(
            "  Q{:2}: {} (rc={}, pg={}{}) in {:?}",
            n, status, sr_rc, pg_count, diff_info, elapsed
        );
    }
    eprintln!();
    eprintln!("=== Summary: pass={} fail={} ===", pass, fail);
}
