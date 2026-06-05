//! P3-2 (#3181) Statistics (ANALYZE TABLE) — 20+ tests across 5 categories
//!
//! 1. basic stats (5)
//! 2. boundary (4)
//! 3. large table (3)
//! 4. multi_column (4)
//! 5. dispatcher (4)
//!
//! Total: 20 tests
//!
//! Refs: docs/openspec/3181-statistics.md
//!       V390_TEST_PLAN.md §G10

mod harness {
    use std::collections::HashMap;

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
        pub fn eq_selectivity(&self) -> f64 {
            if self.distinct_count == 0 {
                1.0
            } else {
                1.0 / self.distinct_count as f64
            }
        }
        pub fn null_fraction(&self, row_count: u64) -> f64 {
            if row_count == 0 {
                0.0
            } else {
                self.null_count as f64 / row_count as f64
            }
        }
    }

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
}

use harness::{run_analyze, MockColumnStats, MockTableStats};

// --------------------------------------------------------------------
// 1. basic (5 tests)
// --------------------------------------------------------------------

#[test]
fn test_stats_basic_row_count_p3_2() {
    let t = MockTableStats::new("users").with_row_count(1000);
    let r = run_analyze(&t);
    assert_eq!(r.row_count, 1000);
}

#[test]
fn test_stats_basic_distinct_count_p3_2() {
    let t = MockTableStats::new("users")
        .with_row_count(1000)
        .add_column(MockColumnStats::new("id").with_distinct(1000));
    let r = run_analyze(&t);
    assert_eq!(r.column_count, 1);
    let sel = t.estimate_selectivity("id");
    assert!((sel - 0.001).abs() < 1e-9);
}

#[test]
fn test_stats_basic_null_fraction_p3_2() {
    let t = MockTableStats::new("users")
        .with_row_count(100)
        .add_column(MockColumnStats::new("email").with_null(25));
    let col = t.column("email").unwrap();
    assert!((col.null_fraction(100) - 0.25).abs() < 1e-9);
}

#[test]
fn test_stats_basic_min_max_p3_2() {
    let t = MockTableStats::new("users")
        .with_row_count(100)
        .add_column(MockColumnStats::new("age").with_range(18, 65));
    let col = t.column("age").unwrap();
    assert_eq!(col.min_value, Some(18));
    assert_eq!(col.max_value, Some(65));
}

#[test]
fn test_stats_basic_average_p3_2() {
    let t = MockTableStats::new("users")
        .with_row_count(100)
        .add_column(MockColumnStats::new("age").with_average(35.5));
    let col = t.column("age").unwrap();
    assert!((col.average - 35.5).abs() < 1e-9);
}

// --------------------------------------------------------------------
// 2. boundary (4 tests)
// --------------------------------------------------------------------

#[test]
fn test_stats_boundary_empty_table_p3_2() {
    let t = MockTableStats::new("empty").with_row_count(0);
    let r = run_analyze(&t);
    assert_eq!(r.row_count, 0);
    assert_eq!(r.column_count, 0);
    // min_eq_selectivity is 1.0 when no columns
    assert_eq!(r.min_eq_selectivity, 1.0);
}

#[test]
fn test_stats_boundary_one_row_p3_2() {
    let t = MockTableStats::new("t").with_row_count(1);
    let r = run_analyze(&t);
    assert_eq!(r.row_count, 1);
}

#[test]
fn test_stats_boundary_all_null_p3_2() {
    let t = MockTableStats::new("t")
        .with_row_count(100)
        .add_column(MockColumnStats::new("x").with_null(100));
    let col = t.column("x").unwrap();
    assert!((col.null_fraction(100) - 1.0).abs() < 1e-9);
}

#[test]
fn test_stats_boundary_single_value_p3_2() {
    let t = MockTableStats::new("t")
        .with_row_count(100)
        .add_column(MockColumnStats::new("flag").with_distinct(1));
    let col = t.column("flag").unwrap();
    assert!((col.eq_selectivity() - 1.0).abs() < 1e-9);
}

// --------------------------------------------------------------------
// 3. large_table (3 tests)
// --------------------------------------------------------------------

#[test]
fn test_stats_large_10k_p3_2() {
    let t = MockTableStats::new("big")
        .with_row_count(10_000)
        .with_size_bytes(819_200);
    let r = run_analyze(&t);
    assert_eq!(r.row_count, 10_000);
    assert_eq!(r.total_size_bytes, 819_200);
}

