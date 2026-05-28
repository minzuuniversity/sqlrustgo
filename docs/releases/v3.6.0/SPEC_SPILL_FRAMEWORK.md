# Spill Framework Specification

## Issue
#1464: Architecture: Spill Framework

## Summary
The Spill Framework provides memory management for query execution by spilling large datasets to disk when memory limits are exceeded. This is essential for hash joins and other memory-intensive operators.

## Architecture

```
Spillable (trait)
    │
    ├── open() -> SpillState
    ├── spill_chunk(chunk: &Chunk) -> Result<()>
    ├── next_chunk() -> Result<Option<Chunk>>
    └── close() -> Result<()>

DiskSpillManager (implementation)
    ├── max_memory_bytes: usize
    ├── current_memory_bytes: usize
    ├── spilled_chunks: Vec<PathBuf>
    └── temp_dir: PathBuf

PartitionedSpill (for hash join)
    ├── partitions: Vec<DiskSpillManager>
    └── num_partitions: usize
```

## API

### Spillable Trait

```rust
pub trait Spillable {
    fn open() -> Self;
    fn spill_chunk(&mut self, chunk: &Chunk) -> Result<()>;
    fn next_chunk(&mut self) -> Result<Option<Chunk>>;
    fn close(&mut self) -> Result<()>;
}
```

### DiskSpillManager

- `new(max_memory_bytes: usize, temp_prefix: &str) -> Self`
- `current_memory_usage() -> usize`
- `spilled_file_count() -> usize`

### PartitionedSpill

- `new(num_partitions: usize, max_memory_bytes: usize) -> Self`
- `spill_to_partition(&mut self, partition_id: usize, chunk: &Chunk) -> Result<()>`
- `next_chunk_for_partition(&mut self, partition_id: usize) -> Result<Option<Chunk>>`

## Memory Management

1. When `current_memory_bytes + chunk_size > max_memory_bytes`, the chunk is spilled to disk
2. Spilled chunks are stored in temporary files using `std::fs`
3. Temporary files are cleaned up on `close()`

## Files

```
crates/executor/src/spill/
├── mod.rs        # SpillManager trait, Spillable trait, DiskSpillManager struct
├── manager.rs    # DiskSpillManager implementation
└── partition.rs  # PartitionedSpill for hash join
```

## Dependencies

- `std::fs` for file I/O
- `std::path` for temp file paths
- `serde` for serialization (Chunk data)
