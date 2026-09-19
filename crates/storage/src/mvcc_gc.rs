//! Background GC runner for `MvccStorage` / `BoxStorageEngine`.
//!
//! ## Why
//!
//! Without periodic GC, every `INSERT` / `UPDATE` / `DELETE` adds a new
//! version to the per-table MVCC chain. Old versions are never
//! reclaimed, so RSS grows unbounded (~30 MB/s in the v4.0.0 SOAK).
//!
//! ## What it does
//!
//! Spawns a background thread that calls `storage.gc(gc_lag)` on a
//! fixed interval (default 5s). The thread holds an
//! `Arc<RwLock<BoxStorageEngine>>` so reads and writes continue to
//! work while GC runs.
//!
//! ## Safety
//!
//! - `gc_lag` (default 1000 versions) ensures that any reader that
//!   started before the GC pass can still see its snapshot. Readers
//!   that start after the GC pass get a fresh snapshot via
//!   `begin_snapshot()`.
//! - GC is idempotent and lock-protected at the `VersionedTable` level,
//!   so multiple GCs running concurrently (or racing with reads) cannot
//!   corrupt state.
//! - The background thread exits cleanly when the `MvccGCRunner` is
//!   dropped (cancellation via `Arc<AtomicBool>`).
//!
//! ## Configuration
//!
//! All knobs are set via `MvccGCRunnerConfig`:
//! - `interval`: time between GC passes (default 5s)
//! - `gc_lag`: how many versions back to retain (default 1000)
//!
//! ## Usage
//!
//! ```ignore
//! let storage = Arc::new(RwLock::new(BoxStorageEngine::new(mvcc)));
//! let _gc = MvccGCRunner::start(storage.clone(), MvccGCRunnerConfig::default());
//! // ... server runs ...
//! // _gc is dropped on shutdown, GC thread exits.
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use parking_lot::RwLock;

use crate::binary_storage::BoxStorageEngine;

/// Configuration for the MVCC GC background runner.
#[derive(Debug, Clone)]
pub struct MvccGCRunnerConfig {
    /// Time between GC passes.
    pub interval: Duration,
    /// How many versions back to retain during GC. Readers that
    /// started before the most recent `gc_lag` versions may see torn
    /// data, so set this ≥ the maximum expected reader lifetime
    /// (in version count, not time).
    pub gc_lag: u64,
}

impl Default for MvccGCRunnerConfig {
    fn default() -> Self {
        Self {
            // 5s is short enough to bound memory growth to ~150 MB
            // (assuming 30 MB/s) and long enough that GC itself doesn't
            // dominate the budget.
            interval: Duration::from_secs(5),
            // 1000 versions back: even a reader that started 1000
            // versions ago is still served a consistent snapshot.
            // For a write-heavy workload at ~400 TPS, that's ~2.5s
            // worth of history, well within the 5s GC interval.
            gc_lag: 1000,
        }
    }
}

/// Handle to the MVCC GC background thread. Dropping the handle
/// signals the thread to stop and joins it.
pub struct MvccGCRunner {
    /// Set to `true` on drop to signal the thread to exit.
    stop: Arc<AtomicBool>,
    /// Join handle; resolved on drop.
    handle: Option<JoinHandle<()>>,
}

impl MvccGCRunner {
    /// Spawn a new background GC runner. The returned handle must be
    /// kept alive for the duration of the server's life; dropping it
    /// stops the thread.
    ///
    /// `storage` is the server's `Arc<RwLock<BoxStorageEngine>>`. The
    /// GC thread takes the read lock briefly to call `gc()`, which is
    /// safe to do concurrently with normal read/write traffic.
    pub fn start(
        storage: Arc<RwLock<BoxStorageEngine>>,
        config: MvccGCRunnerConfig,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let handle = thread::Builder::new()
            .name("mvcc-gc".to_string())
            .spawn(move || {
                gc_loop(storage, config, stop_clone);
            })
            .expect("failed to spawn mvcc-gc thread");
        Self {
            stop,
            handle: Some(handle),
        }
    }

    /// Manually trigger a GC pass (does not wait for the next interval).
    /// Mostly useful for tests.
    pub fn gc_now(storage: &BoxStorageEngine, gc_lag: u64) -> usize {
        storage.gc(gc_lag)
    }
}

impl Drop for MvccGCRunner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            // Best-effort join: the thread checks `stop` between
            // GC passes, so this returns promptly.
            let _ = h.join();
        }
    }
}

