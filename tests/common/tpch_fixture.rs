//! Unified TPC-H fixture resolution for test targets (issue #5379).
//!
//! # Why this module exists
//!
//! Before this module, every fixture-dependent test resolved its own data
//! directory, and the two committed "fixtures" were **symlinks pointing outside
//! the repository**:
//!
//! ```text
//! 120000 tests/data/tpch-sf001 -> /tmp/tpch-sf001
//! 120000 tests/data/tpch-sf01  -> /tmp/tpch-sf01
//! ```
//!
//! An absolute `/tmp` path only resolves on the machine that created it, so
//! every one of these tests panicked in CI with
//! `panicked at tests/integration/oracle/diag_q11.rs:16:45` — while
//! `cargo test`'s default fail-fast stopped the run, leaving 382 of the 446
//! declared test targets never executed at all (measured on CI run 7613,
//! log `/tmp/testjob.log`).
//!
//! This module replaces that ad-hoc resolution with one rule:
//!
//! 1. `TPCH_DATA_DIR` (set by CI, see `scripts/gate/prepare_tpch_fixture.sh`)
//! 2. then a small set of repository-local candidate directories
//! 3. otherwise the fixture is **missing** — and a missing fixture is an
//!    *explicit, disclosed state*, never a panic and never a silent pass.
//!
//! # The three states
//!
//! | state | meaning | CI accounting |
//! |---|---|---|
//! | `Present` | fixture found and complete | test may assert |
//! | `Missing` | no fixture directory | `not_run` (never `passed`) |
//! | `ProfileMismatch` | fixture found, but its scale profile is not the one the test's expectations were calibrated against | `not_run` (never `passed`) |
//!
//! A test that cannot run prints a machine-readable marker:
//!
//! ```text
//! TPCH-NOT-RUN: <cargo-target> reason=<code> detail=<...>
//! ```
//!
//! CI subtracts these from the `passed` count and reports them separately, as
//! required by ADR-008 §Policy 1 (every PASS claim must be qualified with the
//! number of tests that actually ran). `AGENTS.md §8` #5 forbids counting a
//! SKIP as a PASS — that is exactly why these markers are counted separately
//! instead of being silently green.
//!
//! # Example
//!
//! ```no_run
//! #[path = "../../common/tpch_fixture.rs"]
//! mod tpch_fixture;
//! use tpch_fixture::FixtureState;
//!
//! #[test]
//! fn q11_having() {
//!     let state = FixtureState::resolve();
//!     let Some(fixture) = state.require("diag_q11") else { return };
//!     // ... use `fixture` ...
//! }
//! ```

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use sqlrustgo::{MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;

/// Marker prefix printed by a test that could not run.
///
/// CI counts these occurrences and reports them as `not_run`; they are
/// **never** counted as `passed`.
pub const NOT_RUN_MARKER: &str = "TPCH-NOT-RUN";

/// Marker prefix printed when an engine/oracle mismatch is already registered
/// in `tests/baseline/tpch_known_differences.json`.
pub const KNOWN_DIFF_MARKER: &str = "TPCH-KNOWN-DIFF";

/// The eight TPC-H tables, in dependency order.
pub const TPC_H_TABLES: [&str; 8] = [
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

/// Controlled reason codes for [`NOT_RUN_MARKER`].
pub mod reason {
    /// No fixture directory could be found.
    pub const FIXTURE_MISSING: &str = "fixture_missing";
    /// The fixture exists but its scale profile differs from the profile the
    /// test's expected values were calibrated against.
    pub const PROFILE_MISMATCH: &str = "profile_mismatch";
    /// The test is only meaningful at SF=1 scale (the G4 scale layer).
    pub const SCALE_LAYER_ONLY: &str = "scale_layer_only";
    /// The engine differs from the independent oracle, and the difference is
    /// registered (with an issue and an owner) in the known-difference
    /// registry.
    pub const KNOWN_DIFFERENCE: &str = "known_difference";
    /// The check needs an **external service** (a MySQL / PostgreSQL server)
    /// that the environment does not provide.
    ///
    /// A distinct reason from [`FIXTURE_MISSING`] on purpose: the fixture is
    /// present and the test is ready to run — what is missing is a server
    /// that only the CI job could start.
    pub const EXTERNAL_SERVICE_UNAVAILABLE: &str = "external_service_unavailable";
    /// The check needs a **pre-built local artefact** (a WAL directory, a
    /// pre-built SQLite database) that neither the repository content nor
    /// any CI step produces.
    ///
    /// Same reasoning as [`EXTERNAL_SERVICE_UNAVAILABLE`]: such a test can
    /// say nothing about the code here, so it must be counted as *not run*
    /// (and disclosed), never as *passed*.
    pub const PREBUILT_DATA_UNAVAILABLE: &str = "prebuilt_data_unavailable";
}

/// The case id used for a statement-level engine-vs-oracle difference.
///
/// It is deliberately **independent of the test target**: a difference that
/// comes from the engine's SQL semantics (rather than from one test's
/// setup) has to be registered once, or it would have to be registered
/// again for every target that happens to run the same statement.
pub fn statement_case_id(qid: &str) -> String {
    format!("tpch:{qid}")
}

/// Repository-local candidate fixture directories, tried in order when
/// `TPCH_DATA_DIR` is not set.
///
/// Note the deliberate absence of `tests/data/tpch-sf01` and
/// `tests/data/tpch-sf001`: those were symlinks into `/tmp` and never
/// resolved anywhere but on one author's machine. They were removed by this
/// change — a dangling symlink is worse than no fixture, because it claims
/// data exists.
const CANDIDATE_DIRS: [&str; 2] = ["tests/data/tpch-fixture", "data/tpch-sf01"];

/// Name of the manifest file written next to the generated fixture.
pub const MANIFEST_FILE: &str = "tpch_fixture_manifest.json";

// =========================================================================
// Manifest
// =========================================================================

/// Metadata describing the fixture that is actually on disk.
///
/// Written by `scripts/gate/prepare_tpch_fixture.sh`. The `sf` value is what
/// lets a test tell "the data I have" from "the data my expectations were
/// calibrated against".
#[derive(Debug, Clone, Default)]
pub struct Manifest {
    /// Generator that produced the fixture.
    pub generator: String,
    /// Random seed used by the generator.
    pub seed: i64,
    /// Scale factor. `0.0` means "unknown" (no manifest present).
    pub sf: f64,
    /// Actual row count per table, measured from the generated files.
    pub row_counts: BTreeMap<String, u64>,
}

impl Manifest {
    /// Read the manifest next to `dir`, if any. A missing or malformed
    /// manifest yields a default manifest (`sf` unknown) rather than an
    /// error: the fixture is still usable for data-independent assertions.
    pub fn load(dir: &Path) -> Manifest {
        let path = dir.join(MANIFEST_FILE);
        match std::fs::read_to_string(&path) {
            Ok(raw) => Manifest::parse(&raw).unwrap_or_else(|e| {
                eprintln!(
                    "TPCH-FIXTURE-WARN: manifest {} is not parseable ({e}); \
                     treating scale profile as unknown",
                    path.display()
                );
                Manifest::default()
            }),
            Err(_) => Manifest::default(),
        }
    }

    /// Parse a manifest JSON document.
    pub fn parse(raw: &str) -> Result<Manifest, String> {
        let v: serde_json::Value =
            serde_json::from_str(raw).map_err(|e| format!("manifest is not valid JSON: {e}"))?;
        let seed = v
            .get("seed")
            .and_then(|s| s.as_i64())
            .ok_or("manifest is missing integer field `seed`")?;
        let sf = v
            .get("sf")
            .and_then(|s| s.as_f64())
            .ok_or("manifest is missing numeric field `sf`")?;
        let mut row_counts = BTreeMap::new();
        if let Some(obj) = v.get("row_counts").and_then(|s| s.as_object()) {
            for (k, val) in obj {
                if let Some(n) = val.as_u64() {
                    row_counts.insert(k.clone(), n);
                }
            }
        }
        Ok(Manifest {
            generator: v
                .get("generator")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            seed,
            sf,
            row_counts,
        })
    }
}

// =========================================================================
// Fixture
// =========================================================================

/// A resolved TPC-H fixture directory.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// Directory holding the `.tbl` files.
    pub dir: PathBuf,
    /// Metadata about the fixture.
    pub manifest: Manifest,
}

impl Fixture {
    /// Path of one table's `.tbl` file.
    pub fn table_path(&self, table: &str) -> PathBuf {
        self.dir.join(format!("{table}.tbl"))
    }

    /// Whether a table file exists.
    pub fn has_table(&self, table: &str) -> bool {
        self.table_path(table).is_file()
    }

    /// Read one table's raw `.tbl` text.
    pub fn read_table(&self, table: &str) -> Result<String, String> {
        let path = self.table_path(table);
        std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    }

    /// Tables from [`TPC_H_TABLES`] that are absent from this fixture.
    pub fn missing_tables(&self) -> Vec<&'static str> {
        TPC_H_TABLES
            .iter()
            .filter(|t| !self.has_table(t))
            .copied()
            .collect()
    }

    /// Row count of a table, as recorded in the manifest.
    pub fn row_count(&self, table: &str) -> Option<u64> {
        self.manifest.row_counts.get(table).copied()
    }

    /// The scale factor of this fixture (`0.0` when unknown).
    pub fn sf(&self) -> f64 {
        self.manifest.sf
    }

    /// Human-readable profile label, e.g. `sf0.01` or `unknown`.
    pub fn profile(&self) -> String {
        if self.manifest.sf == 0.0 {
            "unknown".to_string()
        } else {
            format!("sf{}", self.manifest.sf)
        }
    }

    /// Parse one table into trimmed field vectors.
    pub fn load_rows(&self, table: &str) -> Result<Vec<Vec<String>>, String> {
        Ok(parse_tbl(&self.read_table(table)?))
    }
}

