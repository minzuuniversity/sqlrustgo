//! V400-05 cross-model write tracker — global slot.
//!
//! Avoids per-instance state and circular crate dependencies by using
//! a process-global tracker. VectorStore, DiskGraphStore, audit chain,
//! etc. all call the same `register_*` helpers to publish writes to
//! the current transaction.

use parking_lot::Mutex;
use std::sync::Arc;

use crate::CrossModelWriteTracker;

/// Process-global tracker slot. Set once at server startup via
/// `set_global_tracker`; queried by every write path.
static GLOBAL_TRACKER: Mutex<Option<Arc<dyn CrossModelWriteTracker>>> = Mutex::new(None);

/// V400-05 model kind values for the global tracker. Match the
/// `ModelKind` enum in `sqlrustgo_transaction`:
/// 0 = Sql, 1 = Vector, 2 = Graph, 3 = Audit.
pub const MODEL_SQL: u8 = 0;
pub const MODEL_VECTOR: u8 = 1;
pub const MODEL_GRAPH: u8 = 2;
pub const MODEL_AUDIT: u8 = 3;

/// Install the process-global tracker. Pass `None` to clear.
pub fn set_global_tracker(tracker: Option<Arc<dyn CrossModelWriteTracker>>) {
    *GLOBAL_TRACKER.lock() = tracker;
}

/// Read the current tracker (if any).
pub fn get_global_tracker() -> Option<Arc<dyn CrossModelWriteTracker>> {
    GLOBAL_TRACKER.lock().clone()
}

/// Helper: register an SQL write (kind=0).
pub fn register_sql_write(description: &str) {
    if let Some(t) = GLOBAL_TRACKER.lock().as_ref() {
        t.register_write(MODEL_SQL, description);
    }
}

/// Helper: register a vector write (kind=1).
pub fn register_vector_write(description: &str) {
    if let Some(t) = GLOBAL_TRACKER.lock().as_ref() {
        t.register_write(MODEL_VECTOR, description);
    }
}

/// Helper: register a graph write (kind=2).
pub fn register_graph_write(description: &str) {
    if let Some(t) = GLOBAL_TRACKER.lock().as_ref() {
        t.register_write(MODEL_GRAPH, description);
    }
}

/// Helper: register an audit write (kind=3).
pub fn register_audit_write(description: &str) {
    if let Some(t) = GLOBAL_TRACKER.lock().as_ref() {
        t.register_write(MODEL_AUDIT, description);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingTracker {
        count: AtomicUsize,
    }
    impl CrossModelWriteTracker for CountingTracker {
        fn register_write(&self, _kind: u8, _desc: &str) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn global_tracker_round_trip() {
        set_global_tracker(None);
        register_sql_write("test");
        assert!(get_global_tracker().is_none());
    }

    #[test]
    fn register_with_tracker_records() {
        let counter = Arc::new(CountingTracker {
            count: AtomicUsize::new(0),
        });
        set_global_tracker(Some(counter.clone()));
        register_sql_write("x");
        register_vector_write("y");
        register_graph_write("z");
        register_audit_write("w");
        assert_eq!(counter.count.load(Ordering::SeqCst), 4);
        set_global_tracker(None);
    }

    #[test]
    fn each_model_kind_distinct() {
        // Just verify the four constants are distinct
        assert_ne!(MODEL_SQL, MODEL_VECTOR);
        assert_ne!(MODEL_VECTOR, MODEL_GRAPH);
        assert_ne!(MODEL_GRAPH, MODEL_AUDIT);
    }
}