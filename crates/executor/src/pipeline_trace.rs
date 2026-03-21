//! Pipeline Trace Module
//!
//! Provides JSON format execution plan graph for teaching-enhanced features.
//! This enables visualization of the query execution pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;
use uuid::Uuid;

/// Global trace collector for all query traces
pub static GLOBAL_TRACE_COLLECTOR: TraceCollector = TraceCollector::new();

/// Trace collector - stores all query traces
pub struct TraceCollector {
    traces: RwLock<Vec<QueryTrace>>,
}

impl TraceCollector {
    pub const fn new() -> Self {
        Self {
            traces: RwLock::new(Vec::new()),
        }
    }

    /// Record a query trace
    pub fn record(&self, trace: QueryTrace) {
        if let Ok(mut traces) = self.traces.write() {
            // Keep only last 1000 traces
            if traces.len() >= 1000 {
                traces.remove(0);
            }
            traces.push(trace);
        }
    }

    /// Get all traces
    pub fn get_all(&self) -> Vec<QueryTrace> {
        self.traces.read().unwrap().clone()
    }

    /// Get trace by query ID
    pub fn get_by_id(&self, query_id: &str) -> Option<QueryTrace> {
        self.traces.read().unwrap().iter()
            .find(|t| t.query_id == query_id)
            .cloned()
    }

    /// Clear all traces
    pub fn clear(&self) {
        self.traces.write().unwrap().clear();
    }
}

/// Query-level trace containing all operator traces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTrace {
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
    /// Root operator trace
    pub root_operator: OperatorTrace,
    /// Total rows returned
    pub total_rows: usize,
    /// Total batches processed
    pub total_batches: usize,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl QueryTrace {
    /// Create a new query trace
    pub fn new(sql: &str) -> Self {
        Self {
            query_id: Uuid::new_v4().to_string(),
            sql: sql.to_string(),
            start_time: 0,
            end_time: 0,
            duration_ns: 0,
            success: true,
            error_message: None,
            root_operator: OperatorTrace::new("Query", "root"),
            total_rows: 0,
            total_batches: 0,
            metadata: HashMap::new(),
        }
    }

    /// Mark query as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.success = false;
        self.error_message = Some(error.to_string());
    }

    /// Finish the query trace
    pub fn finish(&mut self, duration: std::time::Duration) {
        self.duration_ns = duration.as_nanos() as u64;
        self.end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }
}

/// Individual operator trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorTrace {
    /// Trace ID for this operator
    pub trace_id: String,
    /// Parent trace ID
    #[serde(default)]
    pub parent_trace_id: Option<String>,
    /// Operator name
    pub operator_name: String,
    /// Physical operator type
    pub operator_type: String,
    /// Start time (offset from query start)
    pub start_ns: u64,
    /// End time (offset from query start)
    pub end_ns: u64,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Number of rows processed
    pub rows_processed: usize,
    /// Number of batches processed
    pub batches_processed: usize,
    /// Child operator traces
    #[serde(default)]
    pub children: Vec<OperatorTrace>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl OperatorTrace {
    /// Create a new operator trace
    pub fn new(operator_name: &str, operator_type: &str) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            parent_trace_id: None,
            operator_name: operator_name.to_string(),
            operator_type: operator_type.to_string(),
            start_ns: 0,
            end_ns: 0,
            duration_ns: 0,
            rows_processed: 0,
            batches_processed: 0,
            children: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Start tracing this operator
    pub fn start(&mut self, query_start: Instant) {
        self.start_ns = query_start.elapsed().as_nanos() as u64;
    }

    /// Finish tracing this operator
    pub fn finish(&mut self, query_start: Instant) {
        self.end_ns = query_start.elapsed().as_nanos() as u64;
        self.duration_ns = self.end_ns - self.start_ns;
    }

    /// Record a batch processed
    pub fn record_batch(&mut self) {
        self.batches_processed += 1;
    }

    /// Record rows processed
    pub fn record_rows(&mut self, count: usize) {
        self.rows_processed += count;
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Add child operator
    pub fn add_child(&mut self, child: OperatorTrace) {
        self.children.push(child);
    }
}

/// Execution pipeline visualization data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineVisualization {
    /// Query ID
    pub query_id: String,
    /// SQL query
    pub sql: String,
    /// Total duration in nanoseconds
    pub duration_ns: u64,
    /// Success status
    pub success: bool,
    /// Pipeline nodes
    pub nodes: Vec<PipelineNode>,
    /// Pipeline edges (relationships between nodes)
    pub edges: Vec<PipelineEdge>,
}

impl PipelineVisualization {
    /// Create from query trace
    pub fn from_trace(trace: &QueryTrace) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        
        // Build nodes and edges recursively
        Self::build_pipeline(&trace.root_operator, &mut nodes, &mut edges, None);
        
        Self {
            query_id: trace.query_id.clone(),
            sql: trace.sql.clone(),
            duration_ns: trace.duration_ns,
            success: trace.success,
            nodes,
            edges,
        }
    }

    /// Build pipeline nodes and edges recursively
    fn build_pipeline(
        operator: &OperatorTrace,
        nodes: &mut Vec<PipelineNode>,
        edges: &mut Vec<PipelineEdge>,
        parent_id: Option<String>,
    ) {
        let node_id = operator.trace_id.clone();
        
        // Add node
        nodes.push(PipelineNode {
            id: node_id.clone(),
            name: operator.operator_name.clone(),
            operator_type: operator.operator_type.clone(),
            duration_ns: operator.duration_ns,
            rows_processed: operator.rows_processed,
            batches_processed: operator.batches_processed,
            metadata: operator.metadata.clone(),
        });
        
        // Add edge to parent
        if let Some(parent) = parent_id {
            edges.push(PipelineEdge {
                from: parent,
                to: node_id.clone(),
            });
        }
        
        // Process children
        for child in &operator.children {
            Self::build_pipeline(child, nodes, edges, Some(node_id.clone()));
        }
    }

    /// Export as JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Pipeline node for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineNode {
    /// Unique node ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Operator type
    pub operator_type: String,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Rows processed
    pub rows_processed: usize,
    /// Batches processed
    pub batches_processed: usize,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Pipeline edge for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
}

