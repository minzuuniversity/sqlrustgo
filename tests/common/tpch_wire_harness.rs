//! TPC-H wire-protocol test harness — shared helper for 13 in-process
//! → wire protocol migration tests/tpch_*.rs files.

use super::tpch_wire_scope;
use super::MySqlTestClient;
use serde_json::Value as JsonValue;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Process-wide counter that makes every wire staging directory unique.
///
/// The tests of one binary run in parallel and each of them stages the
/// fixture into a directory inside the shared fixture directory. A fixed
/// directory name let one test's cleanup delete the directory another test
/// was copying into (CI run 8117: "copy .../.tpch-wire-staging/region.tbl:
/// No such file or directory").
static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

pub const SF001_DIR: &str = "tests/data/tpch-sf001";
pub const SF01_DIR: &str = "tests/data/tpch-sf01";

/// Marker prefix for a test that could not run.
///
/// The canonical definition lives in `tests/common/tpch_fixture.rs`; the two
/// must stay in sync — the CI summary counts every log line containing this
/// prefix as a `not_run` test (never as a passed one).
pub const NOT_RUN_MARKER: &str = "TPCH-NOT-RUN";

/// Repository-local fixture candidates, tried when `TPCH_DATA_DIR` is unset.
///
/// The former entries `tests/data/tpch-sf01` and `tests/data/tpch-sf001` were
/// symlinks pointing at `/tmp` — they resolved on exactly one machine and
/// nowhere else, which is why every fixture-dependent test panicked in CI
/// (issue #5379). Both symlinks were removed; the fixture is now generated
/// at run time and handed over through `TPCH_DATA_DIR`.
const FIXTURE_CANDIDATES: [&str; 2] = ["tests/data/tpch-fixture", "data/tpch-sf01"];

/// Resolve the TPC-H fixture directory **at run time**.
///
/// `TPCH_DATA_DIR` wins (that is how CI provides the generated fixture);
/// otherwise the repository-local candidates are tried. The old code baked
/// the path in at compile time through `option_env!`, which could never see
/// a run-time environment variable.
pub fn resolve_fixture_dir() -> Option<PathBuf> {
    match std::env::var("TPCH_DATA_DIR") {
        Ok(dir) if !dir.trim().is_empty() => {
            let path = PathBuf::from(dir.trim());
            if path.join("lineitem.tbl").is_file() {
                Some(path)
            } else {
                None
            }
        }
        _ => FIXTURE_CANDIDATES
            .iter()
            .map(PathBuf::from)
            .find(|p| p.join("lineitem.tbl").is_file()),
    }
}

/// Name of the manifest file written next to a generated fixture.
pub const FIXTURE_MANIFEST_FILE: &str = "tpch_fixture_manifest.json";

/// Scale factor of the fixture that is actually on disk.
///
/// `None` when the fixture has no manifest, or the manifest is unreadable —
/// an *unknown* profile. Callers that compare against a calibrated baseline
/// must treat `None` as a mismatch rather than as "compatible": a test
/// calibrated against one dataset cannot be trusted to hold against an
/// unidentified one.
pub fn fixture_scale_factor() -> Option<f64> {
    let dir = resolve_fixture_dir()?;
    let raw = std::fs::read_to_string(dir.join(FIXTURE_MANIFEST_FILE)).ok()?;
    let v: JsonValue = serde_json::from_str(&raw).ok()?;
    v.get("sf").and_then(|s| s.as_f64())
}

/// Report that a test could not run, in the machine-readable form the CI
/// summary counts.
pub fn report_not_run(target: &str, reason: &str, detail: &str) {
    let line = format!("{NOT_RUN_MARKER}: {target} reason={reason} detail={detail}");
    println!("{line}");
    if let Ok(path) = std::env::var("TPCH_NOT_RUN_FILE") {
        if !path.trim().is_empty() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.trim())
            {
                let _ = writeln!(f, "{line}");
            }
        }
    }
}

/// The name of the synthetic leading column that the wire harness adds to
/// the two TPC-H tables whose first column is **not** unique.
///
/// The file-storage backend keys a row by the **table's first column**:
/// rows that share the first column's value collapse into a single visible
/// row. `partsupp` (four rows per part) and `lineitem` (one to seven rows
/// per order) are exactly that shape, so without the synthetic column the
/// wire layer sees a quarter of the fixture while LOAD DATA cheerfully
/// reports every row as loaded.
///
/// This is a workaround for an engine defect (issue #5392), not a schema
/// improvement: the column is not referenced by any query, and it exists
/// only so the storage backend can tell the rows apart. The harness asserts
/// after every load that the number of rows the engine can see equals the
/// number of rows in the fixture, so the defect cannot come back silently.
pub const ROWID_COLUMN: &str = "tpch_rowid";

