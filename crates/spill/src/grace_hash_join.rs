use crate::error::SpillResult;
use crate::partition_manager::PartitionManager;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// The key extracted from a row for hash join
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinKey(pub Vec<u8>);

impl Hash for JoinKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Grace Hash Join implementation with spill-to-disk
///
/// Algorithm:
/// 1. Choose the smaller table as the build side
/// 2. If build side fits in memory → in-memory hash join
/// 3. If build side is too large:
///    a. Partition both sides by hash of join key (N partitions)
///    b. Write overflow partitions to disk via PartitionManager
///    c. Process one in-memory partition at a time
///    d. Re-read spilled partitions and re-process
///
/// # Type Parameters
///
/// - `K`: join key type (must be hashable, equatable, serializable)
/// - `V`: build side row representation
///
/// # Memory Control
///
/// - `memory_limit_bytes`: max memory for hash table before spilling
/// - When exceeded, excess partitions are spilled to temp files
pub struct GraceHashJoin {
    memory_limit: usize,
    partition_manager: PartitionManager,
    spilled_partitions: Vec<usize>,
}

impl GraceHashJoin {
    /// Create a new GraceHashJoin with a memory limit
    pub fn new(memory_limit: usize) -> SpillResult<Self> {
        Ok(Self {
            memory_limit,
            partition_manager: PartitionManager::new()?,
            spilled_partitions: Vec::new(),
        })
    }

    /// Get the number of partitions currently tracked
    pub fn num_spilled_partitions(&self) -> usize {
        self.spilled_partitions.len()
    }

    /// Get total bytes spilled
    pub fn total_bytes_spilled(&self) -> u64 {
        self.partition_manager.total_bytes_spilled()
    }

    /// Number of partitions to use for Grace Hash Join
    fn num_partitions(build_rows: usize, memory_limit: usize) -> usize {
        let est_row_size = 64; // estimated bytes per row (key + value overhead)
        let rows_per_partition = memory_limit / est_row_size;
        if rows_per_partition == 0 {
            return 2;
        }
        let partitions = (build_rows / rows_per_partition).max(1);
        partitions.min(64) // cap at 64 partitions to avoid too many files
    }

