//! F-31: performance_schema (runtime instrumentation)
//!
//! **Issue**: #2830
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-31)
//! **Change**: openspec/changes/f-31-performance-schema
//!
//! In-memory performance instrumentation. Real integration in v3.9.0.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct StatementMetrics {
    pub count: u64,
    pub total_time_ns: u64,
    pub max_time_ns: u64,
    pub lock_time_ns: u64,
}

impl StatementMetrics {
    pub fn avg_time_ns(&self) -> u64 {
        if self.count == 0 {
            0
        } else {
            self.total_time_ns / self.count
        }
    }

    pub fn record(&mut self, time_ns: u64) {
        self.count += 1;
        self.total_time_ns += time_ns;
        if time_ns > self.max_time_ns {
            self.max_time_ns = time_ns;
        }
    }
}

#[derive(Debug, Clone)]
pub struct MutexEvent {
    pub name: String,
    pub timestamp_ns: u64,
    pub duration_ns: u64,
}

#[derive(Debug, Clone)]
pub struct FileIoEvent {
    pub file: String,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub fsync_count: u64,
}

pub struct PerformanceSchema {
    statements: Arc<RwLock<HashMap<String, StatementMetrics>>>,
    mutex_events: Arc<RwLock<Vec<MutexEvent>>>,
    file_io: Arc<RwLock<HashMap<String, FileIoEvent>>>,
    enabled: Arc<RwLock<bool>>,
}

impl PerformanceSchema {
    pub fn new() -> Self {
        Self {
            statements: Arc::new(RwLock::new(HashMap::new())),
            mutex_events: Arc::new(RwLock::new(Vec::new())),
            file_io: Arc::new(RwLock::new(HashMap::new())),
            enabled: Arc::new(RwLock::new(false)),
        }
    }

    pub fn enable(&self) {
        *self.enabled.write().unwrap() = true;
    }

    pub fn disable(&self) {
        *self.enabled.write().unwrap() = false;
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read().unwrap()
    }

    pub fn record_statement(&self, sql: &str, time: Duration) {
        if !self.is_enabled() {
            return;
        }
        let mut stats = self.statements.write().unwrap();
        let entry = stats.entry(sql.to_string()).or_default();
        entry.record(time.as_nanos() as u64);
    }

    pub fn record_mutex_wait(&self, name: &str, duration_ns: u64) {
        if !self.is_enabled() {
            return;
        }
        self.mutex_events.write().unwrap().push(MutexEvent {
            name: name.to_string(),
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            duration_ns,
        });
    }

    pub fn record_file_io(&self, file: &str, bytes_read: u64, bytes_written: u64) {
        if !self.is_enabled() {
            return;
        }
        let mut io = self.file_io.write().unwrap();
        let entry = io.entry(file.to_string()).or_insert_with(|| FileIoEvent {
            file: file.to_string(),
            bytes_read: 0,
            bytes_written: 0,
            fsync_count: 0,
        });
        entry.bytes_read += bytes_read;
        entry.bytes_written += bytes_written;
    }

    pub fn record_fsync(&self, file: &str) {
        if !self.is_enabled() {
            return;
        }
        let mut io = self.file_io.write().unwrap();
        let entry = io.entry(file.to_string()).or_insert_with(|| FileIoEvent {
            file: file.to_string(),
            bytes_read: 0,
            bytes_written: 0,
            fsync_count: 0,
        });
        entry.fsync_count += 1;
    }

    pub fn get_statement(&self, sql: &str) -> Option<StatementMetrics> {
        self.statements.read().unwrap().get(sql).cloned()
    }

    pub fn all_statements(&self) -> HashMap<String, StatementMetrics> {
        self.statements.read().unwrap().clone()
    }

    pub fn mutex_event_count(&self) -> usize {
        self.mutex_events.read().unwrap().len()
    }

    pub fn file_io_count(&self) -> usize {
        self.file_io.read().unwrap().len()
    }

    pub fn reset(&self) {
        self.statements.write().unwrap().clear();
        self.mutex_events.write().unwrap().clear();
        self.file_io.write().unwrap().clear();
    }
}

impl Default for PerformanceSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_statement_metrics_basic() {
    let ps = PerformanceSchema::new();
    ps.enable();
    ps.record_statement("SELECT 1", Duration::from_millis(10));
    ps.record_statement("SELECT 1", Duration::from_millis(20));
    let m = ps.get_statement("SELECT 1").unwrap();
    assert_eq!(m.count, 2);
    assert_eq!(m.total_time_ns, 30_000_000);
    assert_eq!(m.max_time_ns, 20_000_000);
    assert_eq!(m.avg_time_ns(), 15_000_000);
}

#[test]
fn test_statement_metrics_disabled() {
    let ps = PerformanceSchema::new();
    // NOT enabled
    ps.record_statement("SELECT 1", Duration::from_millis(10));
    assert!(ps.get_statement("SELECT 1").is_none());
}

#[test]
fn test_mutex_wait_tracking() {
    let ps = PerformanceSchema::new();
    ps.enable();
    ps.record_mutex_wait("log_buffer_lock", 1000);
    ps.record_mutex_wait("log_buffer_lock", 2000);
    assert_eq!(ps.mutex_event_count(), 2);
}

#[test]
fn test_file_io_tracking() {
    let ps = PerformanceSchema::new();
    ps.enable();
    ps.record_file_io("data.db", 1024, 0);
    ps.record_file_io("data.db", 1024, 512);
    ps.record_fsync("data.db");
    assert_eq!(ps.file_io_count(), 1);
}

#[test]
fn test_reset_metrics() {
    let ps = PerformanceSchema::new();
    ps.enable();
    ps.record_statement("SELECT 1", Duration::from_millis(10));
    ps.record_mutex_wait("lock", 100);
    ps.record_file_io("f", 100, 0);
    assert_eq!(ps.all_statements().len(), 1);
    ps.reset();
    assert!(ps.all_statements().is_empty());
    assert_eq!(ps.mutex_event_count(), 0);
    assert_eq!(ps.file_io_count(), 0);
}

#[test]
fn test_enable_disable_toggle() {
    let ps = PerformanceSchema::new();
    assert!(!ps.is_enabled());
    ps.enable();
    assert!(ps.is_enabled());
    ps.disable();
    assert!(!ps.is_enabled());
}

#[test]
fn test_multiple_statements_tracked() {
    let ps = PerformanceSchema::new();
    ps.enable();
    ps.record_statement("SELECT * FROM t1", Duration::from_millis(5));
    ps.record_statement("INSERT INTO t2 VALUES (1)", Duration::from_millis(15));
    ps.record_statement("UPDATE t3 SET x=1", Duration::from_millis(8));
    let all = ps.all_statements();
    assert_eq!(all.len(), 3);
}
