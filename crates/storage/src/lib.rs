// SQLRustGo storage module

pub mod adaptive_hash_index;
pub mod backup;
pub mod binary_format;
pub mod binary_storage;
pub mod bplus_tree;
pub mod buffer_pool;
pub mod change_buffer;
pub mod checkpoint;
pub mod clustered_table;
pub mod double_write_buffer;
pub mod engine;
pub mod file_storage;
pub mod io_delay;

// Re-export for integration tests that import via sqlrustgo_storage::
pub use io_delay::{IoDelayConfig, IoFaultInjector};
pub mod lock;
pub mod page;
pub mod predicate;
pub mod read_write_split;
pub mod recovery_engine;
pub mod restore_filespace;
pub mod vtu_guard;
pub mod vtu_ir;
pub mod wal;
pub mod wal_legacy;
pub mod wal_storage;

pub use adaptive_hash_index::{
    AdaptiveHashIndex, IndexKey, PageLocation, DEFAULT_PROMOTION_THRESHOLD,
};
pub use binary_format::BinaryFormat;
pub use binary_storage::{BinaryTableStorage, BoxStorageEngine};
pub use bplus_tree::BPlusTree;
pub use buffer_pool::BufferPool;
pub use change_buffer::{ChangeBuffer, ChangeEntry, ChangeOp};
pub use checkpoint::{CheckpointConfig, CheckpointManager, CheckpointMetadata};
pub use double_write_buffer::{DoubleWriteBuffer, DwbPage};
pub use engine::{
    evaluate_check_constraint, ColumnDefinition, ForeignKeyAction, ForeignKeyConstraint,
    MemoryStorage, Record, RowFilter, RowMutation, SequenceInfo, SqlResult, StorageEngine, TableData,
    TableInfo, TriggerEvent, TriggerInfo, TriggerTiming, UniqueConstraint, Value,
};
pub use file_storage::FileStorage;
pub use lock::{GapLock, GapLockManager, GapLockType, IsolationLevel};
pub use page::Page;
pub use wal::{FileBackedWalManager, MemoryWalManager, WalManager};
pub use wal_storage::{WalStorage, WalSyncMode};
