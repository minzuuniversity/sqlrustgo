//! V311-06 (F-31 Performance Schema instrumentation hooks)
//!
//! Provides a trait-based observer pattern for query execution events.
//! Designed for zero-cost default (NoopInstrumentationHook) so production
//! queries don't pay measurable overhead.
//!
//! Use `CountingInstrumentationHook` from tests/monitoring to observe
//! operator behavior without parsing log output.
//!
//! ## Example
//!
//! ```rust
//! use sqlrustgo_executor::instrumentation::{CountingInstrumentationHook, InstrumentationHook};
//! use std::sync::Arc;
//!
//! let hook = Arc::new(CountingInstrumentationHook::new());
//! // ... pass hook into execution engine ...
//! // after running query:
//! assert!(hook.seq_scan_count() >= 1);
//! assert!(hook.filter_count() >= 1);
//! ```

use std::sync::atomic::{AtomicU64, Ordering};

/// Trait for observing query execution events.
///
/// All methods have default no-op implementations, so observers only
/// override what they care about. The default `NoopInstrumentationHook`
/// is zero-cost: all methods are inline-empty and the compiler dead-codes
/// the trait dispatch overhead.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` because queries may run on
/// parallel rayon workers. Use atomics or locks for any mutable state.
pub trait InstrumentationHook: Send + Sync {
    /// SeqScan operator started iteration.
    fn on_seq_scan_start(&self, _table: &str) {}

    /// Filter operator started. `rows_in` is the rows received from child.
    fn on_filter_start(&self, _table: &str, _rows_in: usize) {}

    /// Filter operator finished. `rows_out` is rows passing the predicate.
    fn on_filter_end(&self, _table: &str, _rows_out: usize) {}

    /// Project operator started.
    fn on_project_start(&self, _table: &str, _rows_in: usize) {}

    /// Project operator finished.
    fn on_project_end(&self, _table: &str, _rows_out: usize) {}

    /// HashJoin build phase started (constructing hash table from `side`).
    fn on_hash_join_build(&self, _side: &str) {}

    /// HashJoin probe phase started. `rows` is the count from probe side.
    fn on_hash_join_probe(&self, _rows: usize) {}

    /// Aggregate operator started. `groups` is the GROUP BY cardinality estimate.
    fn on_aggregate_start(&self, _groups: usize) {}

    /// Sort operator started.
    fn on_sort_start(&self, _rows: usize) {}

    /// Generic: query complete. `wall_us` is total query wall time.
    fn on_query_complete(&self, _wall_us: u64) {}
}

/// Zero-cost no-op hook. Use as the default to avoid overhead in production.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopInstrumentationHook;

impl InstrumentationHook for NoopInstrumentationHook {}

/// Counting hook with `AtomicU64` counters. For tests and benchmarks.
/// Use Relaxed ordering — these are just counters, no happens-before needed.
pub struct CountingInstrumentationHook {
    pub seq_scan_count: AtomicU64,
    pub filter_count: AtomicU64,
    pub filter_rows_in_total: AtomicU64,
    pub filter_rows_out_total: AtomicU64,
    pub project_count: AtomicU64,
    pub project_rows_in_total: AtomicU64,
    pub project_rows_out_total: AtomicU64,
    pub hash_join_build_count: AtomicU64,
    pub hash_join_probe_count: AtomicU64,
    pub hash_join_probe_rows_total: AtomicU64,
    pub aggregate_count: AtomicU64,
    pub sort_count: AtomicU64,
    pub query_complete_count: AtomicU64,
}

impl CountingInstrumentationHook {
    pub fn new() -> Self {
        Self {
            seq_scan_count: AtomicU64::new(0),
            filter_count: AtomicU64::new(0),
            filter_rows_in_total: AtomicU64::new(0),
            filter_rows_out_total: AtomicU64::new(0),
            project_count: AtomicU64::new(0),
            project_rows_in_total: AtomicU64::new(0),
            project_rows_out_total: AtomicU64::new(0),
            hash_join_build_count: AtomicU64::new(0),
            hash_join_probe_count: AtomicU64::new(0),
            hash_join_probe_rows_total: AtomicU64::new(0),
            aggregate_count: AtomicU64::new(0),
            sort_count: AtomicU64::new(0),
            query_complete_count: AtomicU64::new(0),
        }
    }

    pub fn seq_scan_count(&self) -> u64 {
        self.seq_scan_count.load(Ordering::Relaxed)
    }
    pub fn filter_count(&self) -> u64 {
        self.filter_count.load(Ordering::Relaxed)
    }
    pub fn project_count(&self) -> u64 {
        self.project_count.load(Ordering::Relaxed)
    }
    pub fn hash_join_build_count(&self) -> u64 {
        self.hash_join_build_count.load(Ordering::Relaxed)
    }
    pub fn hash_join_probe_count(&self) -> u64 {
        self.hash_join_probe_count.load(Ordering::Relaxed)
    }
    pub fn aggregate_count(&self) -> u64 {
        self.aggregate_count.load(Ordering::Relaxed)
    }
    pub fn sort_count(&self) -> u64 {
        self.sort_count.load(Ordering::Relaxed)
    }
    pub fn query_complete_count(&self) -> u64 {
        self.query_complete_count.load(Ordering::Relaxed)
    }
    pub fn filter_rows_in_total(&self) -> u64 {
        self.filter_rows_in_total.load(Ordering::Relaxed)
    }
    pub fn filter_rows_out_total(&self) -> u64 {
        self.filter_rows_out_total.load(Ordering::Relaxed)
    }
    pub fn project_rows_in_total(&self) -> u64 {
        self.project_rows_in_total.load(Ordering::Relaxed)
    }
    pub fn project_rows_out_total(&self) -> u64 {
        self.project_rows_out_total.load(Ordering::Relaxed)
    }
    pub fn hash_join_probe_rows_total(&self) -> u64 {
        self.hash_join_probe_rows_total.load(Ordering::Relaxed)
    }
}