/// The tables whose first column is not unique and therefore need the
/// synthetic leading column.
const NEEDS_ROWID: [&str; 2] = ["partsupp", "lineitem"];

/// The canonical TPC-H schema, used by every wire-protocol test that loads
/// the fixture through this harness.
///
/// The `orders` table declares all **nine** canonical columns. It used to
/// declare only eight — `o_totalprice` was missing — while the `.tbl` files
/// carry nine fields, so every column after `o_orderstatus` was shifted by
/// one: `o_orderdate` read the price, `o_clerk` read the date, and
/// `o_comment` fell off the end of the row.
///
/// The damage was silent. A filter on `o_orderstatus` still matched the
/// right rows (it is the third column, before the shift), so the tests
/// looked healthy, while every later column held the wrong value. The
/// in-process path failed loudly instead ("row width 9 does not match the 8
/// declared columns"), and the symptom that finally made it visible was a
/// `LIKE` filter matching nothing: `o_comment LIKE '%special%requests%'`
/// selected 0 rows because the engine was comparing the *price* column.
///
/// `partsupp` and `lineitem` carry the synthetic [`ROWID_COLUMN`] as their
/// first column — see its documentation for why.
///
/// The schema is kept in sync with `tests/common/tpch_cli_harness.rs`.
pub const SCHEMA_DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (tpch_rowid INTEGER NOT NULL, ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)",
    "CREATE TABLE lineitem (tpch_rowid INTEGER NOT NULL, l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

pub const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

/// Start ephemeral server + load fixture dir via LOAD DATA LOCAL INFILE.
///
/// `timeout_s`: Some(s) → set read/write timeouts to s seconds; None → use
/// the `MySqlTestClient` default (5s for read, 5s for write).
///
/// Returns `None` when the fixture is unavailable. The NOT-RUN marker is
/// printed in that case, so the test is disclosed as *not run* by the CI
/// summary instead of being counted as passed.
fn start_with_fixture(
    fixture_dir: &Path,
    timeout_s: Option<u64>,
    target: &str,
) -> Option<MySqlTestClient> {
    // The TPC-H fixtures are generated, not committed: `tests/data/tpch-sf01`
    // used to be a symlink into /tmp, which did not survive a reboot and did
    // not exist on CI at all. Without this check the ephemeral server starts
    // against a missing directory and the TCP connection is reset, which
    // reads like a protocol bug rather than a missing prerequisite.
    if !fixture_dir.join("lineitem.tbl").exists() {
        report_not_run(
            target,
            "fixture_missing",
            &format!("TPC-H data not found at {}", fixture_dir.display()),
        );
        return None;
    }
    let config = EphemeralConfig {
        // The server's **storage** directory must NOT be the fixture
        // directory. The server persists every table it receives through
        // LOAD DATA into the storage dir as JSON row files
        // (`lineitem.json` alone is ~40 MB at SF=0.01). A second
        // `start_sf001()` in the same process therefore had to deserialize
        // the *previous* run's JSON before it could accept a connection:
        // the client hit the 5 s handshake timeout, `connect_handle`
        // returned an error, and the handle's `Drop` — which joins the
        // server thread — blocked forever. That turned every test file
        // with more than one `start_sf001()` call into a hang.
        //
        // `data_dir: None` makes the server create a unique temporary
        // storage dir and remove it on drop.
        data_dir: None,
        // The `.tbl` files are the LOAD DATA *source*; they live in the
        // fixture directory, which is a different directory from the
        // storage one. `load_infile_dir` is the LOAD DATA LOCAL INFILE
        // whitelist; without it the whitelist would fall back to the
        // storage dir and every load would be rejected.
        load_infile_dir: Some(fixture_dir.to_path_buf()),
        bootstrap_tables: false,
        bootstrap_users: true,
        metrics_port: None,

        ..Default::default()
    };
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect_handle");
    if let Some(t) = timeout_s {
        client
            .set_timeouts(Duration::from_secs(t), Duration::from_secs(t))
            .expect("set_timeouts");
    }
    load_fixture(
        &mut client,
        fixture_dir
            .to_str()
            .expect("fixture directory is not valid UTF-8"),
    );
    Some(client)
}

