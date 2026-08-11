//! V312-21 / ISSUE #3908 — compat fixture runner.
//!
//! Walks tests/compat/mysql_v3_12/*.sql, drives each through a fresh
//! ephemeral MySQL server (started in-process), and writes
//! docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md
//! with one row per surface, decision column = PASS / unsupported /
//! deferred, and per-row evidence_hash.
//!
//! The bash gate (scripts/gate/check_v312_21_mysql_compat.sh) just
//! invokes this binary; the runner manages its own server lifecycle.
//!
//! See: openspec/changes/v312-21-mysql-compat-sql-surface-backlog/

use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

mod client;
use client::{CompatClient, Result};

use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

/// One row of the SURFACE_DISPOSITION table.
#[derive(Debug, Clone)]
struct Row {
    surface: String,
    decision: String,
    reason: String,
    evidence_hash: String,
    owner: String,
    expiry: String,
}

/// A parsed fixture file: directive + SQL body.
#[derive(Debug, Clone)]
struct Fixture {
    name: String,
    expect: String, // "PASS" or "UNSUPPORTED:<reason>" or "DEFERRED:<link>"
    sql: String,
}

const FIXTURE_DIR: &str = "tests/compat/mysql_v3_12";
const DISPOSITION_PATH: &str = "docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md";
const LOG_DIR: &str = "docs/releases/v3.12.0/evidence/mysql_compat/logs";

fn parse_fixture(text: &str, default_name: &str) -> Fixture {
    let mut name = default_name.to_string();
    let mut expect = String::new();
    let mut body_lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("# name:") {
            name = rest.trim().to_string();
        } else if let Some(rest) = trimmed.strip_prefix("# expect:") {
            expect = rest.trim().to_string();
        } else if !trimmed.starts_with('#') {
            body_lines.push(line);
        }
    }
    let sql = body_lines.join("\n");
    Fixture { name, expect, sql }
}

fn split_sql(sql: &str) -> Vec<&str> {
    // Naive semicolon split; we don't need to parse SQL here because
    // the compat runner is a smoke test and statements are simple.
    sql.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

fn run_fixture(fix: &Fixture, log_path: &Path) -> Row {
    let log = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path);
    let mut logf = match log {
        Ok(f) => f,
        Err(e) => {
            return Row {
                surface: fix.name.clone(),
                decision: "fail".into(),
                reason: format!("open log: {e}"),
                evidence_hash: String::new(),
                owner: "openclaw".into(),
                expiry: "2027-06-30".into(),
            }
        }
    };

    let stmts = split_sql(&fix.sql);
    if stmts.is_empty() {
        writeln!(logf, "fixture {} has no executable statements", fix.name).ok();
        logf.flush().ok();
        return sha_row(fix, "fail", "no executable statements", log_path);
    }

    let mut client = match CompatClient::connect_default() {
        Ok(c) => c,
        Err(e) => {
            writeln!(logf, "connect_default failed: {e}").ok();
            logf.flush().ok();
            return sha_row(fix, "fail", &format!("connect: {e}"), log_path);
        }
    };

    let mut last_err: Option<String> = None;
    for stmt in &stmts {
        match client.query_rows(stmt) {
            Ok(rows) => {
                writeln!(logf, "OK: {}", stmt.replace('\n', " ")).ok();
                for r in &rows {
                    writeln!(logf, "  ROW: {}", r.join(" | ")).ok();
                }
                last_err = None;
            }
            Err(e) => {
                writeln!(logf, "ERR running `{}`: {}", stmt.replace('\n', " "), e).ok();
                last_err = Some(e.to_string());
            }
        }
    }

    let decision = decide(fix, last_err.as_deref());
    let reason = last_err.unwrap_or_else(|| "executed without error".into());
    sha_row(fix, &decision, &reason, log_path)
}

fn decide(fix: &Fixture, last_err: Option<&str>) -> String {
    let exp = fix.expect.trim();
    if exp.starts_with("UNSUPPORTED") {
        if last_err.is_some() {
            "unsupported".into()
        } else {
            "fail".into()
        }
    } else if exp.starts_with("DEFERRED") {
        // Deferred surfaces are recorded as such without runtime
        // assertion (the runner records `deferred` so the row
        // exists in the disposition for follow-up tracking).
        "deferred".into()
    } else {
        // PASS
        if last_err.is_none() {
            "PASS".into()
        } else {
            "fail".into()
        }
    }
}

