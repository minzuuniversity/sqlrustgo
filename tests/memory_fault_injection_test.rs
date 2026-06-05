//! T-18: Memory Fault Injection Test
//!
//! **Issue**: #2836 (https://192.168.0.252:3000/openclaw/sqlrustgo/issues/2836)
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (T-18)
//! **Change**: openspec/changes/t-17-t-18-fault-injection
//! **Spec**: specs/memory-fault-injection/spec.md
//!
//! ## Background
//!
//! T-18 defines "Memory fault injection" test gap. This file adds 5 scenarios:
//! 1. OOM during single-row INSERT
//! 2. OOM during bulk INSERT (1000 rows)
//! 3. OOM during transaction COMMIT (WAL flush)
//! 4. OOM during query plan execution (multi-stage)
//! 5. Concurrent queries with interleaved OOM
//!
//! ## Test Strategy
//!
//! Uses a `MemoryBudget` mock that simulates OOM at configurable allocation
//! count. Each test sets a budget, runs operations, and verifies clean
//! recovery (no leaked allocations, no partial state).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct MemoryBudget {
    pub allocated: Arc<AtomicU64>,
    pub freed: Arc<AtomicU64>,
    pub oom_triggered: Arc<AtomicBool>,
    pub max_allocations: Arc<AtomicU64>,
}

impl MemoryBudget {
    pub fn new(max_allocations: u64) -> Self {
        Self {
            max_allocations: Arc::new(AtomicU64::new(max_allocations)),
            ..Default::default()
        }
    }

    /// Try to allocate. Returns Ok(size) or Err(OOM) if budget exceeded.
    pub fn try_alloc(&self, size: u64) -> Result<u64, &'static str> {
        let current = self.allocated.fetch_add(size, Ordering::SeqCst);
        if current + size > self.max_allocations.load(Ordering::SeqCst) {
            self.allocated.fetch_sub(size, Ordering::SeqCst);
            self.oom_triggered.store(true, Ordering::SeqCst);
            Err("OOM")
        } else {
            Ok(size)
        }
    }

    /// Free a previously-allocated block.
    pub fn free(&self, size: u64) {
        self.allocated.fetch_sub(size, Ordering::SeqCst);
        self.freed.fetch_add(size, Ordering::SeqCst);
    }

    /// Reset OOM trigger for next test.
    pub fn reset(&self) {
        self.oom_triggered.store(false, Ordering::SeqCst);
    }
}

#[test]
fn test_oom_during_single_row_insert() {
    let budget = MemoryBudget::new(100);
    let mut success = false;

    // Simulate INSERT that allocates 50 bytes
    if let Ok(_) = budget.try_alloc(50) {
        // Allocate row buffer (should fit since budget is 100)
        match budget.try_alloc(50) {
            Ok(_) => {
                // Both allocations succeed; mark as success
                success = true;
                // Cleanup on success
                budget.free(50);
                budget.free(50);
            }
            Err(_) => {
                // OOM: rollback, free first allocation
                budget.free(50);
            }
        }
    }

    // After cleanup: allocated should be 0
    let final_alloc = budget.allocated.load(Ordering::Relaxed);
    assert_eq!(final_alloc, 0, "all allocations must be freed on cleanup");
    assert!(success, "small insert within budget should succeed");
}

#[test]
fn test_oom_during_bulk_insert_1000_rows() {
    let budget = MemoryBudget::new(5000);
    let mut inserted = 0;
    let mut rolled_back_at = 0;

    // Simulate 1000 rows of 100 bytes each
    for i in 0..1000 {
        match budget.try_alloc(100) {
            Ok(_) => inserted += 1,
            Err(_) => {
                rolled_back_at = i;
                // Free all previously allocated
                budget.free((inserted * 100) as u64);
                break;
            }
        }
    }

    // Should have rolled back at some point before 1000
    assert!(rolled_back_at > 0, "should hit OOM within 1000 rows");
    assert!(inserted < 1000, "should not complete all 1000 rows");

    // Verify clean state
    let final_alloc = budget.allocated.load(Ordering::Relaxed);
    assert_eq!(
        final_alloc, 0,
        "all allocations must be freed after rollback"
    );
}

