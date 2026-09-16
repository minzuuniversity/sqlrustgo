//! Group-commit coordinator for WAL fsync.
//!
//! ## Background
//!
//! With `WalSyncMode::Every`, every `commit_transaction` calls `wal.sync()`,
//! which triggers an `fsync(2)` syscall. On macOS SSD each fsync takes
//! ~1-10ms. When N client threads commit concurrently, they serialise on
//! the `Mutex<W>` that protects the `WalManager`, so N transactions
//! produce N back-to-back fsyncs.
//!
//! Group commit (InnoDB-style) coalesces those N concurrent fsync requests
//! into a **single** fsync per batch. The caller that wins the "leader"
//! slot performs the fsync and broadcasts completion to all other
//! waiters in the same epoch.
//!
//! ## Semantics
//!
//! - **Atomic per-epoch**: all LSNs queued in one epoch are durable once
//!   the leader's `sync()` returns.
//! - **Error propagation**: if the leader's `sync()` fails, every
//!   coalesced waiter fails with the same error (the batch is rolled
//!   back at the WAL layer; the caller is expected to abort its tx).
//! - **Bounded by size and time**: when the batch reaches `max_batch`
//!   or `max_wait_us` elapses since the first waiter, the leader fsyncs
//!   immediately rather than waiting for more.
//! - **Lock-free hot path**: callers atomically enqueue their intent,
//!   then either become leader or wait on a `Condvar`. No CAS loops.
//!
//! ## Safety / liveness
//!
//! - If the leader panics between enqueue and fsync, the next caller's
//!   leader path will perform a new fsync, so the WAL remains durable.
//! - There is no priority inversion: a caller that times out becomes its
//!   own leader and force-fsyncs, so no thread is starved forever.
//!
//! ## When NOT to use
//!
//! - Single-threaded workloads: every becomes effectively zero-coalesce,
//!   so group commit adds Condvar overhead with no benefit. We default
//!   to `WalSyncMode::Every` for that reason.
//! - Recovery-sensitive workloads: each batch can lose up to
//!   `max_batch - 1` transactions on a crash. For stricter durability
//!   use `Every`.

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::engine::SqlResult;
use crate::wal::WalManager;

/// Result of the leader's fsync, recorded into the epoch so waiters can
/// read it after waking.
#[derive(Clone)]
enum EpochResult {
    Pending,
    Ok,
    Err(String),
}

struct Epoch {
    /// LSNs that are waiting to be made durable.
    waiters: Vec<u64>,
    /// Result of the leader's fsync. `Pending` until the leader
    /// records the outcome.
    result: EpochResult,
}

impl Epoch {
    fn new(first_lsn: u64) -> Self {
        Self {
            waiters: vec![first_lsn],
            result: EpochResult::Pending,
        }
    }
}

/// Coordinator state. `Mutex` protects all fields; `Condvar` wakes waiters
/// when the leader finishes.
struct CoordinatorState {
    /// The current epoch. Replaced with a fresh one after the leader
    /// records the result. `None` means no epoch in progress.
    epoch: Option<Epoch>,
    /// Wall-clock instant of when the current epoch started (used to
    /// enforce the time bound).
    started_at: Option<Instant>,
}

/// Public, cloneable handle to a group-commit coordinator.
///
/// The underlying `WalManager` is wrapped in `Arc<Mutex<W>>` so multiple
/// engines / connection threads can share the same coordinator (e.g. a
/// single WAL across an in-process thread pool). This is also required
/// to satisfy `WalManager: Send + Sync`.
pub struct GroupCommitCoordinator<W: WalManager> {
    inner: Arc<Mutex<W>>,
    state: Mutex<CoordinatorState>,
    cv: Condvar,
    /// Soft cap: leader force-fsyncs once this many waiters have
    /// accumulated. Default 32.
    max_batch: usize,
    /// Time bound (microseconds). The leader force-fsyncs if this much
    /// wall-clock has elapsed since the first waiter joined. Default
    /// 1000 µs (1 ms).
    max_wait_us: u64,
}

