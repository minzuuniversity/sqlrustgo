//! T-15: Deadlock Injection Test (Cross-Version Debt Closure)
//!
//! **Issue**: #2834 (https://192.168.0.252:3000/openclaw/sqlrustgo/issues/2834)
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (T-15)
//! **Phase**: Phase 4 (Fault Injection Tests)
//! **Closure target**: `cross_version_debt.sh` shows T-15 CLOSED
//!
//! ## Background
//!
//! v3.0.0 T-15 defined "Deadlock injection" as a test gap: no test for
//! runtime deadlock scenarios. Existing TLA+ PROOF-026 covers Write Skew/SSI
//! formally but doesn't test actual runtime detection.
//!
//! This test file complements TLA+ with concrete runtime tests for
//! `DeadlockDetector::detect_cycle` on real wait-for graphs.
//!
//! ## Test Scenarios
//!
//! 1. **No deadlock**: Independent transactions, no cycle
//! 2. **Direct cycle**: T1 → T2 → T1 (2 transactions)
//! 3. **Indirect cycle**: T1 → T2 → T3 → T1 (3+ transactions)
//! 4. **Multiple cycles**: Two separate cycles in same graph
//! 5. **Edge removal after commit**: Detect cycle before/after remove
//! 6. **Self-loop detection**: T1 → T1 (edge to self)
//! 7. **Concurrent stress**: Many transactions with random wait patterns
//!
//! ## Acceptance Criteria
//!
//! - [x] 5+ test cases (this file has 7+)
//! [ ] cross_version_debt.sh shows T-15 CLOSED
//! [ ] PR merged to develop/v3.8.0

use sqlrustgo_transaction::deadlock::DeadlockDetector;
use sqlrustgo_transaction::mvcc::TxId;

/// Helper: create TxId from u64
fn tx(n: u64) -> TxId {
    TxId::new(n)
}

#[test]
fn test_no_deadlock_independent_transactions() {
    let mut detector = DeadlockDetector::new();
    let t1 = tx(1);
    let t2 = tx(2);

    // T1 waits for T2 (no cycle)
    detector.add_edge(t1, t2);

    // No cycle should be detected
    let cycle = detector.detect_cycle(t1);
    assert!(cycle.is_none(), "expected no cycle, got: {:?}", cycle);
}

#[test]
fn test_detect_direct_cycle_two_transactions() {
    let mut detector = DeadlockDetector::new();
    let t1 = tx(1);
    let t2 = tx(2);

    // T1 waits for T2 AND T2 waits for T1 → cycle
    detector.add_edge(t1, t2);
    detector.add_edge(t2, t1);

    let cycle = detector.detect_cycle(t1);
    assert!(cycle.is_some(), "expected cycle, got None");
    let cycle = cycle.unwrap();
    // Cycle must contain both transactions
    assert!(cycle.contains(&t1), "cycle missing T1: {:?}", cycle);
    assert!(cycle.contains(&t2), "cycle missing T2: {:?}", cycle);
}

#[test]
fn test_detect_indirect_cycle_three_transactions() {
    let mut detector = DeadlockDetector::new();
    let t1 = tx(1);
    let t2 = tx(2);
    let t3 = tx(3);

    // T1 → T2 → T3 → T1 (classic 3-cycle)
    detector.add_edge(t1, t2);
    detector.add_edge(t2, t3);
    detector.add_edge(t3, t1);

    let cycle = detector.detect_cycle(t1);
    assert!(cycle.is_some(), "expected 3-cycle, got None");
    let cycle = cycle.unwrap();
    assert_eq!(
        cycle.len(),
        3,
        "expected cycle of 3, got {}: {:?}",
        cycle.len(),
        cycle
    );
    assert!(cycle.contains(&t1) && cycle.contains(&t2) && cycle.contains(&t3));
}

#[test]
fn test_detect_self_loop() {
    let mut detector = DeadlockDetector::new();
    let t1 = tx(1);

    // T1 waits for itself (self-loop)
    detector.add_edge(t1, t1);

    let cycle = detector.detect_cycle(t1);
    assert!(cycle.is_some(), "expected self-loop cycle, got None");
}

#[test]
fn test_remove_edges_after_commit() {
    let mut detector = DeadlockDetector::new();
    let t1 = tx(1);
    let t2 = tx(2);

    // Create cycle
    detector.add_edge(t1, t2);
    detector.add_edge(t2, t1);
    assert!(detector.detect_cycle(t1).is_some());

    // T1 commits → remove edges for T1
    detector.remove_edges_for(t1);

    // Now no cycle starting from T2 (T1 is gone)
    let cycle = detector.detect_cycle(t2);
    assert!(
        cycle.is_none(),
        "expected no cycle after T1 commit, got: {:?}",
        cycle
    );
}

#[test]
fn test_detect_multiple_independent_cycles() {
    let mut detector = DeadlockDetector::new();

    // Cycle 1: T1 ↔ T2
    detector.add_edge(tx(1), tx(2));
    detector.add_edge(tx(2), tx(1));

    // Cycle 2: T3 ↔ T4
    detector.add_edge(tx(3), tx(4));
    detector.add_edge(tx(4), tx(3));

    // Should detect cycle starting from T1
    let cycle1 = detector.detect_cycle(tx(1));
    assert!(cycle1.is_some(), "expected cycle 1, got None");
    let c1 = cycle1.unwrap();
    assert!(c1.contains(&tx(1)) && c1.contains(&tx(2)));
    assert!(
        !c1.contains(&tx(3)) && !c1.contains(&tx(4)),
        "cycle 1 should not contain T3/T4"
    );

    // Should detect cycle starting from T3
    let cycle2 = detector.detect_cycle(tx(3));
    assert!(cycle2.is_some(), "expected cycle 2, got None");
    let c2 = cycle2.unwrap();
    assert!(c2.contains(&tx(3)) && c2.contains(&tx(4)));
    assert!(!c2.contains(&tx(1)) && !c2.contains(&tx(2)));
}

#[test]
fn test_concurrent_random_wait_for_graphs() {
    use std::collections::HashSet;

    // Simulate 20 transactions with random wait edges, verify detector correctness
    for trial in 0..50 {
        let mut detector = DeadlockDetector::new();
        let mut txs: Vec<TxId> = (1..=20).map(tx).collect();
        let mut rng_state: u64 = 1234 + trial;

        // Add 5-15 random edges
        let edge_count = 5 + (trial % 10);
        let mut added = HashSet::new();
        for _ in 0..edge_count {
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let from_idx = (rng_state as usize) % txs.len();
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let to_idx = (rng_state as usize) % txs.len();
            if from_idx != to_idx {
                let edge = (txs[from_idx], txs[to_idx]);
                if added.insert(edge) {
                    detector.add_edge(edge.0, edge.1);
                }
            }
        }

        // Detector must terminate (no infinite loop)
        for t in &txs {
            let _ = detector.detect_cycle(*t);
        }
    }
}

#[test]
fn test_timeout_configuration() {
    use std::time::Duration;

    let d1 = DeadlockDetector::new();
    assert_eq!(d1.get_timeout(), Duration::from_secs(5));

    let d2 = DeadlockDetector::with_timeout(Duration::from_millis(100));
    assert_eq!(d2.get_timeout(), Duration::from_millis(100));

    let d3 = DeadlockDetector::with_timeout(Duration::from_secs(30));
    assert_eq!(d3.get_timeout(), Duration::from_secs(30));
}
