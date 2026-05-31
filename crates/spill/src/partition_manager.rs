use crate::error::{SpillError, SpillResult};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use tempfile::TempDir;

pub struct PartitionManager {
    spill_dir: TempDir,
    partitions: Vec<PartitionFile>,
}

struct PartitionFile {
    path: PathBuf,
    size_bytes: u64,
}

impl PartitionManager {
    pub fn new() -> SpillResult<Self> {
        let spill_dir = tempfile::tempdir().map_err(SpillError::IoError)?;
        Ok(Self {
            spill_dir,
            partitions: Vec::new(),
        })
    }

    pub fn with_dir(path: PathBuf) -> SpillResult<Self> {
        fs::create_dir_all(&path).map_err(SpillError::IoError)?;
        let spill_dir = TempDir::new_in(&path).map_err(SpillError::IoError)?;
        Ok(Self {
            spill_dir,
            partitions: Vec::new(),
        })
    }

    pub fn write_partition<T: serde::Serialize>(&mut self, data: &[T]) -> SpillResult<usize> {
        let partition_id = self.partitions.len();
        let path = self
            .spill_dir
            .path()
            .join(format!("partition_{}.bin", partition_id));

        let file = File::create(&path).map_err(SpillError::IoError)?;
        let mut writer = BufWriter::new(file);

        let bytes =
            bincode::serialize(data).map_err(|e| SpillError::PartitionError(e.to_string()))?;
        writer.write_all(&bytes).map_err(SpillError::IoError)?;

        writer.flush().map_err(SpillError::IoError)?;

        let size_bytes = fs::metadata(&path).map_err(SpillError::IoError)?.len();

        let partition = PartitionFile { path, size_bytes };
        self.partitions.push(partition);

        Ok(partition_id)
    }

    pub fn read_partition<T: serde::de::DeserializeOwned>(
        &self,
        partition_id: usize,
    ) -> SpillResult<Vec<T>> {
        let partition = self.partitions.get(partition_id).ok_or_else(|| {
            SpillError::PartitionError(format!("Invalid partition {}", partition_id))
        })?;

        let bytes = fs::read(&partition.path).map_err(SpillError::IoError)?;

        let items: Vec<T> =
            bincode::deserialize(&bytes).map_err(|e| SpillError::PartitionError(e.to_string()))?;

        Ok(items)
    }

    pub fn num_partitions(&self) -> usize {
        self.partitions.len()
    }

    pub fn total_bytes_spilled(&self) -> u64 {
        self.partitions.iter().map(|p| p.size_bytes).sum()
    }

    pub fn cleanup(&mut self) {
        for partition in &self.partitions {
            let _ = fs::remove_file(&partition.path);
        }
        self.partitions.clear();
    }
}

impl Default for PartitionManager {
    fn default() -> Self {
        Self::new().expect("Failed to create tempdir")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_write_and_read() {
        let mut pm = PartitionManager::new().unwrap();
        let data = vec![1, 2, 3, 4, 5];
        let id = pm.write_partition(&data).unwrap();
        assert_eq!(id, 0);
        assert_eq!(pm.num_partitions(), 1);

        let read_back: Vec<i32> = pm.read_partition(id).unwrap();
        assert_eq!(read_back, data);
    }

    #[test]
    fn test_multiple_partitions() {
        let mut pm = PartitionManager::new().unwrap();
        let id1 = pm.write_partition(&vec!["a", "b"]).unwrap();
        let id2 = pm.write_partition(&vec!["c", "d", "e"]).unwrap();
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(pm.num_partitions(), 2);
        assert!(pm.total_bytes_spilled() > 0);
    }

    #[test]
    fn test_invalid_partition_id() {
        let pm = PartitionManager::new().unwrap();
        let result: SpillResult<Vec<i32>> = pm.read_partition(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_cleanup_clears_partitions() {
        let mut pm = PartitionManager::new().unwrap();
        pm.write_partition(&vec![1, 2, 3]).unwrap();
        assert_eq!(pm.num_partitions(), 1);
        pm.cleanup();
        assert_eq!(pm.num_partitions(), 0);
        assert_eq!(pm.total_bytes_spilled(), 0);
    }

    #[test]
    fn test_partition_with_struct() {
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Row {
            id: i32,
            name: String,
            score: f64,
        }

        let mut pm = PartitionManager::new().unwrap();
        let rows = vec![
            Row { id: 1, name: "Alice".into(), score: 95.5 },
            Row { id: 2, name: "Bob".into(), score: 87.0 },
        ];
        let id = pm.write_partition(&rows).unwrap();
        let read_back: Vec<Row> = pm.read_partition(id).unwrap();
        assert_eq!(read_back, rows);
    }
}
