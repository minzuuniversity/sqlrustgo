//! Operator Profile Module
//!
//! Provides detailed profiling data for query execution.
//! This is a teaching-enhanced feature that helps understand query performance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;
use uuid::Uuid;

/// Global profiler for all query profiles
pub static GLOBAL_PROFILER: QueryProfiler = QueryProfiler::new();

/// Query profiler - stores all query profiles
pub struct QueryProfiler {
    profiles: RwLock<Vec<QueryProfile>>,
}

impl QueryProfiler {
    pub const fn new() -> Self {
        Self {
            profiles: RwLock::new(Vec::new()),
        }
    }

    /// Record a query profile
    pub fn record_query(&self, profile: QueryProfile) {
        if let Ok(mut profiles) = self.profiles.write() {
            // Keep only last 1000 profiles
            if profiles.len() >= 1000 {
                profiles.remove(0);
            }
            profiles.push(profile);
        }
    }

    /// Get all profiles
    pub fn get_all(&self) -> Vec<QueryProfile> {
        self.profiles.read().unwrap().clone()
    }

    /// Get profile by query ID
    pub fn get_by_id(&self, query_id: &str) -> Option<QueryProfile> {
        self.profiles.read().unwrap().iter()
            .find(|p| p.query_id == query_id)
            .cloned()
    }

    /// Clear all profiles
    pub fn clear(&self) {
        self.profiles.write().unwrap().clear();
    }

    /// Get top N slowest queries
    pub fn get_slowest(&self, n: usize) -> Vec<QueryProfile> {
        let mut profiles = self.profiles.read().unwrap().clone();
        profiles.sort_by(|a, b| b.duration_ns.cmp(&a.duration_ns));
        profiles.into_iter().take(n).collect()
    }

    /// Get summary statistics
    pub fn summary(&self) -> ProfilerSummary {
        let profiles = self.profiles.read().unwrap();
        
        if profiles.is_empty() {
            return ProfilerSummary {
                total_queries: 0,
                successful_queries: 0,
                failed_queries: 0,
                avg_duration_ns: 0,
                min_duration_ns: 0,
                max_duration_ns: 0,
                total_cpu_time_ns: 0,
                total_rows: 0,
            };
        }
        
        let total = profiles.len();
        let successful = profiles.iter().filter(|p| p.success).count();
        let failed = total - successful;
        
        let durations: Vec<u64> = profiles.iter().map(|p| p.duration_ns).collect();
        let total_duration: u64 = durations.iter().sum();
        let avg_duration = total_duration / total as u64;
        
        let min_duration = *durations.iter().min().unwrap_or(&0);
        let max_duration = *durations.iter().max().unwrap_or(&0);
        
        let total_cpu: u64 = profiles.iter()
            .map(|p| p.operators.iter().map(|o| o.cpu_time_ns).sum::<u64>())
            .sum();
        
        let total_rows: usize = profiles.iter()
            .map(|p| p.operators.iter().map(|o| o.rows_processed).sum::<usize>())
            .sum();
        
        ProfilerSummary {
            total_queries: total,
            successful_queries: successful,
            failed_queries: failed,
            avg_duration_ns: avg_duration,
            min_duration_ns: min_duration,
            max_duration_ns: max_duration,
            total_cpu_time_ns: total_cpu,
            total_rows,
        }
    }
}

/// Profile summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilerSummary {
    pub total_queries: usize,
    pub successful_queries: usize,
    pub failed_queries: usize,
    pub avg_duration_ns: u64,
    pub min_duration_ns: u64,
    pub max_duration_ns: u64,
    pub total_cpu_time_ns: u64,
    pub total_rows: usize,
}