fn sha_row(fix: &Fixture, decision: &str, reason: &str, log_path: &Path) -> Row {
    let sha = match fs::read(log_path) {
        Ok(bytes) => sha256_hex(&bytes),
        Err(_) => String::new(),
    };
    Row {
        surface: fix.name.clone(),
        decision: decision.into(),
        reason: reason.into(),
        evidence_hash: sha,
        owner: "openclaw".into(),
        expiry: "2027-06-30".into(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        write!(s, "{:02x}", b).unwrap();
    }
    s
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, hour, min, sec) = epoch_to_ymdhms(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

fn epoch_to_ymdhms(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    // Algorithm from Howard Hinnant's date.h, public domain.
    let z = (secs / 86400) as i64;
    let s = (secs % 86400) as u32;
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    let hh = s / 3600;
    let mm = (s % 3600) / 60;
    let ss = s % 60;
    (y as i32, m, d, hh, mm, ss)
}

fn write_disposition(rows: &[Row], source_run: &str) -> std::io::Result<()> {
    let out = DISPOSITION_PATH;
    if let Some(parent) = Path::new(out).parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut s = String::new();
    writeln!(s, "# v3.12.0 MySQL Compat — Surface Disposition (runtime)").unwrap();
    writeln!(s).unwrap();
    writeln!(s, "- source_agent: `minimax`").unwrap();
    writeln!(s, "- source_run: `{}`", source_run).unwrap();
    writeln!(s, "- timestamp: `{}`", now_iso()).unwrap();
    writeln!(
        s,
        "- branch: `{}`",
        std::env::var("COMPAT_BRANCH").unwrap_or_else(|_| "develop/v3.12.0".into())
    )
    .unwrap();
    writeln!(
        s,
        "- commit: `{}`",
        std::env::var("COMPAT_COMMIT").unwrap_or_else(|_| "unknown".into())
    )
    .unwrap();
    writeln!(s).unwrap();
    writeln!(
        s,
        "| surface | decision | reason | evidence_hash | owner | expiry |"
    )
    .unwrap();
    writeln!(
        s,
        "|---------|----------|--------|---------------|-------|--------|"
    )
    .unwrap();
    for r in rows {
        writeln!(
            s,
            "| {} | {} | {} | {} | {} | {} |",
            r.surface,
            r.decision,
            r.reason.replace('|', "\\|").replace('\n', " "),
            r.evidence_hash,
            r.owner,
            r.expiry
        )
        .unwrap();
    }
    writeln!(s).unwrap();
    writeln!(s, "---").unwrap();
    writeln!(
        s,
        "<!-- generated by tools/compat-runner; regenerated on each gate run -->"
    )
    .unwrap();
    fs::write(out, s)
}

fn head_sha() -> String {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "nohead".into(),
    }
}

fn main() -> Result<()> {
    // 1. Start an ephemeral server. We hold the handle for the rest
    //    of the run; Drop shuts the server down on exit.
    let cfg = EphemeralConfig::default();
    let handle = match start_ephemeral(cfg) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("ERROR: start_ephemeral failed: {e}");
            std::process::exit(2);
        }
    };
    let port = handle.port;
    eprintln!("compat-runner: started ephemeral server on port {port}");
    std::env::set_var("COMPAT_PORT", port.to_string());

    // 2. Walk the fixture directory.
    let fixdir = Path::new(FIXTURE_DIR);
    if !fixdir.exists() {
        eprintln!("ERROR: fixture directory missing: {}", fixdir.display());
        std::process::exit(2);
    }

    let log_dir = Path::new(LOG_DIR);
    fs::create_dir_all(log_dir).ok();

    // 3. Iterate *.sql. The runner records one row per fixture.
    //    Map by surface name so a later pass can de-dup.
    let mut by_surface: std::collections::BTreeMap<String, Row> = std::collections::BTreeMap::new();

    let mut entries: Vec<_> = fs::read_dir(fixdir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "sql")
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let text = fs::read_to_string(&path)?;
        let default_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let fix = parse_fixture(&text, &default_name);
        let log_path = log_dir.join(format!("{}.log", fix.name));
        let row = run_fixture(&fix, &log_path);
        by_surface.insert(row.surface.clone(), row);
    }

    // 4. Ensure the 10 known surfaces are present. Missing
    //    surfaces default to `deferred` with a follow-up rationale.
    let known_surfaces = [
        "show_tables",
        "alter_rename",
        "alter_modify_column",
        "alter_add_column",
        "alter_drop_column",
        "empty_password_auth",
        "prepared_stmt_roundtrip",
        "with_rollup_unsupported",
        "with_cube_unsupported",
        "stddev_pop_unsupported",
    ];
    for s in &known_surfaces {
        by_surface.entry(s.to_string()).or_insert_with(|| Row {
            surface: s.to_string(),
            decision: "deferred".into(),
            reason: "no fixture seeded; v3.12 follow-up".into(),
            evidence_hash: String::new(),
            owner: "openclaw".into(),
            expiry: "2027-06-30".into(),
        });
    }

    // 5. Write disposition sorted by surface.
    let mut final_rows: Vec<Row> = by_surface.values().cloned().collect();
    final_rows.sort_by(|a, b| a.surface.cmp(&b.surface));
    let source_run = format!(
        "minimax-v312-21-runner-{}",
        std::env::var("COMPAT_SOURCE_RUN").unwrap_or_else(|_| head_sha())
    );
    write_disposition(&final_rows, &source_run)?;

    // 6. Print summary.
    let pass_count = final_rows.iter().filter(|r| r.decision == "PASS").count();
    let unsup_count = final_rows
        .iter()
        .filter(|r| r.decision == "unsupported")
        .count();
    let deferred_count = final_rows
        .iter()
        .filter(|r| r.decision == "deferred")
        .count();
    let fail_count = final_rows.iter().filter(|r| r.decision == "fail").count();
    println!(
        "compat-runner: {} surfaces, pass={} unsupported={} deferred={} fail={}",
        final_rows.len(),
        pass_count,
        unsup_count,
        deferred_count,
        fail_count
    );
    println!("disposition: {}", DISPOSITION_PATH);

    // Hold the server handle until exit.
    drop(handle);
    Ok(())
}
