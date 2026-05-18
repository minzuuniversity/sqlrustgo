//! Storage module for perf-baseline (JSON file-based)

use crate::models::{BenchmarkResult, MetricRecord};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

pub struct BaselineDb {
    base_path: PathBuf,
}

impl BaselineDb {
    pub fn new(base_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Self {
            base_path: base_path.clone(),
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
