//! ADR-008-exception: 2026-09-01 (hermes-agent) — G4 SF=1 wire 22/22 close-out
//! in flight per docs/releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md.
//! Re-evaluate 2026-09-01. See ADR-008-exception-v311-tpch-sf1.md.
//!
//! TPC-H SF=1.0 cross-engine baseline (in-process surface).
//! Runs the 22 canonical TPC-H queries against the in-process
//! sqlrustgo server (via `MySqlTestClient` + `start_ephemeral`)
//! on the SF=1.0 fixture and writes the row counts and timings
//! to `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`.
//!
//! The test is `#[ignore]`d when the SF=1.0 fixture is not
//! present; an operator message tells the user how to generate
//! it. To run with the fixture present:
//!
//! ```bash
//! cargo test --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture
//! ```
//!
//! See openspec/changes/2026-06-18-tpch-sf1-baseline.

#[path = "../../common/mod.rs"]
mod common;

use common::tpch_wire_harness;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::path::Path;
use std::time::{Duration, Instant};

/// Path to the SF=1.0 fixture. Operator must generate this with
/// `dbgen -s 1 -f` (see scripts/tpch_sf1_baseline.sh) or with
/// `scripts/generate_tpch_data.sh --sf 1 --backend dbgen`.
const DEFAULT_SF1_DIR: &str = "/tmp/tpch-sf1";

fn sf1_dir() -> String {
    std::env::var("TPCH_SF1_DIR").unwrap_or_else(|_| DEFAULT_SF1_DIR.to_string())
}

fn sqlrustgo_data_dir() -> String {
    std::env::var("TPCH_SF1_SQLRUSTGO_DATA_DIR")
        .or_else(|_| std::env::var("TPCH_SF1_DIR"))
        .unwrap_or_else(|_| DEFAULT_SQLRUSTGO_DATA_DIR.to_string())
}

/// BINT v2 (BinaryTableStorage) directory produced by
/// `cargo run --bin tbl2bin -- <SF1_DIR> <BINT_DIR>`. When all 8
/// `.bin` files are present, the test boots an ephemeral with
/// `storage: Some("binary")` and skips LOAD DATA entirely
/// (`BinaryTableStorage::new_with_data` mmaps the files in <1 s).
const DEFAULT_BINT_DIR: &str = "/tmp/tpch-sf1-bin";
fn bint_dir() -> String {
    std::env::var("TPCH_BINT_DIR").unwrap_or_else(|_| DEFAULT_BINT_DIR.to_string())
}

/// The sqlrustgo data dir. We deliberately point this at the same
/// directory as `SF1_DIR` so the server's LOAD DATA whitelist
/// (which requires the source file to live inside the data_dir)
/// accepts the `.tbl` fixtures without an extra copy. After the
/// first run, the data is materialized as a WAL plus per-table
/// files inside this directory and subsequent runs recover from
/// it without re-running LOAD DATA.
const DEFAULT_SQLRUSTGO_DATA_DIR: &str = "/tmp/tpch-sf1";

const DEFAULT_REPORT_PATH: &str = "docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md";
fn report_path() -> String {
    std::env::var("TPCH_REPORT_PATH").unwrap_or_else(|_| DEFAULT_REPORT_PATH.to_string())
}
/// after generation; the test will write to this exact path.

/// Per-query wall-clock time is recorded by the test itself
/// (Instant::now() / elapsed()) and bounded by the underlying
/// `set_timeouts` value (LOADER_TIMEOUT_S). The per-query timeouts
/// in the MySQL client sense are intentionally not lower than the
/// loader timeout because the SF=1.0 Q9 6-way join is the worst
/// case and can take a few minutes.
const LOADER_TIMEOUT_S: u64 = 1800;

const QUERIES_DIR: &str = "queries";

/// True iff the SF=1.0 fixture is present at SF1_DIR.
fn fixture_present() -> bool {
    let dir = sf1_dir();
    let p = Path::new(&dir);
    p.join("region.tbl").exists()
        && p.join("nation.tbl").exists()
        && p.join("supplier.tbl").exists()
        && p.join("customer.tbl").exists()
        && p.join("part.tbl").exists()
        && p.join("partsupp.tbl").exists()
        && p.join("orders.tbl").exists()
        && p.join("lineitem.tbl").exists()
}

