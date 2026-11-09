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
//! - A leader that dies inside its epoch — a panic unwinding through
//!   phase-3/4 — is caught by a Drop guard installed when leadership is
//!   claimed (#5686, WALGC-2): the guard closes the epoch with an explicit
//!   `Err`, wakes every joined waiter, and frees the epoch slot so the next
//!   caller starts a fresh epoch. The LSNs queued in a dead epoch were
//!   appended but never fsynced: every caller in that batch learns its
//!   commit failed and must abort its tx — durability is never falsely
//!   claimed. A leader killed by SIGKILL (no unwind) is outside this
//!   guarantee; nothing in-process can notice that.
//! - A closed epoch's result stays readable by generation until every
//!   waiter that joined it has picked the result up, so a joined waiter
//!   always learns its outcome even if the leader closes the epoch first
//!   (#5679 lost-wakeup fix).
//! - Waiters have **no timeout and no takeover**: liveness rests entirely
//!   on the leader always closing the epoch — either by recording its
//!   fsync result or by dying and letting the guard close it. An earlier
//!   revision of this text promised that "a caller that times out becomes
//!   its own leader"; that code path never existed and the promise was
//!   false (#5686).
//!
//! ## When NOT to use
//!
//! - Single-threaded workloads: every becomes effectively zero-coalesce,
//!   so group commit adds Condvar overhead with no benefit. We default
//!   to `WalSyncMode::Every` for that reason.
//! - Recovery-sensitive workloads: each batch can lose up to
//!   `max_batch - 1` transactions on a crash. For stricter durability
//!   use `Every`.

use std::collections::HashMap;
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
    /// Generation id, assigned by the coordinator when the epoch is created.
    /// Joiners capture it at join time and use it to find their result even
    /// after the leader has closed the epoch (#5679 lost-wakeup fix).
    gen: u64,
    /// LSNs that are waiting to be made durable.
    waiters: Vec<u64>,
    /// Result of the leader's fsync. `Pending` until the leader records the
    /// outcome.
    result: EpochResult,
    /// Joined waiters that have not yet picked up `result`. The leader
    /// records this at close time: every thread that pushed its LSN into
    /// `waiters` has an entitlement to exactly one read, and the closed
    /// slot is only reclaimed once all entitlements are consumed.
    outstanding: usize,
}

impl Epoch {
    fn new(first_lsn: u64, gen: u64) -> Self {
        Self {
            gen,
            waiters: vec![first_lsn],
            result: EpochResult::Pending,
            outstanding: 0,
        }
    }
}