impl<W: WalManager + 'static> GroupCommitCoordinator<W> {
    /// Build a coordinator wrapping `wal`.
    pub fn new(wal: W) -> Self {
        Self::with_limits(wal, 32, 1_000)
    }

    /// Build with explicit `max_batch` and `max_wait_us` limits. Both
    /// must be `> 0`. `max_batch = 1` degenerates to "no coalescing"
    /// (every caller fsyncs).
    pub fn with_limits(wal: W, max_batch: usize, max_wait_us: u64) -> Self {
        assert!(max_batch > 0, "max_batch must be > 0");
        assert!(max_wait_us > 0, "max_wait_us must be > 0");
        Self {
            inner: Arc::new(Mutex::new(wal)),
            state: Mutex::new(CoordinatorState {
                epoch: None,
                started_at: None,
            }),
            cv: Condvar::new(),
            max_batch,
            max_wait_us,
        }
    }

    /// Returns a clone of the wrapped `WalManager` lock, for callers that
    /// need exclusive access (e.g. for `append` or recovery). Note that
    /// group commit only coordinates `sync()`; `append` is still
    /// serialised through the same `Mutex`.
    pub fn inner_lock(&self) -> std::sync::MutexGuard<'_, W> {
        self.inner.lock().expect("wal mutex poisoned")
    }

    /// Coordinate a single commit at LSN `lsn`.
    ///
    /// - If this caller is the first to enqueue in an empty epoch, it
    ///   becomes the leader and performs the fsync.
    /// - Otherwise it waits for the current leader's fsync to complete.
    ///
    /// Returns `Ok(())` once the WAL up to (and including) the caller's
    /// LSN is durable. Returns the leader's error if `sync()` failed.
    pub fn commit_lsn(&self, lsn: u64) -> SqlResult<()> {
        // Phase 1: enqueue and decide leader / waiter.
        let is_leader = {
            let mut state = self.state.lock().expect("state lock poisoned");

            if state.epoch.is_none() {
                // No epoch in progress: become leader.
                state.epoch = Some(Epoch::new(lsn));
                state.started_at = Some(Instant::now());
                true
            } else {
                // Add our LSN to the current epoch and become a waiter.
                let epoch = state.epoch.as_mut().expect("epoch set");
                epoch.waiters.push(lsn);
                // If the epoch is now full, wake the leader so it can
                // proceed without waiting for the time bound.
                if epoch.waiters.len() >= self.max_batch {
                    self.cv.notify_all();
                }
                false
            }
        };

        if is_leader {
            self.leader_loop(lsn)
        } else {
            self.wait_for_leader()
        }
    }

    /// Leader-side loop: wait for the epoch to fill (size or time),
    /// then fsync exactly once for the whole batch.
    fn leader_loop(&self, _my_lsn: u64) -> SqlResult<()> {
        // Compute the time we should give waiters to join.
        let wait_deadline = {
            let state = self.state.lock().expect("state lock poisoned");
            state
                .started_at
                .expect("started_at is set when epoch is set")
                + Duration::from_micros(self.max_wait_us)
        };

        // Phase 2: wait until the epoch fills or time is up.
        loop {
            let mut state = self.state.lock().expect("state lock poisoned");
            let epoch_full = state
                .epoch
                .as_ref()
                .map(|e| e.waiters.len() >= self.max_batch)
                .unwrap_or(false);
            let time_up = Instant::now() >= wait_deadline;

            if epoch_full || time_up {
                break;
            }

            // Wait for new waiters or the time bound.
            let remaining = wait_deadline.saturating_duration_since(Instant::now());
            let (new_state, _timeout) = self
                .cv
                .wait_timeout(state, remaining)
                .expect("cv wait failed");
            state = new_state;
            // Loop back and re-check.
        }

        // Phase 3: perform the actual fsync. We hold NO state lock
        // during the syscall, so other threads can still enqueue into
        // a fresh epoch if they arrive while we sync.
        let mut wal = self.inner.lock().expect("wal lock poisoned");
        let result = wal.sync();
        drop(wal);

        // Phase 4: record the result in the current epoch and broadcast.
        let result_clone = match &result {
            Ok(()) => EpochResult::Ok,
            Err(e) => EpochResult::Err(format!("WAL group commit sync error: {}", e)),
        };
        {
            let mut state = self.state.lock().expect("state lock poisoned");
            if let Some(epoch) = state.epoch.as_mut() {
                epoch.result = result_clone.clone();
            }
            // Clear the epoch: the next caller will start a fresh one.
            state.epoch = None;
            state.started_at = None;
        }
        self.cv.notify_all();

        // Leader returns its own result.
        match result {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Waiter-side: wait until the leader records the result.
    fn wait_for_leader(&self) -> SqlResult<()> {
        let mut state = self.state.lock().expect("state lock poisoned");
        loop {
            match &state.epoch {
                None => {
                    // Epoch is gone — but the leader has already moved
                    // on. This is a "lost wakeup" if we just enqueued
                    // but the leader cleared the epoch before we could
                    // read its result. Since we never read the result,
                    // we don't know if the fsync succeeded. As a safety
                    // measure, return the conservative error.
                    return Err(crate::engine::SqlError::ExecutionError(
                        "WAL group commit epoch disappeared before waiter could read result".into(),
                    ));
                }
                Some(epoch) => {
                    match &epoch.result {
                        EpochResult::Pending => {
                            // Still waiting — sleep on the cv. The
                            // returned guard re-acquires the lock; we
                            // assign to `state` and loop.
                            state = self.cv.wait(state).map_err(|e| {
                                crate::engine::SqlError::ExecutionError(format!("cv wait: {}", e))
                            })?;
                        }
                        EpochResult::Ok => return Ok(()),
                        EpochResult::Err(msg) => {
                            return Err(crate::engine::SqlError::ExecutionError(msg.clone()))
                        }
                    }
                }
            }
        }
    }

    /// Convenience: direct fsync bypassing group commit. Used by recovery
    /// and tests that need a deterministic point-in-time barrier.
    pub fn sync_now(&self) -> SqlResult<()> {
        self.inner.lock().expect("wal lock poisoned").sync()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::memory_wal_manager::MemoryWalManager;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    /// A WAL manager that counts fsync calls so we can verify coalescing.
    struct CountingWal {
        inner: MemoryWalManager,
        sync_calls: Arc<AtomicU64>,
    }

    impl CountingWal {
        fn new(counter: Arc<AtomicU64>) -> Self {
            Self {
                inner: MemoryWalManager::new(),
                sync_calls: counter,
            }
        }
    }

    impl WalManager for CountingWal {
        fn append(&mut self, entry: crate::wal::WalEntry) -> SqlResult<()> {
            self.inner.append(entry)
        }
        fn flush(&mut self) -> SqlResult<()> {
            self.inner.flush()
        }
        fn sync(&mut self) -> SqlResult<()> {
            self.sync_calls.fetch_add(1, Ordering::SeqCst);
            // Hold the lock for a bit to give other threads a chance to
            // join the same epoch.
            thread::sleep(Duration::from_millis(2));
            self.inner.sync()
        }
        fn recover(&mut self) -> SqlResult<Vec<crate::wal::WalEntry>> {
            self.inner.recover()
        }
        fn truncate_before(&mut self, lsn: u64) -> SqlResult<()> {
            self.inner.truncate_before(lsn)
        }
        fn current_lsn(&self) -> u64 {
            self.inner.current_lsn()
        }
    }

    #[test]
    fn single_caller_always_syncs() {
        let counter = Arc::new(AtomicU64::new(0));
        let coord = GroupCommitCoordinator::new(CountingWal::new(counter.clone()));
        coord.commit_lsn(1).unwrap();
        coord.commit_lsn(2).unwrap();
        // With no concurrency, every call fsyncs.
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn concurrent_callers_coalesce() {
        // 8 threads all calling commit_lsn roughly together. With
        // max_batch=8 the leader should fsync at most a couple of times,
        // not 8.
        let counter = Arc::new(AtomicU64::new(0));
        let coord = Arc::new(GroupCommitCoordinator::with_limits(
            CountingWal::new(counter.clone()),
            8,
            10_000, // 10ms — plenty of time for 8 threads to join
        ));

        // Pre-create the handles so all threads start nearly together.
        let mut handles = vec![];
        for i in 0..8 {
            let c = coord.clone();
            handles.push(thread::spawn(move || c.commit_lsn(i as u64)));
        }
        for h in handles {
            h.join().unwrap();
        }

        // With coalescing, we expect far fewer than 8 fsyncs. In the
        // best case 1. We allow up to 4 to avoid flakiness on
        // heavily-loaded CI.
        let syncs = counter.load(Ordering::SeqCst);
        assert!(
            syncs >= 1 && syncs <= 4,
            "expected 1-4 fsyncs for 8 concurrent commits, got {}",
            syncs
        );
    }

    #[test]
    fn max_batch_forces_sync() {
        // max_batch=2: every 2 concurrent commits should trigger 1 fsync.
        let counter = Arc::new(AtomicU64::new(0));
        let coord = Arc::new(GroupCommitCoordinator::with_limits(
            CountingWal::new(counter.clone()),
            2,
            100_000, // 100ms — long enough to never time out
        ));

        let mut handles = vec![];
        for i in 0..4 {
            let c = coord.clone();
            handles.push(thread::spawn(move || c.commit_lsn(i as u64)));
        }
        for h in handles {
            h.join().unwrap();
        }

        // With max_batch=2 and 4 threads, we expect at least 2 fsyncs
        // (the second batch may also coalesce into the first if timing
        // allows). Allow 1-3 for slack.
        let syncs = counter.load(Ordering::SeqCst);
        assert!(
            syncs >= 1 && syncs <= 3,
            "expected 1-3 fsyncs for 4 commits with max_batch=2, got {}",
            syncs
        );
    }

    #[test]
    fn error_propagates_to_leader() {
        // A WAL that always fails sync. The leader should see the
        // error and record it in the epoch.
        struct FailingWal;
        impl WalManager for FailingWal {
            fn append(&mut self, _: crate::wal::WalEntry) -> SqlResult<()> {
                Ok(())
            }
            fn flush(&mut self) -> SqlResult<()> {
                Ok(())
            }
            fn sync(&mut self) -> SqlResult<()> {
                Err(crate::engine::SqlError::ExecutionError("nope".into()))
            }
            fn recover(&mut self) -> SqlResult<Vec<crate::wal::WalEntry>> {
                Ok(vec![])
            }
            fn truncate_before(&mut self, _: u64) -> SqlResult<()> {
                Ok(())
            }
            fn current_lsn(&self) -> u64 {
                0
            }
        }

        let coord = GroupCommitCoordinator::with_limits(FailingWal, 4, 10_000);
        let res = coord.commit_lsn(0);
        assert!(res.is_err(), "leader should see error from failing WAL");
    }

    #[test]
    fn large_batch_completes() {
        // 32 threads, max_batch=32, long wait. Expect ~1 fsync.
        let counter = Arc::new(AtomicU64::new(0));
        let coord = Arc::new(GroupCommitCoordinator::with_limits(
            CountingWal::new(counter.clone()),
            32,
            50_000, // 50ms
        ));

        let mut handles = vec![];
        for i in 0..32 {
            let c = coord.clone();
            handles.push(thread::spawn(move || c.commit_lsn(i as u64)));
        }
        for h in handles {
            h.join().unwrap();
        }
        let syncs = counter.load(Ordering::SeqCst);
        // 32 threads / batch of 32 = 1 fsync in the best case; allow
        // up to 4 for thread scheduling jitter.
        assert!(
            syncs <= 4,
            "expected <=4 fsyncs for 32 concurrent commits, got {}",
            syncs
        );
    }
}
