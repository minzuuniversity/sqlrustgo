//! CLI commands for perf-baseline

use std::path::PathBuf;
use crate::db::BaselineDb;
use crate::models::BenchmarkResult;
use chrono::Utc;

pub struct SaveCmd {
    pub commit: String,
    pub db_path: PathBuf,
}

impl SaveCmd {
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let db = BaselineDb::new(&self.db_path)?;
        
        // Run benchmarks and collect results
        let result = self.run_benchmarks()?;
        
        db.save_benchmark(&result)?;
        
        println!("Saved benchmark for commit {}", &result.commit[..8]);
        println!("  qps_update:     {:.0}", result.qps_update);
        println!("  qps_delete:     {:.0}", result.qps_delete);
        println!("  qps_insert:     {:.0}", result.qps_insert);
        println!("  qps_select:     {:.0}", result.qps_select);
        println!("  qps_complex:    {:.0}", result.qps_complex_where);
        println!("  qps_join:       {:.0}", result.qps_join);
        println!("  latency_p50:    {:.2}ms", result.latency_p50);
        println!("  latency_p99:    {:.2}ms", result.latency_p99);
        println!("  memory:         {:.0}MB", result.memory_mb);
        
        Ok(())
    }

    fn run_benchmarks(&self) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        // Get system info
        let system_info = get_system_info();
        
        // Run microbenchmarks via cargo bench
        let qps_update = run_benchmark("update_simple")?;
        let qps_delete = run_benchmark("delete_simple")?;
        let qps_insert = run_benchmark("insert_simple")?;
        let qps_select = run_benchmark("point_select")?;
        let qps_complex_where = run_benchmark("complex_where")?;
        let qps_join = run_benchmark("hash_join")?;
        
        // Run latency benchmarks
        let (latency_p50, latency_p99) = run_latency_benchmark()?;
        
        // Get memory usage
        let memory_mb = get_memory_usage()?;
        
        Ok(BenchmarkResult {
            commit: self.commit.clone(),
            timestamp: Utc::now(),
            system_info,
            qps_update,
            qps_delete,
            qps_insert,
            qps_select,
            qps_complex_where,
            qps_join,
            latency_p50,
            latency_p99,
            memory_mb,
        })
    }
}

pub struct QueryCmd {
    pub metric: String,
    pub range_days: u32,
    pub db_path: PathBuf,
}

impl QueryCmd {
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let db = BaselineDb::new(&self.db_path)?;
        
        let records = db.query_metrics(&self.metric, self.range_days)?;
        
        println!("Metric: {} (last {} days)", self.metric, self.range_days);
        println!("{:<12} {:>12} {:>12}", "timestamp", "value", "commit");
        println!("{}", "-".repeat(40));
        
        for r in records {
            println!("{:<12} {:>12.2} {:>12}", 
                r.timestamp.format("%Y-%m-%d"),
                r.metric_value,
                &r.commit[..8]);
        }
        
        Ok(())
    }
}

pub struct CheckCmd {
    pub baseline: String,
    pub head: String,
    pub threshold: f64,
    pub db_path: PathBuf,
}

impl CheckCmd {
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let db = BaselineDb::new(&self.db_path)?;
        
        // Resolve commits
        let baseline_commit = resolve_git_commit(&self.baseline)?;
        let head_commit = resolve_git_commit(&self.head)?;
        
        let comparisons = db.compare(&baseline_commit, &head_commit)?;
        
        let mut has_regression = false;
        
        println!("Performance Regression Check");
        println!("Baseline: {}  Head: {}  Threshold: {:.1}%", 
            &baseline_commit[..8], &head_commit[..8], self.threshold * 100.0);
        println!();
        println!("{:<20} {:>12} {:>12} {:>10} {:>10}", 
            "metric", "baseline", "current", "change", "status");
        println!("{}", "-".repeat(65));
        
        for c in comparisons {
            let status = if c.regressed { "❌ FAIL" } else { "✅ PASS" };
            let change_str = format!("{:+.1}%", c.change_pct);
            
            println!("{:<20} {:>12.0} {:>12.0} {:>10} {:>10}",
                c.metric, c.baseline_value, c.head_value, change_str, status);
            
            if c.regressed {
                has_regression = true;
            }
        }
        
        println!();
        if has_regression {
            println!("❌ PERFORMANCE REGRESSION DETECTED");
            println!("PR blocked: performance regressed beyond {:.1}% threshold", self.threshold * 100.0);
            std::process::exit(1);
        } else {
            println!("✅ NO SIGNIFICANT REGRESSION");
            std::process::exit(0);
        }
    }
}

fn run_benchmark(name: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("cargo")
        .args(["bench", "--", name, "--noplot"])
        .output()?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Parse benchmark output - looking for "X bytes/second"
    // or QPS = bytes/ns conversion
    for line in stderr.lines() {
        if line.contains(name) && line.contains("ns/iter") {
            // Extract ns/iter value
            if let Some(ns_str) = line.split("ns/iter").next() {
                if let Some(ns) = ns_str.split_whitespace().last() {
                    if let Ok(ns_val) = ns.parse::<f64>() {
                        // Convert to QPS (assuming 1 iteration = 1 operation)
                        let qps = 1_000_000_000.0 / ns_val;
                        return Ok(qps);
                    }
                }
            }
        }
    }
    
    // Fallback: return a placeholder
    Ok(0.0)
}

fn run_latency_benchmark() -> Result<(f64, f64), Box<dyn std::error::Error>> {
    // For now, return placeholder values
    // In a real implementation, this would run a latency benchmark
    Ok((1.0, 5.0))
}

fn get_system_info() -> String {
    format!("{} {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::process::Command::new("nproc")
            .output()
            .map(|o| format!("{} cores", String::from_utf8_lossy(&o.stdout).trim()))
            .unwrap_or_else(|_| "unknown".to_string()))
}

fn get_memory_usage() -> Result<f64, Box<dyn std::error::Error>> {
    // Get RSS memory in MB
    let output = std::process::Command::new("ps")
        .args(["-p", &std::process::id().to_string(), "-o", "rss="])
        .output()?;
    
    if let Ok(rss) = String::from_utf8_lossy(&output.stdout).trim().parse::<f64>() {
        Ok(rss / 1024.0) // Convert KB to MB
    } else {
        Ok(0.0)
    }
}

fn resolve_git_commit(ref_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", ref_str])
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        // Try as direct commit
        if ref_str.len() == 40 {
            Ok(ref_str.to_string())
        } else {
            Err(format!("Cannot resolve git ref: {}", ref_str).into())
        }
    }
}