/// Coordinator state. `Mutex` protects all fields; `Condvar` wakes waiters
/// when the leader finishes.
struct CoordinatorState {
    /// The current epoch. `None` means no epoch in progress.
    epoch: Option<Epoch>,
    /// Closed epochs still owed reads by joined waiters, keyed by generation.
    /// A closed slot stays here until every waiter that joined that epoch has
    /// picked up the result — the fix for the lost-wakeup race where the
    /// leader cleared the epoch before the waiter could read (#5679).
    closed: HashMap<u64, Epoch>,
    /// Monotonic epoch generation counter.
    gen: u64,
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

/// WALGC-2 (#5686) guard: installed on the leader path right after the
/// epoch is registered (see `commit_lsn`). If the leader thread ends
/// without having closed its epoch — the documented case is a panic
/// unwinding through the fsync window, but any unwind counts — `Drop`
/// closes the epoch with an explicit `Err`, parks it in `closed` for the
/// joined waiters (reusing the #5679 generation/outstanding protocol), and
/// wakes everyone. This is the Option-C ruling on #5686: no takeover and no
/// timeout, but a dead leader can never leave waiters stranded, and the
/// epoch slot is always freed so the next caller starts a fresh epoch.
///
/// Durability stance: LSNs queued into a dead epoch were appended but never
/// confirmed durable; every caller in that batch is told its commit failed,
/// so durability is never falsely claimed.
struct LeaderGuard<'a, W: WalManager + 'static> {
    coord: &'a GroupCommitCoordinator<W>,
    gen: u64,
    /// `Cell` rather than a plain bool: `Drop` reading it is invisible to
    /// the `unused_assignments` lint, and the guard never crosses threads.
    closed_by_leader: std::cell::Cell<bool>,
}

impl<W: WalManager + 'static> Drop for LeaderGuard<'_, W> {
    fn drop(&mut self) {
        if self.closed_by_leader.get() {
            return;
        }
        let mut state = self.coord.lock_state();
        if let Some(mut epoch) = state.epoch.take() {
            if epoch.gen == self.gen && matches!(epoch.result, EpochResult::Pending) {
                epoch.result = EpochResult::Err(format!(
                    "WAL group commit leader died before recording the fsync outcome \
                     (WALGC-2 #5686); LSN batch {:?} is NOT known-durable — abort the tx",
                    epoch.waiters
                ));
                // Park the closed epoch for the waiters, same protocol as
                // the normal phase-4 close (#5679).
                epoch.outstanding = epoch.waiters.len().saturating_sub(1);
                if epoch.outstanding > 0 {
                    state.closed.insert(epoch.gen, epoch);
                }
                state.started_at = None;
            } else {
                // Foreign/newer epoch, or the leader already recorded its
                // result: leave the state exactly as found.
                state.epoch = Some(epoch);
            }
        }
        self.coord.cv.notify_all();
    }
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
                closed: HashMap::new(),
                gen: 0,
                started_at: None,
            }),
            cv: Condvar::new(),
            max_batch,
            max_wait_us,
        }
    }

    /// Build a coordinator around an already-shared `Arc<Mutex<W>>`.
    ///
    /// #4913 / v4.1.0-perf: `new` / `with_limits` take ownership of `W`
    /// and allocate their own `Arc<Mutex<W>>`, which makes it impossible
    /// for a storage engine to share the *same* WAL with the coordinator
    /// (`ParallelWalStorage::new_with_shared_wal` takes the `Arc`
    /// directly, and two independent `BufWriter`s over one WAL file would
    /// interleave corruptly). Use this constructor when the caller
    /// already owns the shared lock.
    pub fn with_shared_inner(inner: Arc<Mutex<W>>, max_batch: usize, max_wait_us: u64) -> Self {
        assert!(max_batch > 0, "max_batch must be > 0");
        assert!(max_wait_us > 0, "max_wait_us must be > 0");
        Self {
            inner,
            state: Mutex::new(CoordinatorState {
                epoch: None,
                closed: HashMap::new(),
                gen: 0,
                started_at: None,
            }),
            cv: Condvar::new(),
            max_batch,
            max_wait_us,
        }
    }

    /// Defaults variant of [`Self::with_shared_inner`] (32 / 1000µs).
    pub fn new_with_shared_inner(inner: Arc<Mutex<W>>) -> Self {
        Self::with_shared_inner(inner, 32, 1_000)
    }

    /// Returns a clone of the wrapped `WalManager` lock, for callers that
    /// need exclusive access (e.g. for `append` or recovery). Note that
    /// group commit only coordinates `sync()`; `append` is still
    /// serialised through the same `Mutex`.
    pub fn inner_lock(&self) -> std::sync::MutexGuard<'_, W> {
        self.inner.lock().expect("wal mutex poisoned")
    }

    /// WALGC-2 (#5686): coordinator-internal state locks are poison-tolerant.
    /// A panic under the state lock can only come from a leader inside its
    /// epoch, and the `LeaderGuard` repairs epoch bookkeeping during that
    /// unwinding — so by the time anyone else locks, the data is consistent
    /// again and the poison flag merely records "a panic happened here".
    /// Refusing to proceed would turn one leader panic into permanent
    /// coordinator death (every later caller would panic in `expect`).
    fn lock_state(&self) -> std::sync::MutexGuard<'_, CoordinatorState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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
        // Phase 1: enqueue and decide leader / waiter. `my_gen` pins the
        // epoch this caller joined; the waiter path reads its result by gen,
        // whether the epoch is still live or already closed (#5679).
        let (is_leader, my_gen) = {
            let mut state = self.lock_state();

            if state.epoch.is_none() {
                // No epoch in progress: become leader.
                state.gen += 1;
                let gen = state.gen;
                state.epoch = Some(Epoch::new(lsn, gen));
                state.started_at = Some(Instant::now());
                (true, gen)
            } else {
                // Add our LSN to the current epoch and become a waiter.
                let epoch = state.epoch.as_mut().expect("epoch set");
                let gen = epoch.gen;
                epoch.waiters.push(lsn);
                // If the epoch is now full, wake the leader so it can
                // proceed without waiting for the time bound.
                if epoch.waiters.len() >= self.max_batch {
                    self.cv.notify_all();
                }
                (false, gen)
            }
        };

        if is_leader {
            // WALGC-2 (#5686): leadership is claimed above, so from here on
            // this thread owns the epoch. If it dies before phase-4 records
            // the outcome — the injected failure is `sync()` panicking, but
            // any panic between here and the close counts — the guard below
            // reaps the epoch during unwinding: joined waiters get an
            // explicit `Err` instead of an untimed `cv.wait` forever, and
            // the epoch slot is freed so the next caller starts a fresh
            // epoch instead of joining a dead one.
            let guard = LeaderGuard {
                coord: self,
                gen: my_gen,
                closed_by_leader: std::cell::Cell::new(false),
            };
            let out = self.leader_loop(lsn);
            guard.closed_by_leader.set(true);
            out
        } else {
            self.wait_for_leader(my_gen)
        }
    }

    /// Leader-side loop: wait for the epoch to fill (size or time),
    /// then fsync exactly once for the whole batch.
    fn leader_loop(&self, _my_lsn: u64) -> SqlResult<()> {
        // Compute the time we should give waiters to join.
        let wait_deadline = {
            let state = self.lock_state();
            state
                .started_at
                .expect("started_at is set when epoch is set")
                + Duration::from_micros(self.max_wait_us)
        };

        // Phase 2: wait until the epoch fills or time is up.
        loop {
            let state = self.lock_state();
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
            // `wait_timeout` re-acquires the lock before returning, so bind the
            // guard and hold it until we loop back — the old code rebound
            // `state` here purely for that purpose, and re-reading it on the
            // next iteration is what actually decides whether to break.
            let (_state, _timeout) = self
                .cv
                .wait_timeout(state, remaining)
                .expect("cv wait failed");
            // Loop back and re-check.
        }

        // Phase 3: perform the actual fsync. We hold NO state lock
        // during the syscall, so other threads can still enqueue into
        // a fresh epoch if they arrive while we sync.
        //
        // WALGC-2 (#5686): the lock is taken poison-tolerantly. A previous
        // leader can die inside `sync()` while holding this very lock, and
        // refusing to proceed would convert one injected panic into
        // permanent coordinator death — every later leader would panic in
        // `expect` before fsyncing anything. The dead leader's batch is
        // already being surfaced as `Err` to all of its callers (the
        // WALGC-2 guard semantics), so re-acquiring exclusive `&mut`
        // access here is the survivable reading of the trait contract.
        let mut wal = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let result = wal.sync();
        drop(wal);

        // Phase 4: record the result in the current epoch and broadcast.
        let result_clone = match &result {
            Ok(()) => EpochResult::Ok,
            Err(e) => EpochResult::Err(format!("WAL group commit sync error: {}", e)),
        };
        {
            let mut state = self.lock_state();
            if let Some(mut epoch) = state.epoch.take() {
                epoch.result = result_clone;
                // Every thread that joined this epoch (all `waiters` except
                // the leader itself) is entitled to exactly one read of the
                // result. Park the closed epoch where the waiter path can
                // still find it by gen; it is reclaimed only once every
                // entitlement is consumed (#5679 lost-wakeup fix).
                epoch.outstanding = epoch.waiters.len().saturating_sub(1);
                if epoch.outstanding > 0 {
                    state.closed.insert(epoch.gen, epoch);
                }
            }
            state.started_at = None;
        }
        self.cv.notify_all();

        // Leader returns its own result.
        match result {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Waiter-side: wait until the leader records the result, then pick the
    /// result up for generation `gen` (captured at join time).
    ///
    /// The result is found either in the live epoch (leader has not closed
    /// yet) or in the `closed` slot (leader closed the epoch while this
    /// waiter was parked on the Condvar or between steps). The closed slot
    /// is only reclaimed once every joined waiter has picked up the result,
    /// so a joined waiter can always find it — this closes the lost-wakeup
    /// race where the leader cleared the epoch before the waiter read
    /// (#5679).
    fn wait_for_leader(&self, gen: u64) -> SqlResult<()> {
        let mut state = self.lock_state();
        loop {
            if let Some(epoch) = state.epoch.as_ref() {
                if epoch.gen == gen {
                    match &epoch.result {
                        EpochResult::Pending => {
                            // Still waiting — sleep on the cv. The
                            // returned guard re-acquires the lock; we
                            // assign to `state` and loop.
                            state = self.cv.wait(state).map_err(|e| {
                                crate::engine::SqlError::ExecutionError(format!("cv wait: {}", e))
                            })?;
                            continue;
                        }
                        EpochResult::Ok => return Ok(()),
                        EpochResult::Err(msg) => {
                            return Err(crate::engine::SqlError::ExecutionError(msg.clone()))
                        }
                    }
                }
                // Live epoch belongs to a newer generation: fall through to
                // the closed-slot lookup for our own gen.
            }
            if let Some(epoch) = state.closed.get_mut(&gen) {
                let recorded = !matches!(epoch.result, EpochResult::Pending);
                let outcome = match &epoch.result {
                    EpochResult::Ok => Ok(()),
                    EpochResult::Err(msg) => {
                        Err(crate::engine::SqlError::ExecutionError(msg.clone()))
                    }
                    EpochResult::Pending => Err(crate::engine::SqlError::ExecutionError(
                        "WAL group commit closed epoch unexpectedly pending".into(),
                    )),
                };
                if recorded {
                    epoch.outstanding = epoch.outstanding.saturating_sub(1);
                }
                let prune = state.closed.get(&gen).is_some_and(|e| e.outstanding == 0);
                if prune {
                    state.closed.remove(&gen);
                }
                return outcome;
            }
            // Defensive fallback — unreachable under the gen/outstanding
            // protocol: every joined waiter is counted in `outstanding`, and
            // a closed slot is reclaimed only after all counted readers have
            // read. Reaching here means state was corrupted beyond the
            // protocol's assumptions; keep the legacy conservative behavior.
            return Err(crate::engine::SqlError::ExecutionError(
                "WAL group commit epoch disappeared before waiter could read result".into(),
            ));
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

    // ---------------------------------------------------------------------
    // WALGC-2 (#5686): leader-death livelock regression tests.
    //
    // The probe that opened #5686 promoted to permanent tests. Before the
    // Drop-guard fix these tests are RED: a leader that dies in phase-3
    // leaves `state.epoch` at `Some/Pending` forever — joined waiters block
    // on an untimed `cv.wait` and every later caller joins the dead epoch
    // instead of starting a new one. Nothing ever notifies them.
    //
    // Every test below bounds its observation with a channel timeout, so a
    // livelock shows up as a test failure, never as a hung test binary.
    // ---------------------------------------------------------------------

    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
    use std::sync::mpsc;
    use std::time::Duration as StdDuration;

    /// WAL whose `sync()` panics — the injected death of a leader inside
    /// the fsync window (real triggers: storage-layer asserts, lock-poison
    /// contagion). `arm_first_panic` mode lets a follow-up commit succeed so
    /// the recovery path can be asserted too.
    struct PanickingWal {
        inner: MemoryWalManager,
        panic_once: bool,
        panicked: AtomicBool,
    }

    impl WalManager for PanickingWal {
        fn append(&mut self, entry: crate::wal::WalEntry) -> SqlResult<()> {
            self.inner.append(entry)
        }
        fn flush(&mut self) -> SqlResult<()> {
            self.inner.flush()
        }
        fn sync(&mut self) -> SqlResult<()> {
            if !self.panic_once || !self.panicked.swap(true, AtomicOrdering::SeqCst) {
                panic!("injected leader panic inside fsync (WALGC-2 regression)");
            }
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

    /// Run `commit_lsn` on a helper thread and report its outcome without
    /// ever letting the harness hang: `Ok(v)` = returned value, `Err(())` =
    /// timed out (the livelock signature).
    fn probe_commit<F>(f: F, within: StdDuration) -> Option<String>
    where
        F: FnOnce() -> String + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            // `catch_unwind` keeps a panicking leader from killing the test
            // binary; the point of the probe is WHERE the call ends up, not
            // which flavour of failure it dies with.
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
                .unwrap_or_else(|_| "PANICKED".to_string());
            let _ = tx.send(out);
        });
        match rx.recv_timeout(within) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }

    /// WALGC-2 core: after the leader dies in fsync, the NEXT caller must
    /// reach a terminal state (return value or panic) in bounded time — it
    /// must not join a dead epoch and hang forever.
    #[test]
    fn walgc2_next_caller_after_leader_death_is_not_hung() {
        let coord = Arc::new(GroupCommitCoordinator::with_limits(
            PanickingWal {
                inner: MemoryWalManager::new(),
                panic_once: true,
                panicked: AtomicBool::new(false),
            },
            8,
            10_000,
        ));
        let c1 = coord.clone();
        let leader = thread::spawn(move || c1.commit_lsn(1));
        let _ = leader.join(); // leader panics in phase-3; epoch left Pending

        let outcome = probe_commit(
            move || {
                let c2 = coord.clone();
                match c2.commit_lsn(2) {
                    Ok(()) => "Ok".to_string(),
                    Err(e) => format!("Err({})", e),
                }
            },
            StdDuration::from_secs(3),
        );
        assert!(
            outcome.is_some(),
            "WALGC-2 livelock: commit_lsn(2) after leader death never returned \
             within 3s — it joined the dead epoch (#5686)"
        );
    }

    /// A waiter that had ALREADY joined the dead epoch must learn its
    /// outcome (an explicit Err) instead of blocking on the untimed
    /// `cv.wait` in `wait_for_leader`.
    #[test]
    fn walgc2_joined_waiter_gets_err_when_leader_dies() {
        // Gate controls when the parked leader is allowed to die, so the
        // waiter is deterministically parked on the condvar when the panic
        // unwinds through the leader.
        let (gate_tx, gate_rx) = mpsc::channel::<()>();
        struct GatedWal {
            inner: MemoryWalManager,
            gate: Mutex<Option<mpsc::Receiver<()>>>,
        }
        impl WalManager for GatedWal {
            fn append(&mut self, e: crate::wal::WalEntry) -> SqlResult<()> {
                self.inner.append(e)
            }
            fn flush(&mut self) -> SqlResult<()> {
                self.inner.flush()
            }
            fn sync(&mut self) -> SqlResult<()> {
                if let Some(rx) = self.gate.lock().unwrap().take() {
                    let _ = rx.recv_timeout(StdDuration::from_secs(5));
                }
                panic!("injected leader death in fsync (WALGC-2 regression)");
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
        let coord = Arc::new(GroupCommitCoordinator::with_limits(
            GatedWal {
                inner: MemoryWalManager::new(),
                gate: Mutex::new(Some(gate_rx)),
            },
            8,
            50_000,
        ));

        let c1 = coord.clone();
        let leader = thread::spawn(move || c1.commit_lsn(1)); // parks inside sync()
        thread::sleep(StdDuration::from_millis(120)); // leader is past phase-2

        let (wtx, wrx) = mpsc::channel();
        let c2 = coord.clone();
        let waiter = thread::spawn(move || {
            let out =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match c2.commit_lsn(2) {
                    Ok(()) => "Ok".to_string(),
                    Err(e) => format!("Err({})", e),
                }))
                .unwrap_or_else(|_| "PANICKED".to_string());
            let _ = wtx.send(out);
        });

        let _ = gate_tx.send(()); // release the leader into its panic
        let _ = leader.join();

        let outcome = wrx
            .recv_timeout(StdDuration::from_secs(3))
            .expect("WALGC-2 livelock: joined waiter never woke after leader death (#5686)");
        let _ = waiter.join();
        assert!(
            outcome.starts_with("Err") || outcome == "PANICKED",
            "waiter must never see Ok from an epoch whose leader died; got {outcome}"
        );
    }

    /// After the dead epoch is reaped, the coordinator must accept a new
    /// epoch: a later commit with a healthy `sync()` returns Ok. Poisoned
    /// locks from the dead leader must not turn one panic into permanent
    /// coordinator death.
    #[test]
    fn walgc2_coordinator_recovers_with_new_epoch_after_leader_death() {
        let coord = Arc::new(GroupCommitCoordinator::with_limits(
            PanickingWal {
                inner: MemoryWalManager::new(),
                panic_once: true,
                panicked: AtomicBool::new(false),
            },
            8,
            10_000,
        ));
        let c1 = coord.clone();
        let leader = thread::spawn(move || c1.commit_lsn(1));
        let _ = leader.join(); // first sync panics; epoch must be reaped

        let outcome = probe_commit(
            move || {
                let c2 = coord.clone();
                match c2.commit_lsn(2) {
                    // panic_once: this leader's sync() succeeds.
                    Ok(()) => "Ok".to_string(),
                    Err(e) => format!("Err({})", e),
                }
            },
            StdDuration::from_secs(3),
        );
        assert_eq!(
            outcome.as_deref(),
            Some("Ok"),
            "after leader death the coordinator must run a fresh epoch to success"
        );
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
            // join() yields JoinHandle<Result<_>>: the outer unwrap is the
            // thread panic check, the inner one propagates commit errors —
            // a swallowed Err here would leave the fsync assertions below
            // asserting on a run where every commit failed.
            h.join().unwrap().unwrap();
        }

        // With coalescing, we expect far fewer than 8 fsyncs. In the
        // best case 1. We allow up to 4 to avoid flakiness on
        // heavily-loaded CI.
        let syncs = counter.load(Ordering::SeqCst);
        assert!(
            (1..=4).contains(&syncs),
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
            h.join().unwrap().unwrap();
        }

        // With max_batch=2 and 4 threads, we expect at least 2 fsyncs
        // (the second batch may also coalesce into the first if timing
        // allows). Allow 1-3 for slack.
        let syncs = counter.load(Ordering::SeqCst);
        assert!(
            (1..=3).contains(&syncs),
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
            h.join().unwrap().unwrap();
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