/// Split a TPC-H `.tbl` document into trimmed field vectors.
///
/// Empty lines are skipped; the trailing `|` of a TPC-H record yields a final
/// empty field, which is preserved (callers pick the columns they need).
pub fn parse_tbl(content: &str) -> Vec<Vec<String>> {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.split('|').map(|f| f.trim().to_string()).collect())
        .collect()
}

// =========================================================================
// Fixture state
// =========================================================================

/// Outcome of fixture resolution.
#[derive(Debug, Clone)]
pub enum FixtureState {
    /// A usable fixture was found.
    Present(Fixture),
    /// No fixture could be found.
    Missing {
        /// Human-readable explanation.
        detail: String,
    },
    /// A fixture was found, but not at the profile the test expects.
    ProfileMismatch {
        /// The profile the test was calibrated against.
        expected: String,
        /// The profile actually on disk.
        actual: String,
        /// Human-readable explanation.
        detail: String,
    },
}

impl FixtureState {
    /// Resolve the fixture for the current process.
    ///
    /// Order: `TPCH_DATA_DIR` → repository-local candidates → missing.
    pub fn resolve() -> FixtureState {
        match std::env::var("TPCH_DATA_DIR") {
            Ok(dir) if !dir.trim().is_empty() => {
                let path = PathBuf::from(dir.trim());
                if !path.is_dir() {
                    return FixtureState::Missing {
                        detail: format!("TPCH_DATA_DIR={} is not a directory", path.display()),
                    };
                }
                FixtureState::Present(Fixture {
                    manifest: Manifest::load(&path),
                    dir: path,
                })
            }
            Ok(_) => FixtureState::Missing {
                detail: "TPCH_DATA_DIR is set but empty".to_string(),
            },
            Err(_) => {
                for cand in CANDIDATE_DIRS {
                    let path = PathBuf::from(cand);
                    if path.is_dir() {
                        return FixtureState::Present(Fixture {
                            manifest: Manifest::load(&path),
                            dir: path,
                        });
                    }
                }
                FixtureState::Missing {
                    detail: format!(
                        "no TPC-H fixture found (TPCH_DATA_DIR unset; tried {})",
                        CANDIDATE_DIRS.join(", ")
                    ),
                }
            }
        }
    }

