//! GMP Graph Writer - Layer 0 knowledge extraction to Graph
//!
//! This crate provides the bridge between Layer 0 knowledge extraction
//! (NER entities, RE relations, classification) and the SQLRustGo Graph store.

pub mod writer;

pub use writer::*;