/// Query-level profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProfile {
    /// Unique query identifier
    pub query_id: String,
    /// The SQL query text
    pub sql: String,
    /// Start timestamp
    pub start_time: u64,
    /// End timestamp  
    pub end_time: u64,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Whether query succeeded
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Operator profiles
    pub operators: Vec<OperatorProfile>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl QueryProfile {
    /// Create a new query profile
    pub fn new(query_id: &str, sql: &str) -> Self {
        Self {
            query_id: query_id.to_string(),
            sql: sql.to_string(),
            start_time: 0,
            end_time: 0,
            duration_ns: 0,
            success: true,
            error_message: None,
            operators: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Mark query as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.success = false;
        self.error_message = Some(error.to_string());
    }

    /// Finish the query profile
    pub fn finish(&mut self, duration_ns: u64) {
        self.duration_ns = duration_ns;
        self.end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
    }

    /// Add operator profile
    pub fn add_operator(&mut self, operator: OperatorProfile) {
        self.operators.push(operator);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Get total rows processed
    pub fn total_rows(&self) -> usize {
        self.operators.iter().map(|o| o.rows_processed).sum()
    }

    /// Get total batches processed
    pub fn total_batches(&self) -> usize {
        self.operators.iter().map(|o| o.batches_processed).sum()
    }

    /// Get execution efficiency (rows per nanosecond)
    pub fn efficiency(&self) -> f64 {
        if self.duration_ns == 0 {
            return 0.0;
        }
        self.total_rows() as f64 / self.duration_ns as f64
    }
}

/// Operator-level profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorProfile {
    /// Unique operator identifier
    pub operator_id: String,
    /// Operator name
    pub operator_name: String,
    /// Physical operator type
    pub operator_type: String,
    /// CPU time in nanoseconds
    pub cpu_time_ns: u64,
    /// Wall time in nanoseconds
    pub wall_time_ns: u64,
    /// Number of rows processed
    pub rows_processed: usize,
    /// Number of batches processed
    pub batches_processed: usize,
    /// Peak memory usage in bytes
    #[serde(default)]
    pub peak_memory_bytes: u64,
    /// Number of function calls
    #[serde(default)]
    pub function_calls: usize,
    /// Additional metrics
    #[serde(default)]
    pub metrics: HashMap<String, String>,
}

impl OperatorProfile {
    /// Create a new operator profile
    pub fn new(operator_name: &str, operator_type: &str) -> Self {
        Self {
            operator_id: Uuid::new_v4().to_string(),
            operator_name: operator_name.to_string(),
            operator_type: operator_type.to_string(),
            cpu_time_ns: 0,
            wall_time_ns: 0,
            rows_processed: 0,
            batches_processed: 0,
            peak_memory_bytes: 0,
            function_calls: 0,
            metrics: HashMap::new(),
        }
    }

    /// Record an execution
    pub fn record_execution(&mut self, wall_time_ns: u64, rows: usize, batches: usize) {
        self.wall_time_ns += wall_time_ns;
        self.cpu_time_ns += wall_time_ns; // Approximate CPU = wall for single-threaded
        self.rows_processed += rows;
        self.batches_processed += batches;
        self.function_calls += 1;
    }

    /// Add a metric
    pub fn add_metric(&mut self, key: &str, value: &str) {
        self.metrics.insert(key.to_string(), value.to_string());
    }

    /// Set peak memory
    pub fn set_peak_memory(&mut self, bytes: u64) {
        if bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = bytes;
        }
    }

    /// Get throughput (rows per second)
    pub fn throughput(&self) -> f64 {
        if self.wall_time_ns == 0 {
            return 0.0;
        }
        self.rows_processed as f64 / (self.wall_time_ns as f64 / 1_000_000_000.0)
    }

    /// Get batches per second
    pub fn batch_throughput(&self) -> f64 {
        if self.wall_time_ns == 0 {
            return 0.0;
        }
        self.batches_processed as f64 / (self.wall_time_ns as f64 / 1_000_000_000.0)
    }
}

/// Profile timer for measuring execution time
pub struct ProfileTimer {
    name: String,
    operator_type: String,
    start: Instant,
    rows: usize,
    batches: usize,
}

impl ProfileTimer {
    /// Create a new profile timer
    pub fn new(name: &str, operator_type: &str) -> Self {
        Self {
            name: name.to_string(),
            operator_type: operator_type.to_string(),
            start: Instant::now(),
            rows: 0,
            batches: 0,
        }
    }

    /// Record rows processed
    pub fn record_rows(&mut self, count: usize) {
        self.rows += count;
        self.batches += 1;
    }

    /// Finish timing and return operator profile
    pub fn finish(self) -> OperatorProfile {
        let duration = self.start.elapsed().as_nanos() as u64;
        
        let mut profile = OperatorProfile::new(&self.name, &self.operator_type);
        profile.record_execution(duration, self.rows, self.batches);
        
        profile
    }
}

/// Vectorized execution trace - detailed trace for vectorized operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizedTrace {
    /// Trace ID
    pub trace_id: String,
    /// Query ID
    pub query_id: String,
    /// Vector size
    pub vector_size: usize,
    /// Number of vectors processed
    pub vectors_processed: usize,
    /// Total rows in all vectors
    pub total_rows: usize,
    /// Time spent in vector operations
    pub vector_time_ns: u64,
    /// Time spent in scalar operations
    pub scalar_time_ns: u64,
    /// Detailed operation traces
    #[serde(default)]
    pub operations: Vec<OperationTrace>,
}