/// Start ephemeral server + load the TPC-H fixture (5s default timeouts).
///
/// The fixture is resolved at run time through `TPCH_DATA_DIR` (see
/// [`resolve_fixture_dir`]). Returns `None` when it is unavailable.
pub fn start_sf001() -> Option<MySqlTestClient> {
    start_sf001_named("tpch_wire_harness")
}

/// Start ephemeral server + load the TPC-H fixture, naming `target` in the
/// NOT-RUN marker.
///
/// The marker has to carry the **cargo target name** so that a reader of the
/// CI log can tell which target was skipped. The generic
/// `tpch_wire_harness` label that [`start_sf001`] uses does not identify a
/// target, which makes the summary ambiguous.
pub fn start_sf001_named(target: &str) -> Option<MySqlTestClient> {
    let Some(dir) = resolve_fixture_dir() else {
        report_not_run(
            target,
            "fixture_missing",
            "no TPC-H fixture found (set TPCH_DATA_DIR to a directory with the 8 .tbl files)",
        );
        return None;
    };
    start_with_fixture(&dir, None, target)
}

/// Start ephemeral server + load the TPC-H fixture (60s timeouts for larger data).
///
/// Returns `None` when the fixture is unavailable.
pub fn start_sf01() -> Option<MySqlTestClient> {
    start_sf01_named("tpch_wire_harness")
}

/// Start ephemeral server + load the TPC-H fixture (60s timeouts), naming
/// `target` in the NOT-RUN marker.
pub fn start_sf01_named(target: &str) -> Option<MySqlTestClient> {
    let Some(dir) = resolve_fixture_dir() else {
        report_not_run(
            target,
            "fixture_missing",
            "no TPC-H fixture found (set TPCH_DATA_DIR to a directory with the 8 .tbl files)",
        );
        return None;
    };
    start_with_fixture(&dir, Some(60), target)
}

/// Load all 8 `.tbl` files via LOAD DATA LOCAL INFILE.
///
/// The fixture is first **staged**: the tables whose first column is not
/// unique get a synthetic unique leading column (see [`ROWID_COLUMN`]),
/// because the file-storage backend collapses rows that share the first
/// column's value (issue #5392). The staged copy lives inside the fixture
/// directory so that it stays inside the server's LOAD DATA whitelist.
///
/// After every table the harness asks the engine how many rows it can
/// actually *see* and compares that with the number of rows in the source
/// file. The comparison is the guard that makes the workaround safe: if the
/// storage defect ever comes back, the tests fail loudly instead of
/// silently asserting against a quarter of the fixture.
pub fn load_fixture(client: &mut MySqlTestClient, fixture_dir: &str) {
    for ddl in SCHEMA_DDL {
        client
            .exec(ddl)
            .unwrap_or_else(|e| panic!("exec DDL `{ddl}`: {e}"));
    }
    let dir = PathBuf::from(fixture_dir);
    let staged = stage_fixture_for_wire(&dir);
    // The staging directory is removed when this guard goes out of scope —
    // including when a test panics below (the harness panics on every load
    // failure), so a failed test cannot leave staging directories behind in
    // the fixture directory.
    let _staging = StagedFixture(staged.clone());
    for tbl in TABLES {
        let path = staged.join(format!("{tbl}.tbl"));
        if !path.exists() {
            panic!("fixture not found: {}", path.display());
        }
        let source_rows = count_tbl_rows(&staged.join(format!("{tbl}.tbl")));
        let n = client
            .load_local_infile(&path, tbl)
            .unwrap_or_else(|e| panic!("load_local_infile {}: {}", path.display(), e));
        let visible = client
            .query_one_i64(&format!("SELECT COUNT(*) FROM {tbl}"))
            .unwrap_or_else(|e| panic!("counting the loaded {tbl} rows: {e}"));
        eprintln!("  loaded {tbl}: {n} rows");
        assert_eq!(
            visible, source_rows,
            "the engine can only see {visible} of the {source_rows} rows of `{tbl}` — \
             the storage backend is dropping rows (see issue #5392)"
        );
    }
}

/// Removes a staging directory when it goes out of scope.
struct StagedFixture(PathBuf);

