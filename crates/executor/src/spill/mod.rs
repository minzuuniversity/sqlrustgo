//! Spill Framework: Memory management by spilling large datasets to disk.
//!
//! # Architecture
//!
//! ```text
//! Spillable (trait)
//!     ├── open() -> SpillState
//!     ├── spill_chunk(chunk: &Chunk) -> Result<()>
//!     ├── next_chunk() -> Result<Option<Chunk>>
//!     └── close() -> Result<()>
//!
//! DiskSpillManager (implementation)
//!     ├── max_memory_bytes: usize
//!     ├── current_memory_bytes: usize
//!     ├── spilled_chunks: Vec<PathBuf>
//!     └── temp_dir: PathBuf
//! ```

pub mod manager;
pub mod partition;

pub use manager::{DiskSpillManager, SpillState, Spillable};
pub use partition::PartitionedSpill;