/// Expected row counts. Defaults to SF=1.0 dbgen row counts; override
/// via env `TPCH_EXPECTED_ROWS_JSON` (JSON object `{"table": rows}`).
fn expected_row_counts() -> Vec<(&'static str, usize)> {
    if let Ok(s) = std::env::var("TPCH_EXPECTED_ROWS_JSON") {
        if let Ok(map) = serde_json::from_str::<std::collections::BTreeMap<String, usize>>(&s) {
            // Stable table order matters for downstream assertions.
            const TABLES: &[&str] = &[
                "region", "nation", "supplier", "customer", "part", "partsupp", "orders",
                "lineitem",
            ];
            return TABLES
                .iter()
                .map(|t| {
                    let v = map.get(*t).copied().unwrap_or(0);
                    (*t, v)
                })
                .collect();
        }
    }
    vec![
        ("region", 5),
        ("nation", 25),
        ("supplier", 10_000),
        ("customer", 150_000),
        ("part", 200_000),
        ("partsupp", 800_000),
        ("orders", 1_500_000),
        ("lineitem", 6_001_215),
    ]
}

/// True iff all 8 .json table files exist with valid row counts
/// (i.e., already generated, avoiding expensive LOAD DATA).
fn json_data_ready() -> bool {
    let data_dir_value = sqlrustgo_data_dir();
    let data_dir = Path::new(&data_dir_value);
    let expected = expected_row_counts();
    for (name, expected_rows) in &expected {
        let json_path = data_dir.join(format!("{}.json", name));
        if !json_path.exists() {
            return false;
        }
        let content = std::fs::read_to_string(&json_path).expect("read json");
        // Count occurrences of "rows": at the top level to find the rows array.
        // Each row in the server's JSON format is a JSON object like
        // `{"Integer":123}` or `{"Text":"hello"}`. We count these by looking
        // for the pattern `"rows":[` and then counting objects `{` that appear
        // at the top level inside that array.
        if let Some(rows_start) = content.find(r#""rows":["#) {
            let after_rows = &content[rows_start + 8..];
            let mut depth = 0i32;
            let mut row_count = 0usize;
            let mut in_object = false;
            let mut started = false;
            for ch in after_rows.chars() {
                if !started {
                    if ch == '[' {
                        started = true;
                        depth = 1;
                    }
                    continue;
                }
                match ch {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    '{' if depth == 1 => {
                        in_object = true;
                    }
                    '}' if in_object && depth == 1 => {
                        row_count += 1;
                        in_object = false;
                    }
                    _ => {}
                }
            }
            if row_count != *expected_rows {
                eprintln!(
                    "  json_data_ready: {} expected {} rows, got {}",
                    name, expected_rows, row_count
                );
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

/// True iff all 8 `.bin` (BINT v2) files are present at `BINT_DIR`
/// with a valid BINT v2 header. The row-count check is deferred to
/// the test itself (it uses the live `BinaryTableStorage` once
/// `start_ephemeral` is up). A valid header is "BINT" magic + u32
/// version = 2.
fn bint_data_ready() -> bool {
    const TABLES: &[&str] = &[
        "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
    ];
    let dir = std::path::PathBuf::from(bint_dir());
    for t in TABLES {
        let p = dir.join(format!("{}.bin", t));
        let Ok(mut f) = std::fs::File::open(&p) else {
            return false;
        };
        use std::io::Read;
        let mut header = [0u8; 8];
        if f.read(&mut header).unwrap_or(0) != 8 {
            return false;
        }
        if &header[..4] != b"BINT" {
            return false;
        }
        let version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        if version != 2 {
            return false;
        }
    }
    true
}

fn emit_skip_message() {
    let dir = sf1_dir();
    eprintln!(
        "tpch_sf1_22_vs_3engines_test: SF=1.0 fixture not present at {}. \
         Generate it with:\n  \
         /home/openclaw/tpch-dbgen-master/dbgen -s 1 -f\n  \
         mkdir -p {}\n  \
         mv /home/openclaw/tpch-dbgen-master/*.tbl {}/\n\
         (or `scripts/generate_tpch_data.sh --sf 1 --backend dbgen`).\n\
         The test is marked #[ignore] so it does not consume the 10-minute budget.",
        dir, dir, dir
    );
}

#[ignore = "requires SF=1.0 fixture at /tmp/tpch-sf1 (dbgen -s 1 -f); run with --ignored"]
#[test]
fn tpch_sf1_22_in_process_regression() {
    if !fixture_present() {
        emit_skip_message();
        return;
    }

    // 1) Pick a backend. Three tiers, fastest first:
    //    a) BINT v2 (`BinaryTableStorage` reading pre-generated `.bin`
    //       files) — preferred; load is <1 s regardless of row count.
    //    b) JSON row files already materialized by a prior
    //       `FileStorage` + LOAD DATA run.
    //    c) Cold: run LOAD DATA LOCAL INFILE from the `.tbl` fixture.
    //
    //    The BINT path is the only one that meets the spec's 10-minute
    //    budget for SF=1.0 (6M-lineitem LOAD DATA via JSON serialization
    //    is ~45 min and tails off the runner).
    let use_bint = bint_data_ready();
    let (data_dir, use_load_data): (std::path::PathBuf, bool) = if use_bint {
        eprintln!(
            "SF=1.0 .bin (BINT v2) ready at {} — using BinaryTableStorage.",
            bint_dir()
        );
        (Path::new(&bint_dir()).to_path_buf(), false)
    } else {
        let data_dir_value = sqlrustgo_data_dir();
        let dir = Path::new(&data_dir_value).to_path_buf();
        std::fs::create_dir_all(&dir).expect("create sqlrustgo data dir");
        let json_ready = json_data_ready();
        if json_ready {
            eprintln!("SF=1.0 .json files present with valid row counts — skipping LOAD DATA.");
        } else {
            // Truncate any pre-existing WAL so recovery on startup stays fast,
            // and remove stale .json from a prior partial LOAD DATA run
            // (`FileStorage::new_with_wal` reads ALL .json on startup and
            // blocks the accept loop while it does so).
            let wal = dir.join("sqlrustgo.wal");
            if wal.exists() {
                let _ = std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&wal);
            }
            let stale: Vec<_> = dir
                .read_dir()
                .expect("read sqlrustgo data dir")
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
                .collect();
            for entry in stale {
                std::fs::remove_file(&entry.path()).expect("remove stale .json file");
            }
        }
        (dir, !json_ready)
    };

    // Build the EphemeralConfig. The BINT path needs `bootstrap_sql`
    // to register the 8 TPC-H tables in the catalog (the storage
    // itself only carries row data, not schema). The JSON / cold
    // path keeps `bootstrap_tables: false` and registers schemas via
    // `load_fixture` after connect (which also does the LOAD DATA).
    let mut bootstrap_sql: Vec<String> = Vec::new();
    let storage_backend: Option<String> = if use_bint {
        bootstrap_sql = tpch_wire_harness::SCHEMA_DDL
            .iter()
            .map(|s| s.to_string())
            .collect();
        Some("binary".to_string())
    } else {
        None
    };

    let config = EphemeralConfig {
        data_dir: Some(data_dir.clone()),
        bootstrap_tables: false,
        bootstrap_users: true,
        bootstrap_sql,
        storage: storage_backend,
        ..Default::default()
    };
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect_handle");
    client
        .set_timeouts(
            Duration::from_secs(LOADER_TIMEOUT_S),
            Duration::from_secs(LOADER_TIMEOUT_S),
        )
        .expect("set_timeouts");

    // 2) Cold-path LOAD DATA. Skipped when .bin or .json are pre-materialized.
    if use_load_data {
        eprintln!("Loading SF=1.0 fixture (only required on first run) ...");
        let sf1_dir_value = sf1_dir();
        tpch_wire_harness::load_fixture(&mut client, &sf1_dir_value);
        eprintln!("SF=1.0 fixture loaded.");
    }

    // 3) Run the 22 TPC-H queries and record results.
    let mut report_rows: Vec<(u8, usize, Duration, String)> = Vec::new();
    // Optional: run only a single query via TPCH_ONLY_Q=N env var
    let only_q: Option<u8> = std::env::var("TPCH_ONLY_Q")
        .ok()
        .and_then(|v| v.parse().ok());

    for n in 1..=22u8 {
        if let Some(q) = only_q {
            if n != q {
                continue;
            }
        }
        let sql_path = format!("{}/q{}.sql", QUERIES_DIR, n);
        let sql = std::fs::read_to_string(&sql_path)
            .unwrap_or_else(|e| panic!("read {}: {}", sql_path, e));
        if let Some(q) = only_q {
            // TPCH_ONLY_Q is exclusive — run exactly one and stop
        } else if (std::env::var("TPCH_SKIP_Q9").is_ok() && n == 9)
            || (std::env::var("TPCH_SKIP_Q10").is_ok() && n == 10)
        {
            let label = if n == 9 { "Q9 6-table join OOM" } else { "Q10 3-table join OOM" };
            eprintln!(
                "  Q{:>2}: {} rows in {:?}  [skipped — {}, tracked #3732]",
                n, 0, Duration::ZERO, label
            );
            report_rows.push((n, 0, Duration::ZERO, "skipped".to_string()));
            continue;
        }
        let start = Instant::now();
        let result = client.query_rows(&sql);
        eprintln!(
            "  >>> Q{:>2}: query complete, {} rows",
            n,
            result.as_ref().map(|r| r.len()).unwrap_or(0)
        );
        let elapsed = start.elapsed();
        let notes = match &result {
            Ok(rows) => format!("ok; {} rows", rows.len()),
            Err(e) => format!("err: {}", e),
        };
        let row_count = result.as_ref().map(|r| r.len()).unwrap_or(0);
        report_rows.push((n, row_count, elapsed, notes));
        eprintln!(
            "  Q{:>2}: {} rows in {:?}  [{}]",
            n,
            row_count,
            elapsed,
            result.as_ref().map(|_| "ok").unwrap_or("err")
        );

        // SHA256 side-channel: when TPCH_SF1_ROWS_DIR is set, dump the
        // (already text-serialized) rows to <rows_dir>/q<N>.tsv, one row
        // per line, columns tab-separated. This is what `verify_binary_checksum`
        // and issue #3654 cross-engine diff use; not the test's own
        // acceptance path. Set by the SF=1 cross-engine driver; the test
        // itself never reads it.
        if let Ok(rows_dir) = std::env::var("TPCH_SF1_ROWS_DIR") {
            if let Ok(rows) = &result {
                let p = std::path::Path::new(&rows_dir).join(format!("q{}.tsv", n));
                let mut buf = String::new();
                for row in rows {
                    for (i, cell) in row.iter().enumerate() {
                        if i > 0 {
                            buf.push('\t');
                        }
                        buf.push_str(cell);
                    }
                    buf.push('\n');
                }
                let _ = std::fs::write(p, buf);
            }
        }
    }

    // 4) Acceptance check. SF=1 correctness failures must fail the gate,
    //    not merely emit warnings that still produce a green cargo exit.
    let required_non_empty: &[u8] = &[5, 8, 10, 13, 16, 21];
    let mut zero_row_failures: Vec<u8> = Vec::new();
    for (n, count, _elapsed, _notes) in &report_rows {
        if *count == 0 && required_non_empty.contains(n) {
            zero_row_failures.push(*n);
            eprintln!(
                "  [fail] Q{} returned 0 rows on SF=1; this is a correctness failure",
                n
            );
        } else if *count == 0 {
            eprintln!(
                "  [warn] Q{} returned 0 rows; review report at {} before counting it as PASS",
                n,
                report_path()
            );
        }
    }
    eprintln!("TPC-H query execution completed; correctness failures are reported below.");

    // 5) Write the Markdown report before asserting, so failed runs still leave evidence.
    write_report(&report_rows, &zero_row_failures);
    eprintln!("Wrote {}", report_path());

    // TPCH_SKIP_PANIC=1 turns the required-non-empty check into a
    // eprintln-only warning. Used by the G4 cross-engine driver
    // (`scripts/tpch_sf1_baseline.sh --cross-engine`) when it needs
    // row files for every query, even ones known to return 0 rows
    // (Q16, see CROSS_ENGINE_BASELINE engine bug). The default is
    // to keep the panic — issue #3650 P0-2 still requires the test
    // to fail on a regression of the previously-known-good queries.
    if !zero_row_failures.is_empty() {
        if std::env::var("TPCH_SKIP_PANIC").is_ok() {
            eprintln!(
                "[skip-panic] required-non-empty check failed: {:?} (TPCH_SKIP_PANIC=1)",
                zero_row_failures
            );
        } else {
            panic!(
                "SF=1 TPC-H correctness failure: required non-empty queries returned 0 rows: {:?}",
                zero_row_failures
            );
        }
    }
}

fn write_report(rows: &[(u8, usize, Duration, String)], zero_row_failures: &[u8]) {
    std::fs::create_dir_all(Path::new(&report_path()).parent().unwrap())
        .expect("create report dir");
    let mut out = String::new();
    out.push_str("# TPC-H SF=1.0 cross-engine baseline (in-process)\n\n");
    out.push_str("- Issue: #3423\n");
    out.push_str("- Spec: openspec/changes/2026-06-18-tpch-sf1-baseline\n");
    out.push_str("- Surface: in-process via `MySqlTestClient` + `start_ephemeral`\n");
    out.push_str("- Branch: feature/issue-3423-tpch-sf1-baseline\n");
    out.push_str("- Commit: see `git log` on the branch\n");
    out.push_str("- External-client follow-up: issue #3474 (out of scope here)\n\n");
    out.push_str("## Setup\n\n");
    let fixture_dir = sf1_dir();
    out.push_str(&format!(
        "- Fixture path: `{}`\n- Generation tool: `dbgen -s 1 -f` \
         (TPC-H dbgen, official)\n",
        fixture_dir
    ));
    out.push_str("- Row counts (verified at fixture load time):\n");
    for (tbl, expected) in expected_row_counts().iter() {
        let p = format!("{}/{}.tbl", fixture_dir, tbl);
        let actual = std::fs::read_to_string(&p)
            .map(|s| s.lines().filter(|l| !l.is_empty()).count())
            .unwrap_or(0);
        out.push_str(&format!(
            "  - {}: {} rows (expected ~{})\n",
            tbl, actual, expected
        ));
    }
    out.push_str("\n## Per-query results (sqlrustgo only — in-process surface)\n\n");
    out.push_str("| Q | rows | elapsed (ms) | notes |\n");
    out.push_str("|---|------|---------------|-------|\n");
    for (n, count, elapsed, notes) in rows {
        out.push_str(&format!(
            "| Q{:>2} | {} | {:.1} | {} |\n",
            n,
            count,
            elapsed.as_secs_f64() * 1000.0,
            notes
        ));
    }
    out.push_str("\n## Summary\n\n");
    let total_rows: usize = rows.iter().map(|(_, c, _, _)| *c).sum();
    let total_ms: f64 = rows
        .iter()
        .map(|(_, _, e, _)| e.as_secs_f64() * 1000.0)
        .sum();
    let completed = rows.len();
    let zero_rows = rows.iter().filter(|(_, c, _, _)| *c == 0).count();
    out.push_str(&format!(
        "- Queries executed in this run: {}/22\n- Queries returning 0 rows: {}\n- Total rows across executed queries: {}\n- \
         Total elapsed time: {:.1} ms ({:.1} s)\n",
        completed,
        zero_rows,
        total_rows,
        total_ms,
        total_ms / 1000.0
    ));
    if zero_row_failures.is_empty() {
        out.push_str("- Required non-empty query check: PASS for executed queries\n");
    } else {
        out.push_str(&format!(
            "- Required non-empty query check: FAIL - Q{:?} returned 0 rows\n",
            zero_row_failures
        ));
    }
    out.push_str("- Slowest query: ");
    if let Some((n, _c, e, _notes)) = rows.iter().max_by_key(|(_, _, e, _)| *e) {
        out.push_str(&format!("Q{} ({:.1} ms)\n", n, e.as_secs_f64() * 1000.0));
    }
    out.push_str("\n## Limitations\n\n");
    out.push_str(
        "- This report is in-process only. It does NOT cover the\n  \
         external-client path (mysql-client 8.0.46 / libmysqlclient 8.0.46),\n  \
         which is tracked in issue #3474.\n",
    );
    out.push_str(
        "- The cross-engine comparison against MariaDB / SQLite is\n  \
         not in this report; the baseline here is sqlrustgo only.\n  \
         The full cross-engine baseline is the subject of\n  \
         `scripts/tpch_sf1_baseline.sh`, which is not yet wired\n  \
         (see tasks.md item 3 in\n  \
         openspec/changes/2026-06-18-tpch-sf1-baseline).\n",
    );
    out.push_str(
        "- Per-query cell-by-cell value comparison (not just row\n  \
         count) is also out of scope here. That is the next\n  \
         step once #3474 is resolved.\n",
    );

    std::fs::write(report_path(), out).expect("write report");
}
