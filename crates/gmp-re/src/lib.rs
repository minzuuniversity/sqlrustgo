//! GMP RE - Relation Extraction for GMP domain
//!
//! Extracts semantic relations between GMP entities using rule-based templates.

pub mod extractor;
pub mod relations;
pub mod types;

pub use extractor::RelationExtractor;
pub use relations::RelationTemplate;
pub use types::{GmpRelation, RelationResult, RelationType};