impl Drop for StagedFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Number of data rows in a `.tbl` file.
fn count_tbl_rows(path: &Path) -> i64 {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count() as i64
}

/// Materialise a "wire-ready" copy of the fixture inside the fixture
/// directory.
///
/// The copy carries a synthetic unique leading column for the tables whose
/// first column is not unique, so that the storage backend can tell their
/// rows apart (issue #5392). Every other table is copied verbatim.
///
/// **Every call gets its own directory.** The tests of one binary run in
/// parallel and each of them stages the fixture, so a shared, fixed
/// directory name let one test's cleanup delete the directory another test
/// was copying into. CI run 8117 hit exactly that:
///
/// ```text
/// copy /tmp/tpch-fixture/.tpch-wire-staging/region.tbl
///   -> /tmp/tpch-fixture/.tpch-wire-staging/region.tbl:
///   No such file or directory (os error 2)
/// ```
///
/// The name carries the process id and a process-wide counter, so it is
/// unique across threads *and* across processes.
fn stage_fixture_for_wire(fixture_dir: &Path) -> PathBuf {
    let seq = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    let staged = fixture_dir.join(format!(".tpch-wire-staging-{}-{seq}", std::process::id()));
    // A previous run of the same process (or a killed one) may have left the
    // directory behind.
    let _ = std::fs::remove_dir_all(&staged);
    std::fs::create_dir_all(&staged).unwrap_or_else(|e| panic!("create {}: {e}", staged.display()));

    for tbl in TABLES {
        let source = fixture_dir.join(format!("{tbl}.tbl"));
        let target = staged.join(format!("{tbl}.tbl"));
        if NEEDS_ROWID.contains(tbl) {
            let content = std::fs::read_to_string(&source)
                .unwrap_or_else(|e| panic!("read {}: {e}", source.display()));
            let mut out = String::with_capacity(content.len() + content.len() / 8);
            for (i, line) in content.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                out.push_str(&(i + 1).to_string());
                out.push('|');
                out.push_str(line);
                out.push('\n');
            }
            std::fs::write(&target, out)
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        } else {
            std::fs::copy(&source, &target).unwrap_or_else(|e| {
                panic!("copy {} -> {}: {e}", source.display(), target.display())
            });
        }
    }
    staged
}

/// A statement that was **not** executed because it is outside the wire
/// scope.
#[derive(Debug, Clone)]
pub struct WireSkip {
    /// The query identifier that was refused.
    pub qid: String,
    /// Controlled reason code.
    pub reason: String,
    /// The issue that tracks the defect.
    pub issue: String,
    /// What was measured.
    pub detail: String,
}

/// The outcome of handing a statement to the wire path.
#[derive(Debug)]
pub enum WireOutcome {
    /// The statement was executed. `Err` carries the server's error message.
    Ran(Result<Vec<Vec<String>>, String>, Duration),
    /// The statement is outside the **wire scope** and was **not** executed.
    ///
    /// Executing it would exhaust the machine (issue #5393): the engine
    /// process is killed by the operating system, and with it the whole test
    /// binary. The caller must disclose the skip — see
    /// [`tpch_wire_scope::report_wire_exclusions`].
    Skipped(WireSkip),
}

/// Run a single **standard TPC-H** query with timing.
///
/// The query id is mandatory on purpose: it is what makes the wire scope
/// enforceable at compile time. A statement outside the scope is **not**
/// executed — the runner returns [`WireOutcome::Skipped`] instead. Every
/// call site therefore has to name the statement it wants to run, and a
/// statement that is known to be unrunnable on the file-storage backend
/// cannot be executed by accident.
///
/// A statement that is *not* one of the 22 standard queries is executed
/// normally — the scope governs the standard set only. Use
/// [`run_custom_query_timed`] for such statements so the intent stays
/// visible at the call site.
pub fn run_query_timed(
    client: &mut MySqlTestClient,
    qid: &str,
    sql: &str,
    timeout_s: u64,
) -> WireOutcome {
    if tpch_wire_scope::is_standard_query(qid) && !tpch_wire_scope::in_wire_scope(qid) {
        let exclusion = tpch_wire_scope::wire_exclusion(qid);
        return WireOutcome::Skipped(WireSkip {
            qid: qid.to_string(),
            reason: exclusion
                .map(|e| e.reason.to_string())
                .unwrap_or_else(|| "unregistered".to_string()),
            issue: exclusion.map(|e| e.issue.to_string()).unwrap_or_default(),
            detail: exclusion.map(|e| e.detail.to_string()).unwrap_or_default(),
        });
    }
    let (result, elapsed) = run_custom_query_timed(client, sql, timeout_s);
    WireOutcome::Ran(result, elapsed)
}