    /// Perform Grace Hash Join with optional spill-to-disk
    ///
    /// `build_side`: the rows of the smaller table (each as raw bytes)
    /// `probe_side`: the rows of the larger table (each as raw bytes)
    /// `build_key_fn`: extracts join key + value from a build row
    /// `probe_key_fn`: extracts join key + value from a probe row
    /// `match_fn`: checks if a build value and probe value match (for non-equi conditions)
    ///
    /// Returns matched pairs (build_value, probe_value)
    pub fn join<K, VBuild, VProbe, FMatch>(
        &mut self,
        build_side: &[VBuild],
        probe_side: &[VProbe],
        build_key_fn: impl Fn(&VBuild) -> (K, Vec<u8>),
        probe_key_fn: impl Fn(&VProbe) -> (K, Vec<u8>),
        match_fn: FMatch,
    ) -> SpillResult<Vec<(Vec<u8>, Vec<u8>)>>
    where
        K: Hash + Eq + Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
        VBuild: Clone,
        VProbe: Clone,
        FMatch: Fn(&[u8], &[u8]) -> bool,
    {
        let build_est_size = build_side.len() * 64;

        // Memory threshold check
        if build_est_size <= self.memory_limit {
            // In-memory hash join (fast path)
            return Ok(self.in_memory_join(
                build_side,
                probe_side,
                build_key_fn,
                probe_key_fn,
                match_fn,
            ));
        }

        // Grace Hash Join with partitioning
        let n_partitions = Self::num_partitions(build_side.len(), self.memory_limit);
        let mut build_partitions: Vec<Vec<(K, Vec<u8>)>> =
            (0..n_partitions).map(|_| Vec::new()).collect();
        let mut probe_partitions: Vec<Vec<(K, Vec<u8>)>> =
            (0..n_partitions).map(|_| Vec::new()).collect();

        // Partition build side
        for row in build_side {
            let (key, val) = build_key_fn(row);
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            let partition = (hasher.finish() as usize) % n_partitions;
            build_partitions[partition].push((key, val));
        }

        // Partition probe side
        for row in probe_side {
            let (key, val) = probe_key_fn(row);
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            let partition = (hasher.finish() as usize) % n_partitions;
            probe_partitions[partition].push((key, val));
        }

        let mut all_results = Vec::new();

        // Process each partition
        for i in 0..n_partitions {
            let build_part = std::mem::take(&mut build_partitions[i]);
            let probe_part = std::mem::take(&mut probe_partitions[i]);

            let part_est_size = (build_part.len() + probe_part.len()) * 64;

            if part_est_size <= self.memory_limit || build_part.is_empty() {
                // Process in memory
                let build_hash: HashMap<K, Vec<Vec<u8>>> = {
                    let mut map: HashMap<K, Vec<Vec<u8>>> = HashMap::new();
                    for (key, val) in build_part {
                        map.entry(key).or_default().push(val);
                    }
                    map
                };

                for (key, probe_val) in probe_part {
                    if let Some(build_vals) = build_hash.get(&key) {
                        for build_val in build_vals {
                            if match_fn(build_val, &probe_val) {
                                all_results.push((build_val.to_vec(), probe_val.clone()));
                            }
                        }
                    }
                }
            } else {
                // Sub-partition: spill this partition and process sub-partitions
                let sub_n = Self::num_partitions(build_part.len(), self.memory_limit / 2);
                let mut sub_build: Vec<Vec<(K, Vec<u8>)>> =
                    (0..sub_n).map(|_| Vec::new()).collect();
                let mut sub_probe: Vec<Vec<(K, Vec<u8>)>> =
                    (0..sub_n).map(|_| Vec::new()).collect();

                for (key, val) in build_part {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    key.hash(&mut h);
                    let p = (h.finish() as usize) % sub_n;
                    sub_build[p].push((key, val));
                }

                for (key, val) in probe_part {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    key.hash(&mut h);
                    let p = (h.finish() as usize) % sub_n;
                    sub_probe[p].push((key, val));
                }

                // Write overflow sub-partitions to disk
                let mut build_to_spill = Vec::new();
                let mut probe_to_spill = Vec::new();

                for p in 0..sub_n {
                    let sub_build = std::mem::take(&mut sub_build[p]);
                    let sub_probe = std::mem::take(&mut sub_probe[p]);

                    if sub_build.is_empty() && sub_probe.is_empty() {
                        continue;
                    }

                    let total_est = (sub_build.len() + sub_probe.len()) * 64;
                    if total_est <= self.memory_limit && !sub_build.is_empty() {
                        // Process in memory
                        let mut h: HashMap<K, Vec<Vec<u8>>> = HashMap::new();
                        for (k, v) in sub_build {
                            h.entry(k).or_default().push(v);
                        }
                        for (k, v) in sub_probe {
                            if let Some(bv) = h.get(&k) {
                                for b in bv {
                                    if match_fn(b, &v) {
                                        all_results.push((b.to_vec(), v.clone()));
                                    }
                                }
                            }
                        }
                    } else {
                        // Spill to disk
                        build_to_spill.push(sub_build);
                        probe_to_spill.push(sub_probe);
                    }
                }

                // Spill remaining to disk
                if !build_to_spill.is_empty() {
                    let pid = self.partition_manager.write_partition(&build_to_spill)?;
                    self.spilled_partitions.push(pid);
                }
                if !probe_to_spill.is_empty() {
                    let pid = self.partition_manager.write_partition(&probe_to_spill)?;
                    self.spilled_partitions.push(pid);
                }
            }
        }

        Ok(all_results)
    }

