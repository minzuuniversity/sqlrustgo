//! Locking primitives for storage engine
//!
//! Provides gap locking for REPEATABLE-READ isolation level.

pub mod gap_lock_manager;

pub use gap_lock_manager::{
    GapLock, GapLockManager, GapLockType, IsolationLevel,
};