impl VectorizedTrace {
    /// Create a new vectorized trace
    pub fn new(query_id: &str, vector_size: usize) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            query_id: query_id.to_string(),
            vector_size,
            vectors_processed: 0,
            total_rows: 0,
            vector_time_ns: 0,
            scalar_time_ns: 0,
            operations: Vec::new(),
        }
    }

    /// Record a vector operation
    pub fn record_vector(&mut self, rows: usize, time_ns: u64) {
        self.vectors_processed += 1;
        self.total_rows += rows;
        self.vector_time_ns += time_ns;
    }

    /// Record a scalar operation
    pub fn record_scalar(&mut self, time_ns: u64) {
        self.scalar_time_ns += time_ns;
    }

    /// Add operation detail
    pub fn add_operation(&mut self, op: OperationTrace) {
        self.operations.push(op);
    }

    /// Get vectorization ratio
    pub fn vectorization_ratio(&self) -> f64 {
        let total = self.vector_time_ns + self.scalar_time_ns;
        if total == 0 {
            return 0.0;
        }
        self.vector_time_ns as f64 / total as f64
    }
}

/// Individual operation trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationTrace {
    /// Operation name
    pub name: String,
    /// Operation type (vector/scalar)
    pub op_type: String,
    /// Time in nanoseconds
    pub time_ns: u64,
    /// Rows affected
    pub rows: usize,
    /// Additional details
    #[serde(default)]
    pub details: HashMap<String, String>,
}

impl OperationTrace {
    /// Create a new operation trace
    pub fn new(name: &str, op_type: &str, time_ns: u64, rows: usize) -> Self {
        Self {
            name: name.to_string(),
            op_type: op_type.to_string(),
            time_ns,
            rows,
            details: HashMap::new(),
        }
    }

    /// Add detail
    pub fn add_detail(&mut self, key: &str, value: &str) {
        self.details.insert(key.to_string(), value.to_string());
    }
}

/// Convert QueryProfile to JSON
pub fn profile_to_json(profile: &QueryProfile) -> String {
    serde_json::to_string_pretty(profile).unwrap_or_default()
}

