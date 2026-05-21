//! Storage module for perf-baseline (JSON file-based)

use crate::models::{BenchmarkResult, MetricRecord};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

pub struct BaselineDb {
    base_path: PathBuf,
}

impl BaselineDb {
    pub fn new(base_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Self {
            base_path: base_path.to_path_buf(),
        };
        db.ensure_dir()?;
        Ok(db)
    }

    fn ensure_dir(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.base_path)?;
        Ok(())
    }

    fn benchmarks_file(&self) -> PathBuf {
        self.base_path.join("benchmarks.json")
    }

    fn metrics_file(&self, metric: &str) -> PathBuf {
        self.base_path.join(format!("metrics_{}.json", metric))
    }

    fn load_benchmarks(&self) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        let file = self.benchmarks_file();
        if !file.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&file)?;
        let results: Vec<BenchmarkResult> = serde_json::from_str(&content)?;
        Ok(results)
    }

    fn save_benchmarks(
        &self,
        results: &[BenchmarkResult],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(results)?;
        fs::write(self.benchmarks_file(), content)?;
        Ok(())
    }

    fn load_metrics(&self, metric: &str) -> Result<Vec<MetricRecord>, Box<dyn std::error::Error>> {
        let file = self.metrics_file(metric);
        if !file.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&file)?;
        let records: Vec<MetricRecord> = serde_json::from_str(&content)?;
        Ok(records)
    }

    fn save_metrics(
        &self,
        metric: &str,
        records: &[MetricRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(records)?;
        fs::write(self.metrics_file(metric), content)?;
        Ok(())
    }

    pub fn save_benchmark(
        &self,
        result: &BenchmarkResult,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut all = self.load_benchmarks()?;

        // Update or append
        if let Some(pos) = all.iter().position(|r| r.commit == result.commit) {
            all[pos] = result.clone();
        } else {
            all.push(result.clone());
        }

        // Sort by timestamp
        all.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        self.save_benchmarks(&all)?;

        // Also save individual metrics
        self.save_metric(
            "qps_update",
            result.qps_update,
            &result.commit,
            result.timestamp,
        )?;
        self.save_metric(
            "qps_delete",
            result.qps_delete,
            &result.commit,
            result.timestamp,
        )?;
        self.save_metric(
            "qps_insert",
            result.qps_insert,
            &result.commit,
            result.timestamp,
        )?;
        self.save_metric(
            "qps_select",
            result.qps_select,
            &result.commit,
            result.timestamp,
        )?;
        self.save_metric(
            "qps_complex_where",
            result.qps_complex_where,
            &result.commit,
            result.timestamp,
        )?;
        self.save_metric(
            "qps_join",
            result.qps_join,
            &result.commit,
            result.timestamp,
        )?;
        self.save_metric(
            "latency_p50",
            result.latency_p50,
            &result.commit,
            result.timestamp,
        )?;
        self.save_metric(
            "latency_p99",
            result.latency_p99,
            &result.commit,
            result.timestamp,
        )?;
        self.save_metric(
            "memory_mb",
            result.memory_mb,
            &result.commit,
            result.timestamp,
        )?;

        Ok(())
    }

    fn save_metric(
        &self,
        name: &str,
        value: f64,
        commit: &str,
        ts: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut records = self.load_metrics(name)?;

        records.push(MetricRecord {
            metric_name: name.to_string(),
            metric_value: value,
            commit: commit.to_string(),
            timestamp: ts,
        });

        // Sort by timestamp
        records.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        self.save_metrics(name, &records)?;
        Ok(())
    }

    pub fn get_commit(
        &self,
        commit: &str,
    ) -> Result<Option<BenchmarkResult>, Box<dyn std::error::Error>> {
        let all = self.load_benchmarks()?;
        Ok(all.into_iter().find(|r| r.commit == commit))
    }

    pub fn query_metrics(
        &self,
        metric: &str,
        days: u32,
    ) -> Result<Vec<MetricRecord>, Box<dyn std::error::Error>> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let records = self.load_metrics(metric)?;
        Ok(records
            .into_iter()
            .filter(|r| r.timestamp >= cutoff)
            .collect())
    }

    pub fn get_latest(&self) -> Result<Option<BenchmarkResult>, Box<dyn std::error::Error>> {
        let all = self.load_benchmarks()?;
        Ok(all.into_iter().max_by_key(|r| r.timestamp))
    }

    pub fn compare(
        &self,
        baseline_commit: &str,
        head_commit: &str,
    ) -> Result<Vec<MetricComparison>, Box<dyn std::error::Error>> {
        let baseline = self.get_commit(baseline_commit)?;
        let head = self.get_commit(head_commit)?;

        let mut comparisons = Vec::new();

        if let (Some(b), Some(h)) = (baseline, head) {
            comparisons.push(MetricComparison {
                metric: "qps_update".to_string(),
                baseline_value: b.qps_update,
                head_value: h.qps_update,
                change_pct: pct_change(b.qps_update, h.qps_update),
                regressed: pct_change(b.qps_update, h.qps_update) < -10.0,
            });
            comparisons.push(MetricComparison {
                metric: "qps_delete".to_string(),
                baseline_value: b.qps_delete,
                head_value: h.qps_delete,
                change_pct: pct_change(b.qps_delete, h.qps_delete),
                regressed: pct_change(b.qps_delete, h.qps_delete) < -10.0,
            });
            comparisons.push(MetricComparison {
                metric: "qps_insert".to_string(),
                baseline_value: b.qps_insert,
                head_value: h.qps_insert,
                change_pct: pct_change(b.qps_insert, h.qps_insert),
                regressed: pct_change(b.qps_insert, h.qps_insert) < -10.0,
            });
            comparisons.push(MetricComparison {
                metric: "qps_select".to_string(),
                baseline_value: b.qps_select,
                head_value: h.qps_select,
                change_pct: pct_change(b.qps_select, h.qps_select),
                regressed: pct_change(b.qps_select, h.qps_select) < -10.0,
            });
            comparisons.push(MetricComparison {
                metric: "qps_complex_where".to_string(),
                baseline_value: b.qps_complex_where,
                head_value: h.qps_complex_where,
                change_pct: pct_change(b.qps_complex_where, h.qps_complex_where),
                regressed: pct_change(b.qps_complex_where, h.qps_complex_where) < -10.0,
            });
            comparisons.push(MetricComparison {
                metric: "qps_join".to_string(),
                baseline_value: b.qps_join,
                head_value: h.qps_join,
                change_pct: pct_change(b.qps_join, h.qps_join),
                regressed: pct_change(b.qps_join, h.qps_join) < -10.0,
            });
        }

        Ok(comparisons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_db() -> (BaselineDb, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().to_path_buf();
        let db = BaselineDb::new(&db_path).unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_baseline_db_new_creates_dir() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().to_path_buf();
        let db = BaselineDb::new(&db_path).unwrap();
        assert!(db_path.exists());
        drop(db);
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_save_and_get_benchmark() {
        use chrono::Utc;
        let (db, _temp) = create_test_db();

        let result = BenchmarkResult {
            commit: "abc123".to_string(),
            timestamp: Utc::now(),
            system_info: "test".to_string(),
            qps_update: 10000.0,
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 256.0,
        };

        db.save_benchmark(&result).unwrap();

        let retrieved = db.get_commit("abc123").unwrap().unwrap();
        assert_eq!(retrieved.commit, "abc123");
        assert_eq!(retrieved.qps_update, 10000.0);
        assert_eq!(retrieved.qps_select, 30000.0);
    }

    #[test]
    fn test_get_commit_not_found() {
        let (db, _temp) = create_test_db();
        let result = db.get_commit("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_save_benchmark_updates_existing() {
        use chrono::Utc;
        let (db, _temp) = create_test_db();

        let result1 = BenchmarkResult {
            commit: "abc123".to_string(),
            timestamp: Utc::now(),
            system_info: "test".to_string(),
            qps_update: 10000.0,
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 256.0,
        };

        db.save_benchmark(&result1).unwrap();

        let mut result2 = result1.clone();
        result2.qps_update = 15000.0;

        db.save_benchmark(&result2).unwrap();

        let retrieved = db.get_commit("abc123").unwrap().unwrap();
        assert_eq!(retrieved.qps_update, 15000.0); // Updated value
    }

    #[test]
    fn test_query_metrics() {
        use chrono::{Duration, Utc};
        let (db, _temp) = create_test_db();

        let now = Utc::now();
        let recent = now - Duration::days(1);
        let old = now - Duration::days(10);

        // Save metrics at different times
        let _record1 = MetricRecord {
            metric_name: "qps_update".to_string(),
            metric_value: 10000.0,
            commit: "abc".to_string(),
            timestamp: recent,
        };
        let _record2 = MetricRecord {
            metric_name: "qps_update".to_string(),
            metric_value: 8000.0,
            commit: "def".to_string(),
            timestamp: old,
        };

        // Directly save via benchmark
        let result = BenchmarkResult {
            commit: "abc".to_string(),
            timestamp: recent,
            system_info: "test".to_string(),
            qps_update: 10000.0,
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 256.0,
        };
        db.save_benchmark(&result).unwrap();

        let records = db.query_metrics("qps_update", 7).unwrap();
        assert!(!records.is_empty());
    }

    #[test]
    fn test_get_latest() {
        use chrono::{Duration, Utc};
        let (db, _temp) = create_test_db();

        let now = Utc::now();
        let earlier = now - Duration::hours(1);

        let result1 = BenchmarkResult {
            commit: "abc".to_string(),
            timestamp: earlier,
            system_info: "test".to_string(),
            qps_update: 10000.0,
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 256.0,
        };

        let result2 = BenchmarkResult {
            commit: "def".to_string(),
            timestamp: now,
            system_info: "test".to_string(),
            qps_update: 12000.0,
            qps_delete: 5500.0,
            qps_insert: 8500.0,
            qps_select: 32000.0,
            qps_complex_where: 4200.0,
            qps_join: 3100.0,
            latency_p50: 0.4,
            latency_p99: 1.4,
            memory_mb: 260.0,
        };

        db.save_benchmark(&result1).unwrap();
        db.save_benchmark(&result2).unwrap();

        let latest = db.get_latest().unwrap().unwrap();
        assert_eq!(latest.commit, "def");
    }

    #[test]
    fn test_compare() {
        use chrono::Utc;
        let (db, _temp) = create_test_db();

        let result1 = BenchmarkResult {
            commit: "abc".to_string(),
            timestamp: Utc::now(),
            system_info: "test".to_string(),
            qps_update: 10000.0,
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 256.0,
        };

        let result2 = BenchmarkResult {
            commit: "def".to_string(),
            timestamp: Utc::now(),
            system_info: "test".to_string(),
            qps_update: 8999.0, // ~10.01% decrease (triggers regressed: < -10.0)
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 256.0,
        };

        db.save_benchmark(&result1).unwrap();
        db.save_benchmark(&result2).unwrap();

        let comparisons = db.compare("abc", "def").unwrap();
        assert!(!comparisons.is_empty());

        let qps_update_comp = comparisons
            .iter()
            .find(|c| c.metric == "qps_update")
            .unwrap();
        assert!((qps_update_comp.change_pct - (-10.0)).abs() < 0.1);
        assert!(qps_update_comp.regressed);
    }

    #[test]
    fn test_compare_no_regression() {
        use chrono::Utc;
        let (db, _temp) = create_test_db();

        let result1 = BenchmarkResult {
            commit: "abc".to_string(),
            timestamp: Utc::now(),
            system_info: "test".to_string(),
            qps_update: 10000.0,
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 256.0,
        };

        let result2 = BenchmarkResult {
            commit: "def".to_string(),
            timestamp: Utc::now(),
            system_info: "test".to_string(),
            qps_update: 10500.0, // 5% improvement
            qps_delete: 5000.0,
            qps_insert: 8000.0,
            qps_select: 30000.0,
            qps_complex_where: 4000.0,
            qps_join: 3000.0,
            latency_p50: 0.5,
            latency_p99: 1.5,
            memory_mb: 256.0,
        };

        db.save_benchmark(&result1).unwrap();
        db.save_benchmark(&result2).unwrap();

        let comparisons = db.compare("abc", "def").unwrap();
        let qps_update_comp = comparisons
            .iter()
            .find(|c| c.metric == "qps_update")
            .unwrap();
        assert!((qps_update_comp.change_pct - 5.0).abs() < 0.1);
        assert!(!qps_update_comp.regressed);
    }
}

fn pct_change(baseline: f64, head: f64) -> f64 {
    if baseline == 0.0 {
        return 0.0;
    }
    ((head - baseline) / baseline) * 100.0
}

#[derive(Debug, Clone)]
pub struct MetricComparison {
    pub metric: String,
    pub baseline_value: f64,
    pub head_value: f64,
    pub change_pct: f64,
    pub regressed: bool,
}