    /// Return the fixture, or report `NOT-RUN` and return `None`.
    pub fn require(&self, target: &str) -> Option<&Fixture> {
        match self {
            FixtureState::Present(f) => Some(f),
            FixtureState::Missing { detail } => {
                report_not_run(target, reason::FIXTURE_MISSING, detail);
                None
            }
            FixtureState::ProfileMismatch {
                expected,
                actual,
                detail,
            } => {
                report_not_run(target, reason::PROFILE_MISMATCH, detail);
                let _ = (expected, actual);
                None
            }
        }
    }

    /// Return the fixture only if its scale profile matches `expected_sf`.
    ///
    /// A fixture whose manifest is absent has an *unknown* profile; that is
    /// treated as a mismatch, because a test calibrated against SF=1 data
    /// cannot be trusted to hold against an unidentified dataset.
    pub fn require_profile(&self, target: &str, expected_sf: f64) -> Option<&Fixture> {
        match self {
            FixtureState::Present(f) => {
                if f.sf() == expected_sf {
                    Some(f)
                } else {
                    report_not_run(
                        target,
                        reason::PROFILE_MISMATCH,
                        &format!(
                            "expected scale factor {} but fixture profile is {}",
                            expected_sf,
                            f.profile()
                        ),
                    );
                    None
                }
            }
            FixtureState::Missing { detail } => {
                report_not_run(target, reason::FIXTURE_MISSING, detail);
                None
            }
            FixtureState::ProfileMismatch { detail, .. } => {
                report_not_run(target, reason::PROFILE_MISMATCH, detail);
                None
            }
        }
    }
}

