//! GMP Classifier - Multi-dimensional document classification for GMP domain

pub mod classifier;
pub mod taxonomy;
pub mod types;

pub use classifier::DocClassifier;
pub use taxonomy::Taxonomy;
pub use types::{ClassificationResult, DocTaxonomy};
