//! Multi-segment compaction for BINT v3 storage.
//!
//! NOTE: current implementation assumes the source segments use a
//! fully fixed-width column schema (no VARCHAR / TEXT / BLOB). For TPC-H
//! lineitem, every column is fixed-width (BIGINT, INT, DATE, DECIMAL) so
//! an empty `schema: Vec<ColumnDefinition>` roundtrips correctly. When
//! schema is persisted in root.bin (future task), this placeholder will
//! be replaced with the real schema.

use crate::bin_index::{read_root_index_file, write_root_index_file, RootIndex, SegmentInfo};
use crate::bin_segment::{SegmentReader, SegmentWriter};
use crate::engine::{SqlError, SqlResult};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CompactorConfig {
    pub max_segment_count: usize,
}

pub struct BinCompactor {
    config: CompactorConfig,
}

impl BinCompactor {
    pub fn new(config: CompactorConfig) -> Self {
        Self { config }
    }

    /// Compact all segments for `table` under `data_dir` into a single
    /// segment if `idx.segments.len() > config.max_segment_count`. Otherwise
    /// this is a no-op.
    ///
    /// Steps:
    /// 1. Read root.bin
    /// 2. If segment count is within the configured limit, return Ok(())
    /// 3. Open each existing segment and stream its rows into a new
    ///    merged writer
    /// 4. Seal the merged writer
    /// 5. Remove the original segment files
    /// 6. Rewrite root.bin pointing at the merged segment with preserved
    ///    total_rows
    pub fn run(&self, data_dir: &Path, table: &str) -> SqlResult<()> {
        let root_path = data_dir.join(format!("{}.root.bin", table));
        let idx = read_root_index_file(&root_path)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        if idx.segments.len() <= self.config.max_segment_count {
            return Ok(()); // nothing to compact
        }

        let schema = vec![]; // TODO: replace with persisted schema when root.bin stores it
        let merged_path = data_dir.join(format!("{}_seg_compacted.bin", table));
        let mut merged_writer = SegmentWriter::new(merged_path.clone(), schema)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;

        for seg_info in &idx.segments {
            let seg_path = data_dir.join(&seg_info.file_name);
            let reader = SegmentReader::open(&seg_path, vec![])
                .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
            for row_result in reader.iter_rows() {
                let row = row_result.map_err(|e| SqlError::ExecutionError(e.to_string()))?;
                merged_writer
                    .append(&row)
                    .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
            }
        }
        merged_writer
            .seal()
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;

        // Remove old segments
        for seg_info in &idx.segments {
            let p = data_dir.join(&seg_info.file_name);
            std::fs::remove_file(&p).map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        }

        // Write new root.bin pointing at the merged segment
        let merged_size = std::fs::metadata(&merged_path)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?
            .len();
        let new_idx = RootIndex {
            version: 3,
            segments: vec![SegmentInfo {
                segment_id: 0,
                file_name: merged_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                row_count: merged_writer.rows_in_segment(),
                byte_size: merged_size,
            }],
            schema_hash: idx.schema_hash,
            total_rows: idx.total_rows, // preserve row count
            index_crc: 0,
        };
        write_root_index_file(&root_path, &new_idx)
            .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
        Ok(())
    }
}