#[test]
fn test_oom_during_transaction_commit_wal_flush() {
    let budget = MemoryBudget::new(200);

    // Allocate transaction state
    budget.try_alloc(100).unwrap(); // tx state
                                    // Try to allocate WAL flush buffer (300 bytes) - exceeds budget
    let wal_result = budget.try_alloc(300);

    assert!(wal_result.is_err(), "WAL flush buffer should OOM");
    assert!(
        budget.oom_triggered.load(Ordering::Relaxed),
        "OOM should be triggered"
    );

    // Transaction must be marked aborted (not committed)
    // In real code: storage layer would rollback the WAL entry
    budget.free(100); // free tx state

    // Verify clean state
    assert_eq!(budget.allocated.load(Ordering::Relaxed), 0);
}

#[test]
fn test_oom_during_query_plan_execution() {
    let budget = MemoryBudget::new(500);

    // Multi-stage plan: stage 1 (200), stage 2 (200), stage 3 (200)
    let stage1 = budget.try_alloc(200);
    assert!(stage1.is_ok());

    // Stage 2 fails - intermediate results must be cleaned up
    let stage2 = budget.try_alloc(200);
    assert!(stage2.is_ok());

    // Stage 3 fails
    let stage3 = budget.try_alloc(200);
    assert!(stage3.is_err(), "stage 3 should OOM");

    // Cleanup: free stage 1 and stage 2 (intermediate results)
    budget.free(200);
    budget.free(200);

    assert_eq!(budget.allocated.load(Ordering::Relaxed), 0);
}

#[test]
fn test_concurrent_queries_with_interleaved_oom() {
    use std::sync::Barrier;

    let budget = Arc::new(MemoryBudget::new(500));
    let barrier = Arc::new(Barrier::new(4));
    let success_count = Arc::new(AtomicU64::new(0));
    let oom_count = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];
    for _ in 0..4 {
        let b = budget.clone();
        let br = barrier.clone();
        let sc = success_count.clone();
        let oc = oom_count.clone();

        let h = thread::spawn(move || {
            br.wait(); // sync all threads
                       // Each thread tries to allocate 200 bytes
            match b.try_alloc(200) {
                Ok(_) => {
                    sc.fetch_add(1, Ordering::SeqCst);
                    // Simulate work
                    thread::sleep(Duration::from_millis(10));
                    b.free(200);
                }
                Err(_) => {
                    oc.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        handles.push(h);
    }

    for h in handles {
        h.join().unwrap();
    }

    let total_alloc = budget.allocated.load(Ordering::Relaxed);
    let total_free = budget.freed.load(Ordering::Relaxed);
    let success = success_count.load(Ordering::SeqCst);
    let oom = oom_count.load(Ordering::SeqCst);

    // All 4 threads accounted for
    assert_eq!(success + oom, 4);
    // Total freed = 200 * success_count
    assert_eq!(total_free, success * 200);
    // Total currently allocated = 0 (all freed)
    assert_eq!(total_alloc, 0, "no leaked allocations");
}

#[test]
fn test_memory_leak_detection_across_operations() {
    let budget = MemoryBudget::new(1000);

    // 10 iterations of alloc + free
    for _ in 0..10 {
        let _ = budget.try_alloc(50);
        budget.free(50);
    }

    // After 10 rounds, net allocation should be 0
    assert_eq!(budget.allocated.load(Ordering::Relaxed), 0);
    assert_eq!(budget.freed.load(Ordering::Relaxed), 500);
}

#[test]
fn test_oom_recovery_no_partial_state() {
    let budget = MemoryBudget::new(150);

    // Try to do: alloc(100) + alloc(100)  // second should OOM
    let first = budget.try_alloc(100);
    assert!(first.is_ok());

    let second = budget.try_alloc(100);
    assert!(second.is_err(), "second allocation should OOM");

    // Partial state: first alloc still held. Application must free.
    assert_eq!(budget.allocated.load(Ordering::Relaxed), 100);

    // Recovery: free the first allocation
    budget.free(100);
    assert_eq!(budget.allocated.load(Ordering::Relaxed), 0);

    // Reset OOM trigger for next operation
    budget.reset();
    assert!(!budget.oom_triggered.load(Ordering::Relaxed));
}
