//! SelectionVector: a list of row indices for selective operations.
//!
//! A SelectionVector holds an array of row indices (positions) that refer to
//! rows in a Chunk. It is used for filtering, sorting, and join operations.

use serde::{Deserialize, Serialize};

/// A selection vector is a list of row indices.
/// Used for filter, sort, and join operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SelectionVector {
    /// The selected row indices.
    indices: Vec<usize>,
}

impl SelectionVector {
    /// Creates a new empty SelectionVector.
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
        }
    }

    /// Creates a new SelectionVector with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            indices: Vec::with_capacity(capacity),
        }
    }

    /// Creates a SelectionVector with indices [0, 1, 2, ..., n-1].
    pub fn identity(n: usize) -> Self {
        Self {
            indices: (0..n).collect(),
        }
    }

    /// Pushes a new index.
    pub fn push(&mut self, idx: usize) {
        self.indices.push(idx);
    }

    /// Appends multiple indices.
    pub fn extend(&mut self, other: &[usize]) {
        self.indices.extend_from_slice(other);
    }

    /// Clears all indices.
    pub fn clear(&mut self) {
        self.indices.clear();
    }

    /// Returns the number of selected rows.
    #[inline]
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Returns true if there are no selected rows.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Returns the index at position `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i >= self.len()`.
    #[inline]
    pub fn get(&self, i: usize) -> usize {
        self.indices[i]
    }

    /// Returns a slice of all indices.
    #[inline]
    pub fn as_slice(&self) -> &[usize] {
        &self.indices
    }

    /// Returns a mutable slice of all indices.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [usize] {
        &mut self.indices
    }
}

impl IntoIterator for SelectionVector {
    type Item = usize;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.indices.into_iter()
    }
}

impl<'a> IntoIterator for &'a SelectionVector {
    type Item = usize;
    type IntoIter = SelectionVectorIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SelectionVectorIter {
            indices: self.indices.as_slice(),
            pos: 0,
        }
    }
}

/// Iterator over a SelectionVector.
pub struct SelectionVectorIter<'a> {
    indices: &'a [usize],
    pos: usize,
}

impl<'a> Iterator for SelectionVectorIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.indices.len() {
            let idx = self.indices[self.pos];
            self.pos += 1;
            Some(idx)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_vector_basic() {
        let mut sv = SelectionVector::new();
        sv.push(0);
        sv.push(5);
        sv.push(10);
        assert_eq!(sv.len(), 3);
        assert_eq!(sv.get(0), 0);
        assert_eq!(sv.get(1), 5);
        assert_eq!(sv.get(2), 10);
    }

    #[test]
    fn test_selection_vector_identity() {
        let sv = SelectionVector::identity(5);
        assert_eq!(sv.len(), 5);
        assert_eq!(sv.as_slice(), &[0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_selection_vector_iter() {
        let sv = SelectionVector::identity(3);
        let collected: Vec<usize> = sv.into_iter().collect();
        assert_eq!(collected, vec![0, 1, 2]);
    }
}
