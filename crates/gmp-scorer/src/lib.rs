//! GMP Document Relevance Scorer
//!
//! Scores documents based on multiple dimensions:
//! - term_density: GMP terminology usage
//! - reg_citation: Regulatory reference
//! - process_completeness: Process documentation completeness
//! - entity_density: Named entity count
//! - dept_coverage: Department assignment

pub mod types;
pub mod scorer;

pub use types::*;
pub use scorer::*;
