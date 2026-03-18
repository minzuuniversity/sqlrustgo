use crate::mvcc::TxId;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub struct DeadlockDetector {
    waits_for: HashMap<TxId, HashSet<TxId>>,
    #[allow(dead_code)]
    lock_wait_timeout: Duration, // Reserved for future timeout functionality
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self {
            waits_for: HashMap::new(),
            lock_wait_timeout: Duration::from_secs(5),
        }
    }

    pub fn add_edge(&mut self, blocked: TxId, holder: TxId) {
        self.waits_for.entry(blocked).or_default().insert(holder);
    }

    pub fn remove_edges_for(&mut self, tx_id: TxId) {
        self.waits_for.remove(&tx_id);
        for holders in self.waits_for.values_mut() {
            holders.remove(&tx_id);
        }
    }
}

impl Default for DeadlockDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deadlock_detector_new() {
        let detector = DeadlockDetector::new();
        assert!(detector.waits_for.is_empty());
    }

    #[test]
    fn test_add_edge() {
        let mut detector = DeadlockDetector::new();
        detector.add_edge(TxId::new(1), TxId::new(2));
        assert!(detector
            .waits_for
            .get(&TxId::new(1))
            .unwrap()
            .contains(&TxId::new(2)));
    }
}
