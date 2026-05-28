//! ChunkOrDone: signals whether an operator has more data or is finished.

use crate::chunk::Chunk;

/// Signals the result of an Operator::next_chunk() call.
#[derive(Debug, Clone)]
pub enum ChunkOrDone {
    /// A chunk of data is available.
    Chunk(Chunk),
    /// The operator has finished producing data.
    Done,
}

impl ChunkOrDone {
    /// Returns true if this is a Chunk.
    pub fn is_chunk(&self) -> bool {
        matches!(self, ChunkOrDone::Chunk(_))
    }

    /// Returns true if this is Done.
    pub fn is_done(&self) -> bool {
        matches!(self, ChunkOrDone::Done)
    }

    /// Consumes self and returns the Chunk if any.
    pub fn into_chunk(self) -> Option<Chunk> {
        match self {
            ChunkOrDone::Chunk(c) => Some(c),
            ChunkOrDone::Done => None,
        }
    }

    /// Returns a reference to the Chunk if any.
    pub fn as_chunk(&self) -> Option<&Chunk> {
        match self {
            ChunkOrDone::Chunk(c) => Some(c),
            ChunkOrDone::Done => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{Chunk, ColumnVector};

    #[test]
    fn test_chunk_or_done_is_chunk() {
        let col = ColumnVector::from_int32(vec![1, 2, 3]);
        let chunk = Chunk::new(vec![col]);
        let cod = ChunkOrDone::Chunk(chunk);
        assert!(cod.is_chunk());
        assert!(!cod.is_done());
    }

    #[test]
    fn test_chunk_or_done_is_done() {
        let cod = ChunkOrDone::Done;
        assert!(!cod.is_chunk());
        assert!(cod.is_done());
    }

    #[test]
    fn test_chunk_or_done_into_chunk() {
        let col = ColumnVector::from_int32(vec![1, 2, 3]);
        let chunk = Chunk::new(vec![col]);
        let cod = ChunkOrDone::Chunk(chunk);
        let extracted = cod.into_chunk();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().rows(), 3);
    }

    #[test]
    fn test_chunk_or_done_into_chunk_none() {
        let cod = ChunkOrDone::Done;
        let extracted = cod.into_chunk();
        assert!(extracted.is_none());
    }
}
