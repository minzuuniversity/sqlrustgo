//! P15 Oracle Required (V4 fix) — 通用 oracle 验证框架
//!
//! 为 G1/G2/G3/G5/G9/G11/G12/G15/G16 提供独立 oracle 对比, 关闭 ADR-006 V4 漏洞.
//! 设计原则见 docs/plans/2026-06-18-oracle-framework-ga-push.md.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Value(pub String);

impl Value {
    pub fn from_row_cell(cell: &str) -> Self {
        let trimmed = cell.trim().trim_matches('\'').trim_matches('"').to_string();
        Value(trimmed)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row(pub Vec<Value>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowSet {
    pub query: String,
    pub row_count: usize,
    pub rows: Vec<Row>,
    pub wall_time_ms: u64,
}

impl RowSet {
    pub fn empty(query: &str) -> Self {
        Self {
            query: query.to_string(),
            row_count: 0,
            rows: vec![],
            wall_time_ms: 0,
        }
    }

    pub fn assert_set_eq(&self, other: &RowSet) -> Result<(), String> {
        if self.row_count != other.row_count {
            return Err(format!(
                "row_count mismatch: engine={} oracle={}",
                self.row_count, other.row_count
            ));
        }

        let mut s_rows = self.rows.clone();
        let mut o_rows = other.rows.clone();
        s_rows.sort_by(|a, b| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)));
        o_rows.sort_by(|a, b| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)));

        for (i, (s, o)) in s_rows.iter().zip(o_rows.iter()).enumerate() {
            if s != o {
                return Err(format!(
                    "row {} mismatch:\n  engine: {:?}\n  oracle:  {:?}",
                    i, s.0, o.0
                ));
            }
        }
        Ok(())
    }
}

pub fn sha256_capture(rs: &RowSet) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rs.row_count.to_le_bytes());
    for row in &rs.rows {
        for cell in &row.0 {
            hasher.update(cell.0.as_bytes());
            hasher.update(b"|");
        }
        hasher.update(b"\n");
    }
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sha256Baseline {
    pub generated_at: String,
    pub scale_factor: String,
    pub queries: Vec<Sha256QueryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sha256QueryEntry {
    pub query_id: String,
    pub row_count: usize,
    pub sha256: String,
    pub wall_time_ms: u64,
}

impl Sha256Baseline {
    pub fn load_or_warn(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!(
                "Baseline not found: {}. Run with --generate-baseline to create.",
                path.display()
            ));
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Read failed: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Parse failed: {}", e))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialize failed: {}", e))?;
        std::fs::write(path, content)
            .map_err(|e| format!("Write failed: {}", e))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DiffReport {
    pub matched: usize,
    pub differed: usize,
    pub missing_in_baseline: Vec<String>,
    pub missing_in_actual: Vec<String>,
    pub diffs: Vec<String>,
}

impl DiffReport {
    pub fn is_clean(&self) -> bool {
        self.differed == 0 && self.missing_in_baseline.is_empty() && self.missing_in_actual.is_empty()
    }
}

pub fn compare_to_baseline(
    actual: &RowSet,
    baseline_path: &Path,
) -> Result<DiffReport, String> {
    let content = std::fs::read_to_string(baseline_path)
        .map_err(|e| format!("Read baseline failed: {}", e))?;
    let baseline: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Parse baseline failed: {}", e))?;

    let expected_count = baseline
        .get("row_count")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "baseline missing 'row_count'".to_string())? as usize;

    let mut report = DiffReport {
        matched: 0,
        differed: 0,
        missing_in_baseline: vec![],
        missing_in_actual: vec![],
        diffs: vec![],
    };

    if actual.row_count != expected_count {
        report.differed += 1;
        report.diffs.push(format!(
            "row_count: actual={} baseline={}",
            actual.row_count, expected_count
        ));
    } else {
        report.matched += 1;
    }

    Ok(report)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfBaseline {
    pub workload: String,
    pub min_qps: f64,
    pub max_latency_p99_ms: f64,
    pub generated_at: String,
}

