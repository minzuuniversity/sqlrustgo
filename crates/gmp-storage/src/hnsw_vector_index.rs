//! HNSW Vector Index wrapper around sqlrustgo_vector::HnswIndex

use anyhow::{Context, Result};
use sqlrustgo_vector::traits::IndexEntry;
use sqlrustgo_vector::{DistanceMetric, HnswIndex, VectorIndex};
use std::sync::{Arc, Mutex};

/// Wrapper around sqlrustgo_vector::HnswIndex for GMP storage
pub struct HnswVectorIndex {
    inner: Arc<Mutex<HnswIndex>>,
    dimension: usize,
}

impl HnswVectorIndex {
    /// Create a new HNSW index with default parameters
    pub fn new(dimension: usize) -> Self {
        let mut index = HnswIndex::new(DistanceMetric::Cosine);
        // Set dimension by inserting a dummy vector
        let dummy = vec![0.0f32; dimension];
        let _ = HnswIndex::insert(&mut index, 0, &dummy);
        // Remove the dummy entry
        let _ = HnswIndex::delete(&mut index, 0);

        Self {
            inner: Arc::new(Mutex::new(index)),
            dimension,
        }
    }

    /// Create with custom HNSW parameters
    pub fn with_params(
        m: usize,
        ef_construction: usize,
        ef_search: usize,
        dimension: usize,
    ) -> Self {
        let mut index =
            HnswIndex::with_params(m, ef_construction, ef_search, DistanceMetric::Cosine);
        // Initialize dimension
        let dummy = vec![0.0f32; dimension];
        let _ = HnswIndex::insert(&mut index, 0, &dummy);
        let _ = HnswIndex::delete(&mut index, 0);

        Self {
            inner: Arc::new(Mutex::new(index)),
            dimension,
        }
    }

    /// Insert a vector with given ID
    pub fn insert(&self, id: u64, vector: &[f32]) -> Result<()> {
        let mut index = self.inner.lock().unwrap();
        HnswIndex::insert(&mut index, id, vector).context("Failed to insert vector into HNSW index")
    }

    /// Search for k nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<IndexEntry>> {
        let index = self.inner.lock().unwrap();
        HnswIndex::search(&index, query, k).context("Failed to search HNSW index")
    }

    /// Get the number of vectors in the index
    pub fn len(&self) -> usize {
        let index = self.inner.lock().unwrap();
        HnswIndex::len(&index)
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        let index = self.inner.lock().unwrap();
        HnswIndex::is_empty(&index)
    }

    /// Get the dimension of vectors in this index
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Build the index (HNSW is built incrementally, but this triggers optimization)
    pub fn build_index(&self) -> Result<()> {
        let mut index = self.inner.lock().unwrap();
        HnswIndex::build_index(&mut index).context("Failed to build HNSW index")
    }

    /// Delete a vector by ID
    pub fn delete(&self, id: u64) -> Result<()> {
        let mut index = self.inner.lock().unwrap();
        HnswIndex::delete(&mut index, id).context("Failed to delete vector from HNSW index")
    }

    /// Get all vectors as records
    pub fn get_all(&self) -> Vec<(u64, Vec<f32>)> {
        let index = self.inner.lock().unwrap();
        HnswIndex::get_all(&index)
            .into_iter()
            .map(|r| (r.id, r.vector))
            .collect()
    }
}

impl Clone for HnswVectorIndex {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            dimension: self.dimension,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_insert_and_search() {
        let index = HnswVectorIndex::new(4);

        // Insert some vectors
        index.insert(1, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        index.insert(2, &[0.0, 1.0, 0.0, 0.0]).unwrap();
        index.insert(3, &[0.0, 0.0, 1.0, 0.0]).unwrap();

        assert_eq!(index.len(), 3);

        // Search for similar vector
        let results = index.search(&[1.0, 0.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
        // First result should be the most similar (itself or the same)
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_hnsw_empty() {
        let index = HnswVectorIndex::new(4);
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn test_hnsw_dimension() {
        let index = HnswVectorIndex::new(128);
        assert_eq!(index.dimension(), 128);
    }
}
