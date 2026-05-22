//! GMP NER - Named Entity Recognition for GMP domain
//!
//! Provides entity extraction from GMP documents using rule-based matching
//! with optional LLM-assisted fallback for complex cases.

pub mod extractor;
pub mod terms;
pub mod types;

pub use extractor::NerExtractor;
pub use terms::TermDictionary;
pub use types::{Entity, EntityType, ExtractionResult};
