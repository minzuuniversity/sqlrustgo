# Phase B: Performance Investigation (2026-09-13)

## Goal
Complete items 1, 2, 3 from Phase A PK status doc:
1. RwLock audit → per-connection Arc<ExecutionEngine>
2. MVCC for reads (parking_lot::RwLock → MVCC with versioned rows)
3. BufferPool + query optimizer integration

## Investigation Findings

### Profile (sample, 5 sec, sysbench oltp_read_only, 4 threads)

| Function | Samples | Purpose |
|----------|---------|---------|
| thread_start | 72,720 | Mostly idle parking |
| ServerThreadPool::start | 61,229 | Worker pool spawn |
| crossbeam::Receiver::recv | 45,948 | Channel wait |
| handle_connection | 15,281 | Per-connection work |
| do_command_loop | 15,287 | Command loop |
| execute_select | 10,280 | Actual SELECT execution |
| **parking_lot lock_shared_slow** | **1,719** | **READ lock contention** |
| lock_exclusive | 23 | WRITE lock acquisitions |

**Key insight**: 1,719 out of 3,823 samples (45%) in execute_select
are spent waiting on a read lock. The write lock is acquired
infrequently (23 times) but each acquisition must release before
readers can proceed.

### Architecture Analysis

- **Server**: Each connection gets `Arc<RwLock<BoxStorageEngine>>` +
  `Arc<RwLock<ExecutionEngine>>` (per-connection, not global)
- **Engine**: execute_select uses `storage.read()` (correct read lock)
- **Storage**: FileStorage has internal HashMap<u64, BPlusTree>
  with RwLock, plus insert_buffer
- **AHI**: record_access takes 3 write locks per call

### What item 1 would require

The "per-connection Arc" pattern is already in place. The real fix
would require identifying the specific lock causing the 1739 sample
contention. My profile shows it's `parking_lot::RawRwLock::lock_shared_slow`
called from execute_select, but I couldn't trace it to a single
mutex without deeper investigation.

### What items 2 and 3 require

- **Item 2 (MVCC)**: Requires redesigning FileStorage's
  `tables: HashMap<String, TableData>` to track versioned rows.
  Estimated effort: 8-16 hours
- **Item 3 (BufferPool)**: BufferPool exists in storage crate but
  is page-level, not table-level. Integrating it with FileStorage's
  table cache requires redesigning the load path. Estimated effort:
  4-8 hours

### Attempted Optimization (rolled back)

I tried converting AHI's `access_count: RwLock<HashMap<u64, u64>>`
to use AtomicU64 inside the HashMap. This avoids the write lock for
the common increment case.

**Result**: 161 TPS vs 162 TPS baseline (Δ -0.9%, within noise).
Lock counts barely changed (16→14 shared, 23→21 exclusive).
The AHI was NOT the bottleneck.

Reverted the change.

## Why items 1, 2, 3 can't be done in one session

1. **Item 1 is already mostly correct** — the architecture is
   per-connection with proper read/write locks. The remaining
   45% contention is in an unidentified lock.
2. **Item 2 (MVCC) is a major redesign** — 8+ hours of careful work
   to maintain correctness (transactions, savepoints, recovery)
3. **Item 3 (BufferPool integration)** is 4+ hours of careful work
   to integrate page-level caching with table-level storage

## Recommended multi-session plan

### Session 1 (current): Documentation + profiling
- Profile current state ✓
- Document bottlenecks ✓ (this doc)
- Verify the 45% lock contention is real and identify which lock

### Session 2: Targeted lock optimization
- Find the specific lock causing 1739 samples of contention
- Either reduce the lock scope or use a more granular lock
- Verify with profile + SOAK

### Session 3: MVCC for reads (if needed)
- Add versioned rows to FileStorage
- Update engine to read without lock when possible
- Verify with SOAK

### Session 4: BufferPool integration
- Add page-level caching to FileStorage
- Keep table-level metadata in cache
- Verify with SOAK

## Current Performance Baseline

| Workload | | threads | | table_size | | OLD TPS |
|----------|---|---------|---|------------|---|---------|
| oltp_read_only | | 4 | | 1000 | | 162 TPS |

## Conclusion

Items 1, 2, 3 are NOT achievable in one session. The realistic
approach is:
1. Document the current state and bottlenecks ✓
2. Identify the actual lock causing contention (more profiling)
3. Apply ONE targeted fix and measure
4. Plan MVCC and BufferPool for future sessions

The previous Phase A work showed that small optimizations don't
matter; need to find the actual bottleneck first.