/// Get detailed profiling report
pub fn get_profiling_report(profiler: &QueryProfiler) -> String {
    let summary = profiler.summary();
    let slowest = profiler.get_slowest(10);
    
    let report = serde_json::json!({
        "summary": summary,
        "slowest_queries": slowest,
    });
    
    serde_json::to_string_pretty(&report).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_profile_creation() {
        let profile = QueryProfile::new("test123", "SELECT * FROM users");
        assert_eq!(profile.query_id, "test123");
        assert_eq!(profile.sql, "SELECT * FROM users");
        assert!(profile.success);
    }

    #[test]
    fn test_query_profile_failed() {
        let mut profile = QueryProfile::new("test123", "SELECT * FROM users");
        profile.mark_failed("Table not found");
        
        assert!(!profile.success);
        assert!(profile.error_message.is_some());
    }

    #[test]
    fn test_query_profile_finish() {
        let mut profile = QueryProfile::new("test123", "SELECT 1");
        profile.finish(1000000);
        
        assert_eq!(profile.duration_ns, 1000000);
    }

    #[test]
    fn test_operator_profile() {
        let mut op = OperatorProfile::new("SeqScan", "seq_scan");
        op.record_execution(1000, 100, 5);
        
        assert_eq!(op.wall_time_ns, 1000);
        assert_eq!(op.rows_processed, 100);
        assert_eq!(op.batches_processed, 5);
    }

    #[test]
    fn test_operator_profile_throughput() {
        let mut op = OperatorProfile::new("SeqScan", "seq_scan");
        op.record_execution(1_000_000_000, 1000, 10); // 1 second, 1000 rows
        
        let throughput = op.throughput();
        assert!((throughput - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_profile_timer() {
        let mut timer = ProfileTimer::new("TestOp", "test");
        timer.record_rows(100);
        
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        let profile = timer.finish();
        
        assert_eq!(profile.operator_name, "TestOp");
        assert_eq!(profile.rows_processed, 100);
        assert!(profile.wall_time_ns > 0);
    }

    #[test]
    fn test_vectorized_trace() {
        let mut trace = VectorizedTrace::new("query123", 1024);
        
        trace.record_vector(1024, 5000);
        trace.record_vector(1024, 6000);
        trace.record_scalar(1000);
        
        assert_eq!(trace.vectors_processed, 2);
        assert_eq!(trace.total_rows, 2048);
        assert_eq!(trace.vector_time_ns, 11000);
        assert_eq!(trace.scalar_time_ns, 1000);
        
        let ratio = trace.vectorization_ratio();
        assert!((ratio - 0.9166).abs() < 0.01); // ~91.66%
    }

    #[test]
    fn test_operation_trace() {
        let mut op = OperationTrace::new("Filter", "vector", 1000, 500);
        op.add_detail("predicate", "age > 18");
        
        assert_eq!(op.name, "Filter");
        assert_eq!(op.details.get("predicate"), Some(&"age > 18".to_string()));
    }

    #[test]
    fn test_global_profiler() {
        GLOBAL_PROFILER.clear();
        
        let profile = QueryProfile::new("test", "SELECT 1");
        GLOBAL_PROFILER.record_query(profile);
        
        let all = GLOBAL_PROFILER.get_all();
        // Note: May have more than 1 due to other tests running in parallel
        assert!(all.len() >= 1);
        
        GLOBAL_PROFILER.clear();
    }

    #[test]
    fn test_profiler_summary() {
        GLOBAL_PROFILER.clear();
        
        // Add 5 test profiles
        for i in 0..5 {
            let mut profile = QueryProfile::new(&format!("test{}", i), &format!("SELECT {}", i));
            profile.finish(1000 * (i + 1) as u64);
            
            let mut op = OperatorProfile::new("Test", "test");
            op.record_execution(500, 100, 1);
            profile.add_operator(op);
            
            GLOBAL_PROFILER.record_query(profile);
        }
        
        let summary = GLOBAL_PROFILER.summary();
        // Should have at least 5 queries (may have more from parallel tests)
        assert!(summary.total_queries >= 5);
        
        GLOBAL_PROFILER.clear();
    }

    #[test]
    fn test_get_slowest_queries() {
        GLOBAL_PROFILER.clear();
        
        for i in 0..10 {
            let mut profile = QueryProfile::new(&format!("test{}", i), &format!("SELECT {}", i));
            profile.finish((10 - i) as u64 * 1000); // Descending order
            GLOBAL_PROFILER.record_query(profile);
        }
        
        let slowest = GLOBAL_PROFILER.get_slowest(3);
        
        // Should be 9, 8, 7 (largest durations first)
        assert!(slowest[0].duration_ns >= slowest[1].duration_ns);
        assert!(slowest[1].duration_ns >= slowest[2].duration_ns);
        
        GLOBAL_PROFILER.clear();
    }

    #[test]
    fn test_profile_to_json() {
        let mut profile = QueryProfile::new("test", "SELECT 1");
        profile.finish(1000);
        
        let json = profile_to_json(&profile);
        
        assert!(json.contains("SELECT 1"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_query_profile_metadata() {
        let mut profile = QueryProfile::new("test", "SELECT 1");
        profile.add_metadata("parser", "native");
        profile.add_metadata("optimizer", "cost-based");
        
        assert_eq!(profile.metadata.get("parser"), Some(&"native".to_string()));
    }

    #[test]
    fn test_operator_profile_metrics() {
        let mut op = OperatorProfile::new("HashJoin", "hash_join");
        op.add_metric("hash_buckets", "1024");
        op.add_metric("collisions", "5");
        
        assert_eq!(op.metrics.get("hash_buckets"), Some(&"1024".to_string()));
    }

    #[test]
    fn test_query_profile_totals() {
        let mut profile = QueryProfile::new("test", "SELECT 1");
        
        let mut op1 = OperatorProfile::new("Scan1", "seq_scan");
        op1.record_execution(1000, 100, 5);
        
        let mut op2 = OperatorProfile::new("Filter", "filter");
        op2.record_execution(500, 80, 5);
        
        profile.add_operator(op1);
        profile.add_operator(op2);
        
        assert_eq!(profile.total_rows(), 180);
        assert_eq!(profile.total_batches(), 10);
    }

    #[test]
    fn test_operator_peak_memory() {
        let mut op = OperatorProfile::new("Test", "test");
        
        op.set_peak_memory(1000);
        op.set_peak_memory(2000); // Higher
        op.set_peak_memory(1500); // Lower
        
        assert_eq!(op.peak_memory_bytes, 2000);
    }

    #[test]
    fn test_vectorized_trace_operations() {
        let mut trace = VectorizedTrace::new("query1", 1024);
        
        let mut op1 = OperationTrace::new("Filter", "vector", 1000, 1024);
        op1.add_detail("condition", "x > 0");
        
        trace.add_operation(op1);
        
        assert_eq!(trace.operations.len(), 1);
    }
}