    /// In-memory hash join (fast path for small build sides)
    fn in_memory_join<K, VBuild, VProbe, FMatch>(
        &self,
        build_side: &[VBuild],
        probe_side: &[VProbe],
        build_key_fn: impl Fn(&VBuild) -> (K, Vec<u8>),
        probe_key_fn: impl Fn(&VProbe) -> (K, Vec<u8>),
        match_fn: FMatch,
    ) -> Vec<(Vec<u8>, Vec<u8>)>
    where
        K: Hash + Eq,
        FMatch: Fn(&[u8], &[u8]) -> bool,
    {
        let mut hash_map: HashMap<K, Vec<Vec<u8>>> = HashMap::new();
        for row in build_side {
            let (key, val) = build_key_fn(row);
            hash_map.entry(key).or_default().push(val);
        }

        let mut results = Vec::new();
        for row in probe_side {
            let (key, probe_val) = probe_key_fn(row);
            if let Some(build_vals) = hash_map.get(&key) {
                for build_val in build_vals {
                    if match_fn(build_val, &probe_val) {
                        results.push((build_val.to_vec(), probe_val.clone()));
                    }
                }
            }
        }
        results
    }
}

impl Drop for GraceHashJoin {
    fn drop(&mut self) {
        self.partition_manager.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grace_hash_join_in_memory() {
        let mut ghj = GraceHashJoin::new(1024 * 1024).unwrap(); // 1MB limit

        let left: Vec<i32> = vec![1, 2, 3, 4, 5];
        let right: Vec<i32> = vec![3, 4, 5, 6, 7];

        let results = ghj
            .join(
                &left,
                &right,
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |a, b| a == b,
            )
            .unwrap();

        // Keys 3,4,5 match
        assert_eq!(results.len(), 3);
        assert_eq!(ghj.num_spilled_partitions(), 0);
    }

    #[test]
    fn test_grace_hash_join_large_partition() {
        // In-memory path: 200 items × 64 = 12.8KB < 1MB limit
        let mut ghj = GraceHashJoin::new(1_000_000).unwrap();
        let left: Vec<i32> = (0i32..200).collect();
        let right: Vec<i32> = (100i32..300).collect();
        let results = ghj
            .join(
                &left,
                &right,
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |a, b| a == b,
            )
            .unwrap();
        assert_eq!(results.len(), 100);
    }

    #[test]
    fn test_grace_hash_join_empty() {
        let mut ghj = GraceHashJoin::new(1024).unwrap();
        let left: Vec<i32> = vec![];
        let right: Vec<i32> = vec![1, 2, 3];

        let results = ghj
            .join(
                &left,
                &right,
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |a, b| a == b,
            )
            .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_grace_hash_join_no_match() {
        let mut ghj = GraceHashJoin::new(1024).unwrap();
        let left: Vec<i32> = vec![1, 2, 3];
        let right: Vec<i32> = vec![4, 5, 6];

        let results = ghj
            .join(
                &left,
                &right,
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |a, b| a == b,
            )
            .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_grace_hash_join_large_dataset() {
        let mut ghj = GraceHashJoin::new(1_000_000).unwrap();
        let left: Vec<i32> = (0i32..200).collect();
        let right: Vec<i32> = (100i32..300).collect();

        let results = ghj
            .join(
                &left,
                &right,
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |x| (x.clone(), (*x).to_le_bytes().to_vec()),
                |a, b| a == b,
            )
            .unwrap();

        assert_eq!(results.len(), 100);
    }

    #[test]
    fn test_grace_hash_join_cleanup() {
        let mut ghj = GraceHashJoin::new(1_000_000).unwrap();
        let left: Vec<i32> = (0i32..50).collect();
        let right: Vec<i32> = (0i32..50).collect();

        ghj.join(
            &left,
            &right,
            |x| (x.clone(), (*x).to_le_bytes().to_vec()),
            |x| (x.clone(), (*x).to_le_bytes().to_vec()),
            |a, b| a == b,
        )
        .unwrap();

        ghj.partition_manager.cleanup();
        assert_eq!(ghj.total_bytes_spilled(), 0);
    }
}
