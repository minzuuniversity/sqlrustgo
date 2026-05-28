//! Disk-backed spill manager implementation.

use crate::chunk::Chunk;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

/// State of the spill manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpillState {
    /// Manager is open and can accept data.
    Open,
    /// Manager is in spill mode, data is being written to disk.
    Spilling,
    /// Manager is reading back spilled data.
    Reading,
    /// Manager has been closed and resources cleaned up.
    Closed,
}

/// Trait for objects that can spill data to disk when memory limit is exceeded.
pub trait Spillable {
    /// Open the spill manager for reading/writing.
    fn open(&mut self);

    /// Spill a chunk to disk.
    fn spill_chunk(&mut self, chunk: &Chunk) -> std::io::Result<()>;

    /// Get the next chunk from spilled data.
    fn next_chunk(&mut self) -> std::io::Result<Option<Chunk>>;

    /// Close the spill manager and clean up resources.
    fn close(&mut self) -> std::io::Result<()>;

    /// Returns the current memory usage in bytes.
    fn current_memory_usage(&self) -> usize;

    /// Returns the number of spilled chunks.
    fn spilled_count(&self) -> usize;
}

/// Disk-backed spill manager that writes chunks to temporary files.
pub struct DiskSpillManager {
    /// Maximum memory allowed before spilling.
    max_memory_bytes: usize,
    /// Current memory usage.
    current_memory_bytes: usize,
    /// Temporary directory for spill files.
    temp_dir: PathBuf,
    /// Paths to spilled chunk files.
    spilled_files: Vec<PathBuf>,
    /// Current state.
    state: SpillState,
    /// Reader index when reading back spilled data.
    read_index: usize,
}

impl DiskSpillManager {
    /// Create a new DiskSpillManager.
    pub fn new(max_memory_bytes: usize, temp_prefix: &str) -> std::io::Result<Self> {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("{}_{}", temp_prefix, unique_id));
        fs::create_dir_all(&temp_dir)?;
        Ok(Self {
            max_memory_bytes,
            current_memory_bytes: 0,
            temp_dir,
            spilled_files: Vec::new(),
            state: SpillState::Open,
            read_index: 0,
        })
    }

    /// Returns the max memory limit.
    pub fn max_memory_bytes(&self) -> usize {
        self.max_memory_bytes
    }

    /// Check if spilling is needed.
    fn needs_spill(&self, chunk_size: usize) -> bool {
        self.current_memory_bytes + chunk_size > self.max_memory_bytes
    }

    /// Generate a unique spill file path.
    fn next_spill_path(&self) -> PathBuf {
        let id = self.spilled_files.len();
        self.temp_dir.join(format!("spill_{}.json", id))
    }

    /// Estimate chunk size in bytes.
    fn estimate_chunk_size(chunk: &Chunk) -> usize {
        // Rough estimation: rows * columns * 8 bytes per value
        chunk.rows() * chunk.columns() * 8
    }

    /// Write a chunk to disk using JSON serialization.
    fn write_chunk_to_disk(&mut self, chunk: &Chunk) -> std::io::Result<()> {
        let path = self.next_spill_path();
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        // Serialize chunk to JSON
        let json = serde_json::to_string(chunk)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")?; // Separator for reading
        writer.flush()?;

        self.spilled_files.push(path);
        Ok(())
    }

    /// Read a chunk from disk.
    fn read_chunk_from_disk(&mut self) -> std::io::Result<Option<Chunk>> {
        if self.read_index >= self.spilled_files.len() {
            return Ok(None);
        }

        let path = &self.spilled_files[self.read_index];
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read entire file content
        let mut contents = String::new();
        reader.read_to_string(&mut contents)?;

        // Parse JSON (may have trailing newline)
        let contents = contents.trim();
        let chunk: Chunk = serde_json::from_str(contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        self.read_index += 1;
        Ok(Some(chunk))
    }
}

impl Spillable for DiskSpillManager {
    fn open(&mut self) {
        self.state = SpillState::Open;
        self.read_index = 0;
    }

    fn spill_chunk(&mut self, chunk: &Chunk) -> std::io::Result<()> {
        let chunk_size = Self::estimate_chunk_size(chunk);

        if self.state == SpillState::Closed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Spill manager is closed",
            ));
        }

        if self.needs_spill(chunk_size) || self.state == SpillState::Spilling {
            self.state = SpillState::Spilling;
            self.write_chunk_to_disk(chunk)?;
        // Memory is not reduced since data is now on disk
        } else {
            self.current_memory_bytes += chunk_size;
        }

        Ok(())
    }

    fn next_chunk(&mut self) -> std::io::Result<Option<Chunk>> {
        if self.state != SpillState::Spilling && self.state != SpillState::Reading {
            return Ok(None);
        }

        self.state = SpillState::Reading;
        self.read_chunk_from_disk()
    }

    fn close(&mut self) -> std::io::Result<()> {
        // Clean up temp files
        for path in &self.spilled_files {
            let _ = fs::remove_file(path);
        }
        self.spilled_files.clear();

        // Remove temp dir if empty
        if self.temp_dir.exists() {
            let _ = fs::remove_dir(&self.temp_dir);
        }

        self.state = SpillState::Closed;
        self.current_memory_bytes = 0;
        self.read_index = 0;

        Ok(())
    }

    fn current_memory_usage(&self) -> usize {
        self.current_memory_bytes
    }

    fn spilled_count(&self) -> usize {
        self.spilled_files.len()
    }
}

impl Drop for DiskSpillManager {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{Chunk, ColumnVector, PrimitiveType, PrimitiveValue};

    fn create_test_chunk() -> Chunk {
        let col = ColumnVector::from_int64(vec![1, 2, 3]);
        Chunk::new(vec![col])
    }

    #[test]
    fn test_spill_manager_basic() {
        // Use max_memory_bytes=1 so even a small chunk will spill
        let mut manager = DiskSpillManager::new(1, "test_spill").unwrap();
        let chunk = create_test_chunk();

        manager.open();
        assert_eq!(manager.state, SpillState::Open);
        assert_eq!(manager.current_memory_usage(), 0);

        // Spill the chunk (should spill because chunk_size > max_memory_bytes)
        manager.spill_chunk(&chunk).unwrap();
        assert_eq!(manager.spilled_count(), 1);

        // Read back
        let read_chunk = manager.next_chunk().unwrap();
        assert!(read_chunk.is_some());

        manager.close().unwrap();
        assert_eq!(manager.state, SpillState::Closed);
    }

    #[test]
    fn test_spill_manager_under_limit() {
        let chunk = create_test_chunk();
        let mut manager = DiskSpillManager::new(1024 * 1024, "test_no_spill").unwrap();

        manager.open();
        manager.spill_chunk(&chunk).unwrap();

        // Should not spill since chunk is small
        assert_eq!(manager.spilled_count(), 0);
        assert!(manager.current_memory_usage() > 0);

        manager.close().unwrap();
    }
}