/// Run a statement that is **not** one of the 22 standard TPC-H queries.
///
/// The wire scope does not apply here: it governs the standard set. The
/// separate function exists so that a call site says out loud that it is
/// running custom SQL.
pub fn run_custom_query_timed(
    client: &mut MySqlTestClient,
    sql: &str,
    timeout_s: u64,
) -> (Result<Vec<Vec<String>>, String>, Duration) {
    client
        .set_timeouts(
            Duration::from_secs(timeout_s),
            Duration::from_secs(timeout_s),
        )
        .expect("set_timeouts");
    let start = Instant::now();
    let result = client.query_rows(sql);
    let elapsed = start.elapsed();
    (result.map_err(|e| e.to_string()), elapsed)
}

/// Read SQLite/MariaDB/PG baseline JSON
pub fn read_baseline(path: &Path) -> JsonValue {
    let body =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("parse JSON {}: {}", path.display(), e))
}

/// Cell-level comparison with float tolerance.
pub fn compare_cells(
    actual: &[Vec<String>],
    baseline: &JsonValue,
    float_tol: f64,
) -> Result<(), String> {
    let engine = baseline
        .get("engines")
        .and_then(|e| e.get("sqlite"))
        .ok_or_else(|| "baseline missing engines.sqlite".to_string())?;

    let expected_rows = engine
        .get("row_count")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "baseline missing engines.sqlite.row_count".to_string())?;

    if actual.len() as u64 != expected_rows {
        return Err(format!(
            "row_count mismatch: actual={} expected={}",
            actual.len(),
            expected_rows
        ));
    }

    let first_3_raw = engine
        .get("first_3_rows")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "baseline missing engines.sqlite.first_3_rows".to_string())?;

    if first_3_raw.is_empty() {
        return Ok(());
    }

    let mut expected: Vec<Vec<String>> = first_3_raw
        .iter()
        .map(|row| {
            row.as_str()
                .map(|s| s.split('|').map(|c| c.to_string()).collect())
                .unwrap_or_default()
        })
        .collect();

    let compare_n = expected.len().min(3).min(actual.len());
    let mut actual_first: Vec<Vec<String>> = actual.iter().take(compare_n).cloned().collect();
    actual_first.sort();
    expected.sort();
    expected.truncate(compare_n);

    let cell_eq = |a: &str, b: &str| -> bool {
        if a == b {
            return true;
        }
        match (a.parse::<f64>(), b.parse::<f64>()) {
            (Ok(av), Ok(bv)) => {
                let diff = (av - bv).abs();
                let scale = av.abs().max(bv.abs()).max(1.0);
                diff <= float_tol * scale
            }
            _ => false,
        }
    };

    for (row_idx, (a_row, e_row)) in actual_first.iter().zip(expected.iter()).enumerate() {
        if a_row.len() != e_row.len() {
            return Err(format!(
                "row {}: column count mismatch: actual={} expected={} (actual={:?}, expected={:?})",
                row_idx,
                a_row.len(),
                e_row.len(),
                a_row,
                e_row
            ));
        }
        for (col_idx, (a, e)) in a_row.iter().zip(e_row.iter()).enumerate() {
            if !cell_eq(a, e) {
                return Err(format!(
                    "cell mismatch at row={} col={}: actual={:?} expected={:?}",
                    row_idx, col_idx, a, e
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{compare_cells, stage_fixture_for_wire, JsonValue, TABLES};
    use serde_json::json;

    /// RED (regression guard for the CI run 8117 failure).
    ///
    /// Every test of a binary runs in parallel and each of them stages the
    /// fixture into a directory **inside the shared fixture directory**.
    /// With a single, fixed directory name, one test's cleanup
    /// (`remove_dir_all`) deletes the directory another test is still
    /// copying into. CI run 8117 hit exactly that:
    ///
    /// ```text
    /// copy /tmp/tpch-fixture/.tpch-wire-staging/region.tbl
    ///   -> /tmp/tpch-fixture/.tpch-wire-staging/region.tbl:
    ///   No such file or directory (os error 2)
    /// ```
    ///
    /// The assertion below does not need the race to be timed: two
    /// consecutive stagings that return the **same** path are already the
    /// defect, and that is deterministic.
    #[test]
    fn consecutive_stagings_must_not_share_a_directory() {
        let dir = std::env::temp_dir().join(format!(
            "tpch-wire-staging-guard-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("t")
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the scratch fixture dir");
        for tbl in TABLES {
            std::fs::write(dir.join(format!("{tbl}.tbl")), "").expect("write the scratch table");
        }

        let first = stage_fixture_for_wire(&dir);
        let second = stage_fixture_for_wire(&dir);

        assert_ne!(
            first,
            second,
            "two stagings returned the same directory ({}) — parallel tests would \
             delete each other's staged data",
            first.display()
        );
        assert!(
            first.exists(),
            "the first staging was removed by the second one ({})",
            first.display()
        );

        let _ = std::fs::remove_dir_all(&first);
        let _ = std::fs::remove_dir_all(&second);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn baseline(row_count: u64, first_3_rows: &[&str]) -> JsonValue {
        json!({
            "engines": {
                "sqlite": {
                    "row_count": row_count,
                    "first_3_rows": first_3_rows,
                }
            }
        })
    }

    #[test]
    fn all_cells_match_returns_ok() {
        let actual = vec![
            vec!["A".into(), "1".into(), "1.5".into()],
            vec!["B".into(), "2".into(), "2.5".into()],
        ];
        let b = baseline(2, &["A|1|1.5", "B|2|2.5"]);
        assert!(compare_cells(&actual, &b, 1e-3).is_ok());
    }

    #[test]
    fn row_count_mismatch_returns_err_with_count_message() {
        let actual = vec![vec!["A".into()]];
        let b = baseline(2, &["A|x"]);
        let err = compare_cells(&actual, &b, 1e-3).unwrap_err();
        assert!(
            err.contains("row_count mismatch"),
            "expected row_count message, got: {err}"
        );
        assert!(err.contains("actual=1"), "got: {err}");
        assert!(err.contains("expected=2"), "got: {err}");
    }

    #[test]
    fn single_cell_mismatch_returns_err_with_position() {
        let actual = vec![
            vec!["A".into(), "1".into(), "1.5".into()],
            vec!["B".into(), "WRONG".into(), "2.5".into()],
        ];
        let b = baseline(2, &["A|1|1.5", "B|2|2.5"]);
        let err = compare_cells(&actual, &b, 1e-3).unwrap_err();
        assert!(
            err.contains("row=1") && err.contains("col=1"),
            "expected cell position row=1 col=1, got: {err}"
        );
        assert!(err.contains("WRONG"), "actual value missing in: {err}");
        assert!(err.contains("\"2\""), "expected value missing in: {err}");
    }

    #[test]
    fn float_tolerance_accepts_close_values() {
        let actual = vec![vec!["1.0001".into()]];
        let b = baseline(1, &["1.0"]);
        assert!(compare_cells(&actual, &b, 1e-3).is_ok());
    }

    #[test]
    fn float_tolerance_rejects_far_values() {
        let actual = vec![vec!["1.5".into()]];
        let b = baseline(1, &["1.0"]);
        let err = compare_cells(&actual, &b, 1e-3).unwrap_err();
        assert!(
            err.contains("cell mismatch"),
            "expected cell mismatch, got: {err}"
        );
    }

    #[test]
    fn empty_first_3_rows_skips_cell_check() {
        let actual: Vec<Vec<String>> = vec![];
        let b = baseline(0, &[]);
        assert!(compare_cells(&actual, &b, 1e-3).is_ok());
    }

    #[test]
    fn missing_engines_sqlite_returns_err() {
        let bad: JsonValue = json!({});
        let actual: Vec<Vec<String>> = vec![];
        let err = compare_cells(&actual, &bad, 1e-3).unwrap_err();
        assert!(err.contains("engines.sqlite"), "got: {err}");
    }

    #[test]
    fn column_count_mismatch_returns_err() {
        let actual = vec![vec!["A".into(), "1".into()]];
        let b = baseline(1, &["A|1|extra"]);
        let err = compare_cells(&actual, &b, 1e-3).unwrap_err();
        assert!(err.contains("column count mismatch"), "got: {err}");
    }
}
