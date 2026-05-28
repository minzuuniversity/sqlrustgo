//! Chunk module: columnar data structures for pipeline execution.
//!
//! # Architecture
//!
//! ```text
//! Chunk
//! ├── ColumnVector
//! │   ├── ColumnData (storage)
//! │   └── NullBitmap
//! ├── types (PrimitiveType, PrimitiveValue)
//! └── SelectionVector
//! ```
//!
//! # Data Flow
//!
//! 1. Storage layer produces `Row` tuples
//! 2. `RowToChunkAdapter` converts Rows → Chunk
//! 3. `Chunk` is passed through pipeline operators
//! 4. Operators produce new Chunks (or consume them)

pub mod chunk;
pub mod column_vector;
pub mod selection_vector;
pub mod types;

// Re-exports for convenience
pub use chunk::Chunk;
pub use column_vector::{ColumnData, ColumnVector, NullBitmap};
pub use selection_vector::SelectionVector;
pub use types::{PrimitiveType, PrimitiveValue};