pub fn assert_perf_within(
    actual_qps: f64,
    baseline: &PerfBaseline,
    tolerance_pct: f64,
) -> Result<(), String> {
    let lower = baseline.min_qps * (1.0 - tolerance_pct / 100.0);
    let upper = baseline.min_qps * (1.0 + tolerance_pct / 100.0);

    if actual_qps < lower {
        return Err(format!(
            "QPS regression: actual={:.2} qps < baseline={:.2} qps ({}% tolerance, lower bound={:.2})",
            actual_qps, baseline.min_qps, tolerance_pct, lower
        ));
    }

    if actual_qps > upper {
        eprintln!(
            "[INFO] QPS improvement: actual={:.2} qps > baseline={:.2} qps ({}% over)",
            actual_qps, baseline.min_qps, tolerance_pct
        );
    }

    Ok(())
}

pub fn time_query<F, R>(f: F) -> (R, Duration)
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = f();
    (result, start.elapsed())
}

pub const ORACLE_BASELINE_DIR: &str = "tests/oracle/baselines";
pub const P15_ORACLE_BASELINE_FILE: &str = "tests/baseline/oracle_baseline.json";
pub const TPC_H_SHA256_BASELINE_FILE: &str = "tests/oracle/baselines/tpch_sha256.json";
pub const PERF_BASELINE_DIR: &str = "tests/oracle/baselines/perf";
pub const COMPAT_BASELINE_FILE: &str = "tests/oracle/baselines/compat_v3.8_v3.9.json";

pub const DEFAULT_PERF_TOLERANCE_PCT: f64 = 20.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_capture_deterministic() {
        let rs1 = RowSet {
            query: "Q1".into(),
            row_count: 2,
            rows: vec![
                Row(vec![Value("A".into()), Value("F".into())]),
                Row(vec![Value("N".into()), Value("O".into())]),
            ],
            wall_time_ms: 0,
        };
        let rs2 = RowSet {
            query: "Q1".into(),
            row_count: 2,
            rows: vec![
                Row(vec![Value("A".into()), Value("F".into())]),
                Row(vec![Value("N".into()), Value("O".into())]),
            ],
            wall_time_ms: 999,
        };
        assert_eq!(sha256_capture(&rs1), sha256_capture(&rs2));
    }

    #[test]
    fn sha256_capture_detects_drift() {
        let rs1 = RowSet {
            query: "Q1".into(),
            row_count: 2,
            rows: vec![
                Row(vec![Value("A".into())]),
                Row(vec![Value("B".into())]),
            ],
            wall_time_ms: 0,
        };
        let rs2 = RowSet {
            query: "Q1".into(),
            row_count: 2,
            rows: vec![
                Row(vec![Value("A".into())]),
                Row(vec![Value("C".into())]),
            ],
            wall_time_ms: 0,
        };
        assert_ne!(sha256_capture(&rs1), sha256_capture(&rs2));
    }

    #[test]
    fn row_set_assert_set_eq_order_independent() {
        let a = RowSet {
            query: "Q".into(),
            row_count: 2,
            rows: vec![
                Row(vec![Value("1".into())]),
                Row(vec![Value("2".into())]),
            ],
            wall_time_ms: 0,
        };
        let b = RowSet {
            query: "Q".into(),
            row_count: 2,
            rows: vec![
                Row(vec![Value("2".into())]),
                Row(vec![Value("1".into())]),
            ],
            wall_time_ms: 0,
        };
        assert!(a.assert_set_eq(&b).is_ok());
    }

    #[test]
    fn row_set_assert_set_eq_count_mismatch() {
        let a = RowSet {
            query: "Q".into(),
            row_count: 2,
            rows: vec![
                Row(vec![Value("1".into())]),
                Row(vec![Value("2".into())]),
            ],
            wall_time_ms: 0,
        };
        let b = RowSet {
            query: "Q".into(),
            row_count: 1,
            rows: vec![Row(vec![Value("1".into())])],
            wall_time_ms: 0,
        };
        assert!(a.assert_set_eq(&b).is_err());
    }

    #[test]
    fn perf_within_tolerance_pass() {
        let baseline = PerfBaseline {
            workload: "point_select".into(),
            min_qps: 1000.0,
            max_latency_p99_ms: 10.0,
            generated_at: "2026-06-18".into(),
        };
        assert!(assert_perf_within(950.0, &baseline, 20.0).is_ok());
    }

    #[test]
    fn perf_within_tolerance_fail() {
        let baseline = PerfBaseline {
            workload: "point_select".into(),
            min_qps: 1000.0,
            max_latency_p99_ms: 10.0,
            generated_at: "2026-06-18".into(),
        };
        assert!(assert_perf_within(700.0, &baseline, 20.0).is_err());
    }
}