/// Print the machine-readable NOT-RUN marker.
///
/// Also appends to the file named by `TPCH_NOT_RUN_FILE` when that variable
/// is set, so a CI summary can count not-run tests without parsing the whole
/// cargo log.
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

// =========================================================================
// Storage loading (shared by every fixture-dependent test)
// =========================================================================

/// Load one table into an in-memory storage under `table`.
///
/// `ncols` is the number of leading columns to keep, matching the TPC-H
/// schema column order used by the tests. Values are typed as
/// integer / float / text, with empty fields becoming NULL.
pub fn load_into_storage(
    storage: &Arc<RwLock<MemoryStorage>>,
    fixture: &Fixture,
    table: &str,
    ncols: usize,
) -> Result<usize, String> {
    let content = fixture.read_table(table)?;
    load_rows_into_storage(storage, &parse_tbl(&content), table, ncols)
}

/// Load already-parsed rows into an in-memory storage under `table`.
///
/// The insert error is **not** swallowed. A failed batch insert leaves the
/// table empty, and every assertion of the calling test would then run
/// against an empty table and pass vacuously — which is precisely the
/// failure mode this loader exists to prevent. The row count returned is
/// the number of rows that were actually handed to the storage layer.
pub fn load_rows_into_storage(
    storage: &Arc<RwLock<MemoryStorage>>,
    rows: &[Vec<String>],
    table: &str,
    ncols: usize,
) -> Result<usize, String> {
    const BATCH_SIZE: usize = 10000;
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0usize;

    let mut flush = |storage: &Arc<RwLock<MemoryStorage>>,
                     batch: &mut Vec<Vec<SqlValue>>|
     -> Result<(), String> {
        if batch.is_empty() {
            return Ok(());
        }
        let n = batch.len();
        // Move the batch out instead of cloning it: a clone of 10,000 rows
        // per batch is pure overhead on the hot path.
        let data = std::mem::replace(batch, Vec::with_capacity(BATCH_SIZE));
        {
            let mut s = storage.write();
            s.insert(table, data)
                .map_err(|e| format!("insert into `{table}`: {e}"))?;
        }
        count += n;
        Ok(())
    };

    for values in rows {
        if values.len() < ncols {
            continue;
        }
        let record: Vec<SqlValue> = values[..ncols].iter().map(|v| parse_value(v)).collect();
        batch.push(record);
        if batch.len() >= BATCH_SIZE {
            flush(storage, &mut batch)?;
        }
    }
    flush(storage, &mut batch)?;
    Ok(count)
}

/// Type one raw TPC-H field: empty → NULL, integer → Integer, float → Float,
/// otherwise → Text.
pub fn parse_value(s: &str) -> SqlValue {
    let t = s.trim();
    if t.is_empty() {
        SqlValue::Null
    } else if let Ok(i) = t.parse::<i64>() {
        SqlValue::Integer(i)
    } else if let Ok(f) = t.parse::<f64>() {
        SqlValue::Float(f)
    } else {
        SqlValue::Text(t.to_string())
    }
}

// =========================================================================
// Known-difference registry
// =========================================================================

/// One registered engine-vs-oracle difference.
#[derive(Debug, Clone)]
pub struct KnownDifference {
    /// Stable identifier.
    ///
    /// For a statement-level difference that comes from the engine's SQL
    /// semantics it is [`statement_case_id`], e.g. `tpch:Q13` — the same in
    /// every target, so the difference is registered once rather than once
    /// per test. For a target-specific difference it is
    /// `<cargo-target>:<query_id>`.
    pub case_id: String,
    /// Cargo test target the difference was observed in, or `*` when the
    /// difference is inherent to the engine and therefore applies to every
    /// target that runs the statement.
    pub target: String,
    /// Short query identifier.
    pub query_id: String,
    /// What kind of difference this is.
    ///
    /// * `row_count` — the two sides returned a different number of rows;
    /// * `cell_values` — the two sides returned the same number of rows but
    ///   different values. (A `row_count` entry whose two counts are equal
    ///   is stale and is rejected by the registry gate; a
    ///   `cell_values` entry legitimately has equal counts.)
    pub kind: String,
    /// Row count the test expects.
    pub expected_row_count: i64,
    /// Row count the engine produced.
    pub actual_row_count: i64,
    /// Row count the independent oracle produced.
    pub oracle_row_count: i64,
    /// Owning issue.
    pub issue: String,
    /// Owner of the difference.
    pub owner: String,
    /// Date the difference was registered.
    pub since: String,
    /// What the difference actually is.
    pub detail: String,
}