impl Default for CountingInstrumentationHook {
    fn default() -> Self {
        Self::new()
    }
}

impl InstrumentationHook for CountingInstrumentationHook {
    fn on_seq_scan_start(&self, _table: &str) {
        self.seq_scan_count.fetch_add(1, Ordering::Relaxed);
    }
    fn on_filter_start(&self, _table: &str, rows_in: usize) {
        self.filter_count.fetch_add(1, Ordering::Relaxed);
        self.filter_rows_in_total
            .fetch_add(rows_in as u64, Ordering::Relaxed);
    }
    fn on_filter_end(&self, _table: &str, rows_out: usize) {
        self.filter_rows_out_total
            .fetch_add(rows_out as u64, Ordering::Relaxed);
    }
    fn on_project_start(&self, _table: &str, rows_in: usize) {
        self.project_count.fetch_add(1, Ordering::Relaxed);
        self.project_rows_in_total
            .fetch_add(rows_in as u64, Ordering::Relaxed);
    }
    fn on_project_end(&self, _table: &str, rows_out: usize) {
        self.project_rows_out_total
            .fetch_add(rows_out as u64, Ordering::Relaxed);
    }
    fn on_hash_join_build(&self, _side: &str) {
        self.hash_join_build_count.fetch_add(1, Ordering::Relaxed);
    }
    fn on_hash_join_probe(&self, rows: usize) {
        self.hash_join_probe_count.fetch_add(1, Ordering::Relaxed);
        self.hash_join_probe_rows_total
            .fetch_add(rows as u64, Ordering::Relaxed);
    }
    fn on_aggregate_start(&self, _groups: usize) {
        self.aggregate_count.fetch_add(1, Ordering::Relaxed);
    }
    fn on_sort_start(&self, _rows: usize) {
        self.sort_count.fetch_add(1, Ordering::Relaxed);
    }
    fn on_query_complete(&self, _wall_us: u64) {
        self.query_complete_count.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_compiles_and_is_default() {
        let hook = NoopInstrumentationHook;
        hook.on_seq_scan_start("t");
        hook.on_query_complete(0);
        // No-op should not panic
    }

    #[test]
    fn counting_records_events() {
        let hook = CountingInstrumentationHook::new();
        hook.on_seq_scan_start("users");
        hook.on_seq_scan_start("orders");
        hook.on_filter_start("users", 100);
        hook.on_filter_end("users", 50);
        hook.on_query_complete(123);

        assert_eq!(hook.seq_scan_count(), 2);
        assert_eq!(hook.filter_count(), 1);
        assert_eq!(hook.filter_rows_in_total(), 100);
        assert_eq!(hook.filter_rows_out_total(), 50);
        assert_eq!(hook.query_complete_count(), 1);
    }

    #[test]
    fn counting_concurrent_increments() {
        use std::sync::Arc;
        use std::thread;
        let hook = Arc::new(CountingInstrumentationHook::new());
        let mut handles = vec![];
        for _ in 0..4 {
            let h = Arc::clone(&hook);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    h.on_seq_scan_start("t");
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }
    #[test]
    fn counting_records_project_events() {
        let hook = CountingInstrumentationHook::new();
        hook.on_project_start("t", 100);
        hook.on_project_end("t", 50);
        assert_eq!(hook.project_count(), 1);
        assert_eq!(hook.project_rows_in_total(), 100);
        assert_eq!(hook.project_rows_out_total(), 50);
    }

    #[test]
    fn counting_records_hash_join_events() {
        let hook = CountingInstrumentationHook::new();
        hook.on_hash_join_build("build");
        hook.on_hash_join_probe(50);
        hook.on_hash_join_probe(75);
        assert_eq!(hook.hash_join_build_count(), 1);
        assert_eq!(hook.hash_join_probe_count(), 2);
        assert_eq!(hook.hash_join_probe_rows_total(), 125);
    }

    #[test]
    fn counting_records_aggregate_events() {
        let hook = CountingInstrumentationHook::new();
        hook.on_aggregate_start(1000);
        assert_eq!(hook.aggregate_count(), 1);
    }

    #[test]
    fn counting_records_sort_events() {
        let hook = CountingInstrumentationHook::new();
        hook.on_sort_start(500);
        assert_eq!(hook.sort_count(), 1);
    }

    #[test]
    fn counting_default_matches_new() {
        let hook: CountingInstrumentationHook = Default::default();
        assert_eq!(hook.seq_scan_count(), 0);
        assert_eq!(hook.query_complete_count(), 0);
    }

    #[test]
    fn noop_clone_is_zero_sized() {
        let h1 = NoopInstrumentationHook;
        let h2 = h1; // Copy
                     // Calls on both should still no-op
        h1.on_seq_scan_start("a");
        h2.on_query_complete(1);
    }
}