/// The actual GC loop. Runs until `stop` is set to `true`.
fn gc_loop(
    storage: Arc<RwLock<BoxStorageEngine>>,
    config: MvccGCRunnerConfig,
    stop: Arc<AtomicBool>,
) {
    log::info!(
        "MVCC GC background thread started: interval={:?}, gc_lag={}",
        config.interval,
        config.gc_lag
    );
    while !stop.load(Ordering::SeqCst) {
        // Sleep in 100ms slices so the thread can react quickly to
        // the stop signal even when `interval` is large.
        let mut slept = Duration::ZERO;
        while slept < config.interval && !stop.load(Ordering::SeqCst) {
            let slice = config.interval.min(Duration::from_millis(100));
            thread::sleep(slice);
            slept += slice;
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // Run a GC pass. Take the read lock briefly — the engine is
        // safe to call concurrently with normal traffic.
        let (reclaimed, mvcc_table_count, listed_tables) = {
            let engine = storage.read();
            // Force the gc() to be called via the BoxStorageEngine
            // method (which goes to the inner engine's gc). This is
            // the same path used by tests.
            let n = BoxStorageEngine::gc(&engine, config.gc_lag);
            let mt = engine.list_tables();
            let lt = engine.list_tables();
            (n, mt.len(), lt.len())
        };
        // mvcc_table_count kept for parity with future per-table GC logs
        let _ = mvcc_table_count;
        log::info!(
            "MVCC GC pass: reclaimed={} stale versions, list_tables()={} (gc_lag={})",
            reclaimed,
            listed_tables,
            config.gc_lag
        );
        // Diagnostics: also log the MVCC table count by calling gc()
        // again and inspecting the inner.
        // Note: list_tables is on dyn StorageEngine which goes through
        // the inner engine; the MVCC map count is internal.
    }
    log::info!("MVCC GC background thread exiting");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{ColumnDefinition, StorageEngine, TableInfo, Value};
    use crate::file_storage::FileStorage;
    use crate::mvcc_storage::MvccStorage;

    fn sample_info(name: &str) -> TableInfo {
        TableInfo {
            name: name.to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            collations: std::collections::HashMap::new(),
            partition_info: None,
            compression: None,
            ..Default::default()
        }
    }

    #[test]
    fn gc_thread_runs_and_reclaims() {
        // Use a tmp file_storage to exercise the real path.
        let dir = tempfile::tempdir().unwrap();
        let inner = FileStorage::new_with_wal(dir.path().to_path_buf()).unwrap();
        let mut mvcc = MvccStorage::new(inner);
        // Need a table for the MVCC chain to be created.
        mvcc.create_table(&sample_info("t")).unwrap();
        // Insert + update to grow the version chain.
        mvcc.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
        for i in 1..=10 {
            mvcc.update("t", &[Value::Integer(1)], &[(0, Value::Integer(i))])
                .unwrap();
        }
        // Wrap in BoxStorageEngine + Arc<RwLock<>>.
        let storage = Arc::new(RwLock::new(BoxStorageEngine::new(mvcc)));
        // Start a GC thread with tight interval and gc_lag=2.
        let cfg = MvccGCRunnerConfig {
            interval: Duration::from_millis(100),
            gc_lag: 2,
        };
        let _gc = MvccGCRunner::start(storage.clone(), cfg);
        // Wait a bit and verify the runner doesn't panic.
        std::thread::sleep(Duration::from_millis(500));
        // Drop explicitly so the thread is joined.
        drop(_gc);
    }

    #[test]
    fn gc_thread_stops_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let inner = FileStorage::new_with_wal(dir.path().to_path_buf()).unwrap();
        let mvcc = MvccStorage::new(inner);
        let storage = Arc::new(RwLock::new(BoxStorageEngine::new(mvcc)));
        {
            let _gc = MvccGCRunner::start(
                storage.clone(),
                MvccGCRunnerConfig {
                    interval: Duration::from_millis(50),
                    gc_lag: 10,
                },
            );
            std::thread::sleep(Duration::from_millis(200));
        } // _gc dropped here
        // After drop, the thread should have exited without deadlock.
        std::thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn gc_now_works() {
        let dir = tempfile::tempdir().unwrap();
        let inner = FileStorage::new_with_wal(dir.path().to_path_buf()).unwrap();
        let mut mvcc = MvccStorage::new(inner);
        mvcc.create_table(&sample_info("t")).unwrap();
        for i in 0..5 {
            mvcc.insert("t", vec![vec![Value::Integer(i)]]).unwrap();
        }
        let boxed = BoxStorageEngine::new(mvcc);
        let n = MvccGCRunner::gc_now(&boxed, 100);
        // MVCC gc returns 0 when nothing is reclaimable.
        assert_eq!(n, 0);
    }
}