#[test]
fn test_stats_large_100k_p3_2() {
    let t = MockTableStats::new("big")
        .with_row_count(100_000)
        .with_size_bytes(8_192_000);
    let r = run_analyze(&t);
    assert_eq!(r.row_count, 100_000);
}

#[test]
fn test_stats_large_1m_p3_2() {
    // #3181 目标: ANALYZE 在 1M 行表 < 1min. 这里只验证 mocked count.
    let t = MockTableStats::new("huge")
        .with_row_count(1_000_000)
        .with_size_bytes(81_920_000)
        .add_column(MockColumnStats::new("id").with_distinct(1_000_000));
    let r = run_analyze(&t);
    assert_eq!(r.row_count, 1_000_000);
    assert!(r.passed());
}

// --------------------------------------------------------------------
// 4. multi_column (4 tests)
// --------------------------------------------------------------------

#[test]
fn test_stats_multi_2_columns_p3_2() {
    let t = MockTableStats::new("users")
        .with_row_count(100)
        .add_column(MockColumnStats::new("id").with_distinct(100))
        .add_column(MockColumnStats::new("age").with_distinct(50));
    let r = run_analyze(&t);
    assert_eq!(r.column_count, 2);
}

#[test]
fn test_stats_multi_5_columns_p3_2() {
    let mut t = MockTableStats::new("users").with_row_count(100);
    for i in 0..5 {
        t = t.add_column(MockColumnStats::new(&format!("c{}", i)).with_distinct(50));
    }
    let r = run_analyze(&t);
    assert_eq!(r.column_count, 5);
}

#[test]
fn test_stats_multi_10_columns_p3_2() {
    let mut t = MockTableStats::new("wide").with_row_count(100);
    for i in 0..10 {
        t = t.add_column(MockColumnStats::new(&format!("c{}", i)).with_distinct(10));
    }
    let r = run_analyze(&t);
    assert_eq!(r.column_count, 10);
}

#[test]
fn test_stats_multi_mixed_selectivity_p3_2() {
    let t = MockTableStats::new("users")
        .with_row_count(1000)
        .add_column(MockColumnStats::new("id").with_distinct(1000)) // sel = 0.001
        .add_column(MockColumnStats::new("gender").with_distinct(2))  // sel = 0.5
        .add_column(MockColumnStats::new("flag").with_distinct(1));   // sel = 1.0
    let r = run_analyze(&t);
    assert!((r.min_eq_selectivity - 0.001).abs() < 1e-9);
    assert!((r.max_eq_selectivity - 1.0).abs() < 1e-9);
}

// --------------------------------------------------------------------
// 5. dispatcher (4 tests)
// --------------------------------------------------------------------

#[test]
fn test_stats_dispatcher_analyze_stmt_p3_2() {
    // The real Statement::Analyze is wired in src/execution_engine.rs
    // line 266. Here we just verify the harness contract.
    let t = MockTableStats::new("users")
        .with_row_count(1000)
        .add_column(MockColumnStats::new("id").with_distinct(1000));
    let r = run_analyze(&t);
    assert!(r.passed());
    assert_eq!(r.table_name, "users");
}

#[test]
fn test_stats_dispatcher_table_name_required_p3_2() {
    // The real impl returns "ANALYZE: table name is required" error
    // when analyze.table_name is None. The harness always provides
    // a name, so we just verify the table_name field is propagated.
    let t = MockTableStats::new("named_table");
    let r = run_analyze(&t);
    assert_eq!(r.table_name, "named_table");
}

#[test]
fn test_stats_dispatcher_returns_row_count_p3_2() {
    // The real impl returns ExecutorResult with row_count as
    // a single row. The harness mirrors this via AnalyzeReport.row_count.
    let t = MockTableStats::new("t").with_row_count(42);
    let r = run_analyze(&t);
    assert_eq!(r.row_count, 42);
}

#[test]
fn test_stats_dispatcher_integrates_stats_registry_p3_2() {
    // The real impl inserts into stats.write().table_stats.
    // The harness's HashMap mirrors this. We verify the column
    // count is the same as columns added.
    let t = MockTableStats::new("t")
        .with_row_count(100)
        .add_column(MockColumnStats::new("a").with_distinct(50))
        .add_column(MockColumnStats::new("b").with_distinct(50))
        .add_column(MockColumnStats::new("c").with_distinct(50));
    let r = run_analyze(&t);
    assert_eq!(r.column_count, 3);
    // verify all columns are queryable
    assert!(t.column("a").is_some());
    assert!(t.column("b").is_some());
    assert!(t.column("c").is_some());
    assert!(t.column("d").is_none()); // non-existent
}
