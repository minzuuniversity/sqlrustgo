//! Exchange module: data distribution interfaces for distributed execution.
//!
//! This module provides the ExchangeOperator trait and implementations for
//! different exchange patterns (Local, Broadcast, Shuffle).

pub mod broadcast;
pub mod local;
pub mod operator;

// Re-exports
pub use operator::{ExchangeOperator, ExchangeType, Partitioning};
