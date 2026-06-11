//! Statistics Harness (P3-2 #3181)
//!
//! Shared utilities for ANALYZE TABLE statistics testing. Provides:
//! - `MockTableStats` — declarative test statistics (mirrors the real
//!   `crates/optimizer/src/stats.rs::TableStats`).
//! - `MockColumnStats` — per-column stats (mirrors `ColumnStats`).
//! - `run_analyze` — execute an analyze step and return an
//!   `AnalyzeReport` (collected stats, eq_selectivity, null_fraction).
//!
//! This file is **not** a test target itself (no `#[test]`); it is
//! shared by `statistics_test.rs`.

#![allow(dead_code)] // helpers consumed by test targets

use std::collections::HashMap;

/// Per-column statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct MockColumnStats {
    pub column_name: String,
    pub distinct_count: u64,
    pub null_count: u64,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub average: f64,
}

impl MockColumnStats {
    pub fn new(name: &str) -> Self {
        Self {
            column_name: name.into(),
            distinct_count: 0,
            null_count: 0,
            min_value: None,
            max_value: None,
            average: 0.0,
        }
    }

    pub fn with_distinct(mut self, count: u64) -> Self {
        self.distinct_count = count;
        self
    }

    pub fn with_null(mut self, count: u64) -> Self {
        self.null_count = count;
        self
    }

    pub fn with_range(mut self, min: i64, max: i64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }

    pub fn with_average(mut self, avg: f64) -> Self {
        self.average = avg;
        self
    }

    /// `eq_selectivity = 1 / distinct_count`, or 1.0 if distinct is 0.
    pub fn eq_selectivity(&self) -> f64 {
        if self.distinct_count == 0 {
            1.0
        } else {
            1.0 / self.distinct_count as f64
        }
    }

    /// `null_fraction = null_count / row_count`, or 0.0 if row_count is 0.
    pub fn null_fraction(&self, row_count: u64) -> f64 {
        if row_count == 0 {
            0.0
        } else {
            self.null_count as f64 / row_count as f64
        }
    }
}

/// Per-table statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct MockTableStats {
    pub table_name: String,
    pub row_count: u64,
    pub size_bytes: u64,
    pub columns: HashMap<String, MockColumnStats>,
    pub last_updated: u64,
}

impl MockTableStats {
    pub fn new(name: &str) -> Self {
        Self {
            table_name: name.into(),
            row_count: 0,
            size_bytes: 0,
            columns: HashMap::new(),
            last_updated: 0,
        }
    }

    pub fn with_row_count(mut self, count: u64) -> Self {
        self.row_count = count;
        self
    }

    pub fn with_size_bytes(mut self, bytes: u64) -> Self {
        self.size_bytes = bytes;
        self
    }

    pub fn add_column(mut self, col: MockColumnStats) -> Self {
        self.columns.insert(col.column_name.clone(), col);
        self
    }

    pub fn with_last_updated(mut self, ts: u64) -> Self {
        self.last_updated = ts;
        self
    }

    pub fn column(&self, name: &str) -> Option<&MockColumnStats> {
        self.columns.get(name)
    }

    pub fn estimate_selectivity(&self, column: &str) -> f64 {
        self.column(column)
            .map(|c| c.eq_selectivity())
            .unwrap_or(1.0)
    }
}

/// Result of running an analyze.
#[derive(Debug, Clone)]
pub struct AnalyzeReport {
    pub table_name: String,
    pub row_count: u64,
    pub column_count: u32,
    pub total_size_bytes: u64,
    pub min_eq_selectivity: f64,
    pub max_eq_selectivity: f64,
}

impl AnalyzeReport {
    pub fn passed(&self) -> bool {
        self.column_count > 0
            && self.min_eq_selectivity >= 0.0
            && self.min_eq_selectivity <= 1.0
            && self.max_eq_selectivity <= 1.0
    }
}

/// Run analyze on a table stats and produce a report.
pub fn run_analyze(stats: &MockTableStats) -> AnalyzeReport {
    let column_count = stats.columns.len() as u32;
    let min_selectivity = stats
        .columns
        .values()
        .map(|c| c.eq_selectivity())
        .fold(f64::INFINITY, f64::min);
    let max_selectivity = stats
        .columns
        .values()
        .map(|c| c.eq_selectivity())
        .fold(0.0_f64, f64::max);

    AnalyzeReport {
        table_name: stats.table_name.clone(),
        row_count: stats.row_count,
        column_count,
        total_size_bytes: stats.size_bytes,
        min_eq_selectivity: if min_selectivity.is_finite() {
            min_selectivity
        } else {
            1.0
        },
        max_eq_selectivity: max_selectivity,
    }
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    fn column_stats_eq_selectivity() {
        let c = MockColumnStats::new("age").with_distinct(100);
        assert!((c.eq_selectivity() - 0.01).abs() < 1e-9);
    }

    #[test]
    fn column_stats_eq_selectivity_zero_distinct() {
        let c = MockColumnStats::new("age"); // distinct = 0
        assert_eq!(c.eq_selectivity(), 1.0);
    }

    #[test]
    fn column_stats_null_fraction() {
        let c = MockColumnStats::new("x").with_null(25);
        assert!((c.null_fraction(100) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn table_stats_estimate_selectivity() {
        let t = MockTableStats::new("users")
            .with_row_count(100)
            .add_column(MockColumnStats::new("id").with_distinct(100));
        assert!((t.estimate_selectivity("id") - 0.01).abs() < 1e-9);
    }

    #[test]
    fn run_analyze_basic_report() {
        let t = MockTableStats::new("users")
            .with_row_count(1000)
            .with_size_bytes(8192)
            .add_column(MockColumnStats::new("id").with_distinct(1000))
            .add_column(MockColumnStats::new("age").with_distinct(50));
        let r = run_analyze(&t);
        assert_eq!(r.row_count, 1000);
        assert_eq!(r.column_count, 2);
        assert_eq!(r.total_size_bytes, 8192);
        assert!(r.passed());
    }
}
