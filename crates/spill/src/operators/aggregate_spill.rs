use crate::error::SpillResult;
use crate::memory_tracker::AdaptiveMemoryTracker;
use crate::partition_manager::PartitionManager;
use crate::r#trait::SpillingIterator;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregatedState {
    pub count: usize,
    pub sum: f64,
}

pub struct AggregateSpillOperator<
    G: Clone + std::hash::Hash + Eq + serde::Serialize + serde::de::DeserializeOwned,
> {
    tracker: Arc<AdaptiveMemoryTracker>,
    partition_manager: PartitionManager,
    groups: HashMap<G, AggregatedState>,
    spilled_groups: Vec<(G, AggregatedState)>,
}

impl<G: Clone + std::hash::Hash + Eq + serde::Serialize + serde::de::DeserializeOwned>
    AggregateSpillOperator<G>
{
    pub fn new(tracker: Arc<AdaptiveMemoryTracker>) -> SpillResult<Self> {
        Ok(Self {
            tracker,
            partition_manager: PartitionManager::new()?,
            groups: HashMap::new(),
            spilled_groups: Vec::new(),
        })
    }

    pub fn aggregate(&mut self, group_key: G, value: f64) -> SpillResult<()> {
        if self.tracker.should_spill() {
            self.spill_groups()?;
        }

        let state = self
            .groups
            .entry(group_key)
            .or_insert(AggregatedState { count: 0, sum: 0.0 });
        state.count += 1;
        state.sum += value;

        let _ = self.tracker.allocate(std::mem::size_of::<G>() as u64);
        Ok(())
    }

    fn spill_groups(&mut self) -> SpillResult<()> {
        for (key, state) in self.groups.drain() {
            self.spilled_groups.push((key, state));
        }
        let _partition_id = self
            .partition_manager
            .write_partition(&self.spilled_groups)?;
        self.spilled_groups.clear();
        Ok(())
    }

    pub fn get_groups(&self) -> &HashMap<G, AggregatedState> {
        &self.groups
    }
}

impl<G: Clone + std::hash::Hash + Eq + serde::Serialize + serde::de::DeserializeOwned>
    SpillingIterator for AggregateSpillOperator<G>
{
    fn start_spill(&mut self) -> SpillResult<()> {
        self.spill_groups()
    }

    fn is_spilling(&self) -> bool {
        self.partition_manager.num_partitions() > 0
    }

    fn num_partitions(&self) -> usize {
        self.partition_manager.num_partitions()
    }

    fn finish_spill(&mut self) {
        self.groups.clear();
        self.spilled_groups.clear();
        self.partition_manager.cleanup();
    }
}

impl<G: Clone + std::hash::Hash + Eq + serde::Serialize + serde::de::DeserializeOwned> Iterator
    for AggregateSpillOperator<G>
{
    type Item = (G, AggregatedState);

    fn next(&mut self) -> Option<Self::Item> {
        self.spilled_groups.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_aggregate_basic() {
        let tracker = Arc::new(AdaptiveMemoryTracker::new(1024, 512));
        let mut op = AggregateSpillOperator::<String>::new(tracker.clone()).unwrap();

        op.aggregate("a".to_string(), 10.0).unwrap();
        op.aggregate("b".to_string(), 20.0).unwrap();
        op.aggregate("a".to_string(), 5.0).unwrap();

        let groups = op.get_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get("a").unwrap().count, 2);
        assert_eq!(groups.get("a").unwrap().sum, 15.0);
        assert_eq!(groups.get("b").unwrap().count, 1);
        assert_eq!(groups.get("b").unwrap().sum, 20.0);
    }

    #[test]
    fn test_aggregate_spill_triggers() {
        let tracker = Arc::new(AdaptiveMemoryTracker::new(32, 8));
        let mut op = AggregateSpillOperator::<String>::new(tracker.clone()).unwrap();

        for i in 0..20 {
            op.aggregate(format!("key_{}", i % 5), i as f64).unwrap();
        }

        assert!(op.is_spilling());
    }

    #[test]
    fn test_aggregate_finish_spill() {
        let tracker = Arc::new(AdaptiveMemoryTracker::new(32, 8));
        let mut op = AggregateSpillOperator::<i32>::new(tracker.clone()).unwrap();

        for i in 0..10 {
            op.aggregate(i, i as f64).unwrap();
        }
        assert!(op.is_spilling());

        op.finish_spill();
        assert!(!op.is_spilling());
        assert_eq!(op.get_groups().len(), 0);
    }
}
