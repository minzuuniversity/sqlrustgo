//! GMP Knowledge Graph Layer
//!
//! Layer 0 of the five-layer architecture: Offline Knowledge Extraction Pipeline
//! Uses SQLRustGo FileStorage as the underlying storage engine.

pub mod storage;
pub mod types;

pub use storage::{KgStats, KgStorage};
pub use types::{DocEntity, DocRelation, DocRelevance, DocTaxonomy, GmpTerm};