/// Convert QueryTrace to JSON format execution plan
pub fn trace_to_json(trace: &QueryTrace) -> String {
    serde_json::to_string_pretty(trace).unwrap_or_default()
}

/// Convert QueryTrace to pipeline visualization
pub fn trace_to_pipeline(trace: &QueryTrace) -> PipelineVisualization {
    PipelineVisualization::from_trace(trace)
}

/// Get summary statistics from all traces
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceSummary {
    pub total_queries: usize,
    pub successful_queries: usize,
    pub failed_queries: usize,
    pub avg_duration_ns: u64,
    pub total_rows: usize,
}

impl TraceCollector {
    /// Get summary statistics
    pub fn summary(&self) -> TraceSummary {
        let traces = self.traces.read().unwrap();
        
        if traces.is_empty() {
            return TraceSummary {
                total_queries: 0,
                successful_queries: 0,
                failed_queries: 0,
                avg_duration_ns: 0,
                total_rows: 0,
            };
        }
        
        let total = traces.len();
        let successful = traces.iter().filter(|t| t.success).count();
        let failed = total - successful;
        let total_duration: u64 = traces.iter().map(|t| t.duration_ns).sum();
        let avg_duration = total_duration / total as u64;
        let total_rows: usize = traces.iter().map(|t| t.total_rows).sum();
        
        TraceSummary {
            total_queries: total,
            successful_queries: successful,
            failed_queries: failed,
            avg_duration_ns: avg_duration,
            total_rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_trace_creation() {
        let trace = QueryTrace::new("SELECT * FROM users");
        assert!(!trace.query_id.is_empty());
        assert_eq!(trace.sql, "SELECT * FROM users");
        assert!(trace.success);
    }

    #[test]
    fn test_query_trace_failed() {
        let mut trace = QueryTrace::new("SELECT * FROM users");
        trace.mark_failed("Table not found");
        
        assert!(!trace.success);
        assert!(trace.error_message.is_some());
    }

    #[test]
    fn test_operator_trace() {
        let mut op = OperatorTrace::new("SeqScan", "seq_scan");
        op.record_rows(100);
        op.record_batch();
        
        assert_eq!(op.rows_processed, 100);
        assert_eq!(op.batches_processed, 1);
    }

    #[test]
    fn test_operator_trace_timing() {
        let start = Instant::now();
        let mut op = OperatorTrace::new("SeqScan", "seq_scan");
        
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        op.finish(start);
        
        assert!(op.duration_ns > 0);
    }

    #[test]
    fn test_pipeline_visualization() {
        let mut query = QueryTrace::new("SELECT * FROM users");
        query.total_rows = 100;
        
        let mut root = OperatorTrace::new("Query", "root");
        let mut child = OperatorTrace::new("SeqScan", "seq_scan");
        child.record_rows(100);
        root.add_child(child);
        
        query.root_operator = root;
        
        // Simulate finishing the query
        query.duration_ns = 1000;
        
        let viz = PipelineVisualization::from_trace(&query);
        
        assert!(!viz.nodes.is_empty());
        assert!(viz.duration_ns == 1000);
    }

    #[test]
    fn test_trace_to_json() {
        let trace = QueryTrace::new("SELECT 1");
        let json = trace_to_json(&trace);
        
        assert!(json.contains("SELECT 1"));
        assert!(json.contains("query_id"));
    }

    #[test]
    fn test_global_collector() {
        GLOBAL_TRACE_COLLECTOR.clear();
        
        let trace = QueryTrace::new("SELECT 1");
        GLOBAL_TRACE_COLLECTOR.record(trace);
        
        let all = GLOBAL_TRACE_COLLECTOR.get_all();
        assert_eq!(all.len(), 1);
        
        GLOBAL_TRACE_COLLECTOR.clear();
    }

    #[test]
    fn test_trace_summary() {
        GLOBAL_TRACE_COLLECTOR.clear();
        
        for i in 0..5 {
            let mut trace = QueryTrace::new(&format!("SELECT {}", i));
            trace.duration_ns = 1000;
            trace.total_rows = 10;
            GLOBAL_TRACE_COLLECTOR.record(trace);
        }
        
        let summary = GLOBAL_TRACE_COLLECTOR.summary();
        assert_eq!(summary.total_queries, 5);
        assert_eq!(summary.total_rows, 50);
        
        GLOBAL_TRACE_COLLECTOR.clear();
    }

    #[test]
    fn test_operator_metadata() {
        let mut op = OperatorTrace::new("IndexScan", "index_scan");
        op.add_metadata("table", "users");
        op.add_metadata("index", "idx_id");
        
        assert_eq!(op.metadata.get("table"), Some(&"users".to_string()));
    }

    #[test]
    fn test_pipeline_edges() {
        let mut root = OperatorTrace::new("Query", "root");
        let child = OperatorTrace::new("SeqScan", "seq_scan");
        root.add_child(child);
        
        let mut query = QueryTrace::new("SELECT * FROM test");
        query.root_operator = root;
        
        let viz = PipelineVisualization::from_trace(&query);
        
        // Should have edges from root to child
        assert!(viz.edges.len() >= 1);
    }
}