/// Path of the known-difference registry, relative to the package root.
pub const KNOWN_DIFF_REGISTRY: &str = "tests/baseline/tpch_known_differences.json";

/// Load the known-difference registry. A missing or malformed registry yields
/// an empty list — which means every mismatch fails the test loudly.
pub fn load_known_differences() -> Vec<KnownDifference> {
    let raw = match std::fs::read_to_string(KNOWN_DIFF_REGISTRY) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("TPCH-FIXTURE-WARN: known-difference registry is not valid JSON: {e}");
            return Vec::new();
        }
    };
    let entries = v
        .get("entries")
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default();
    entries
        .iter()
        .filter_map(|e| {
            Some(KnownDifference {
                case_id: e.get("case_id")?.as_str()?.to_string(),
                target: e.get("target")?.as_str()?.to_string(),
                query_id: e.get("query_id")?.as_str()?.to_string(),
                kind: e
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("row_count")
                    .to_string(),
                expected_row_count: e.get("expected_row_count")?.as_i64()?,
                actual_row_count: e.get("actual_row_count")?.as_i64()?,
                oracle_row_count: e.get("oracle_row_count")?.as_i64()?,
                issue: e.get("issue")?.as_str()?.to_string(),
                owner: e.get("owner")?.as_str()?.to_string(),
                since: e.get("since")?.as_str()?.to_string(),
                detail: e
                    .get("detail")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect()
}

/// Look up a registered difference by `case_id`.
pub fn lookup_known_difference(case_id: &str) -> Option<KnownDifference> {
    load_known_differences()
        .into_iter()
        .find(|d| d.case_id == case_id)
}

/// Look up the registered difference for a TPC-H statement.
///
/// The lookup is target-independent (see [`statement_case_id`]): a
/// difference that stems from the engine's SQL semantics is registered
/// once and applies wherever the statement is executed.
pub fn lookup_statement_known_difference(qid: &str) -> Option<KnownDifference> {
    lookup_known_difference(&statement_case_id(qid))
}

/// Print the known-difference marker for a registered mismatch.
///
/// The registry entry is echoed in full (issue, owner, date) so that a reader
/// of the CI log can tell *which* tracked defect the run is tolerating.
pub fn report_known_difference(entry: &KnownDifference, detail: &str) {
    println!(
        "{KNOWN_DIFF_MARKER}: {} kind={} issue={} owner={} since={} registered_detail={} observed={}",
        entry.case_id, entry.kind, entry.issue, entry.owner, entry.since, entry.detail, detail
    );
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tbl_splits_and_trims() {
        let rows = parse_tbl("1|2|3|\n4|5|6|\n\n  7|8|9|  \n");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["1", "2", "3", ""]);
        assert_eq!(rows[2], vec!["7", "8", "9", ""]);
    }

    #[test]
    fn parse_value_types_fields() {
        assert!(matches!(parse_value(""), SqlValue::Null));
        assert!(matches!(parse_value("42"), SqlValue::Integer(42)));
        assert!(matches!(parse_value("4.5"), SqlValue::Float(_)));
        assert!(matches!(parse_value("abc"), SqlValue::Text(_)));
    }

    #[test]
    fn manifest_roundtrip() {
        let raw = r#"{"generator":"g","seed":42,"sf":0.01,"row_counts":{"lineitem":60000}}"#;
        let m = Manifest::parse(raw).expect("manifest parses");
        assert_eq!(m.seed, 42);
        assert!((m.sf - 0.01).abs() < 1e-12);
        assert_eq!(m.row_counts.get("lineitem"), Some(&60_000));
    }

    #[test]
    fn manifest_rejects_missing_fields() {
        assert!(Manifest::parse(r#"{"seed":42}"#).is_err());
        assert!(Manifest::parse("not json").is_err());
    }

    #[test]
    fn missing_state_reports_not_run() {
        let state = FixtureState::Missing {
            detail: "unit test".to_string(),
        };
        assert!(state.require("some_target").is_none());
    }

    #[test]
    fn statement_case_id_is_target_independent() {
        assert_eq!(statement_case_id("Q13"), "tpch:Q13");
    }
}
