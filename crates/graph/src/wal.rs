//! Write-ahead log for graph mutations.
//!
//! Records are appended to a single file, one record per line (LF-delimited JSON-ish framing
//! around a bincode-encoded payload). Each record carries a CRC32 checksum so that torn
//! writes (kill -9 mid-record) can be detected and skipped during recovery.
//!
//! The WAL is intentionally simple and append-only. Snapshots (`DiskGraphStore::flush`)
//! periodically compact the WAL into a single snapshot file; on next start the snapshot
//! is loaded and only WAL records **after** the snapshot high-water-mark are replayed.

use crate::types::{EdgeId, GraphError, GraphResult, Label, NodeId, PropertyMap};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Magic byte sequence marking the start of every WAL record (after the 4-byte length prefix).
///
/// Not strictly required, but lets `strings`-style tools identify a graph WAL file.
const WAL_MAGIC: [u8; 4] = *b"GWA1";

/// One mutation recorded in the WAL.
///
/// `Serialize`/`Deserialize` derive is on the inner enum so we can use bincode
/// without external version tags (we keep the field set small and additive).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WalRecord {
    /// Node created with the given labels and properties.
    CreateNode {
        /// Assigned node id.
        node_id: NodeId,
        /// Node labels.
        labels: Vec<Label>,
        /// Node properties.
        properties: PropertyMap,
    },
    /// Node replaced (labels + properties).
    UpdateNode {
        /// Node id being updated.
        node_id: NodeId,
        /// New labels.
        labels: Vec<Label>,
        /// New properties.
        properties: PropertyMap,
    },
    /// Node deleted; cascades to incident edges via the in-memory state.
    DeleteNode {
        /// Node id being deleted.
        node_id: NodeId,
    },
    /// Edge created between two existing nodes.
    CreateEdge {
        /// Assigned edge id.
        edge_id: EdgeId,
        /// Source endpoint.
        source: NodeId,
        /// Target endpoint.
        target: NodeId,
        /// Relationship type.
        rel_type: Label,
        /// Edge properties.
        properties: PropertyMap,
    },
    /// Edge properties replaced.
    UpdateEdge {
        /// Edge id being updated.
        edge_id: EdgeId,
        /// New properties.
        properties: PropertyMap,
    },
    /// Edge deleted.
    DeleteEdge {
        /// Edge id being deleted.
        edge_id: EdgeId,
    },
    /// Monotonically increasing id counters checkpoint.
    /// Lets a recovered store resume ids from the last known high-water-mark.
    SetNextIds {
        /// Next node id to allocate.
        next_node_id: u64,
        /// Next edge id to allocate.
        next_edge_id: u64,
    },
    /// Marks the high-water-mark after a successful snapshot.
    /// During recovery, all records strictly **after** this one must be replayed.
    SnapshotMark {
        /// LSN of the last record included in the snapshot.
        /// Records with `lsn <= this` are already applied via snapshot load.
        up_to_lsn: u64,
    },
}

/// A WAL record plus its assigned log sequence number.
#[derive(Debug, Clone)]
pub struct WalEntry {
    /// Log sequence number, 1-indexed. First appended record has `lsn == 1`.
    pub lsn: u64,
    /// The mutation.
    pub record: WalRecord,
}

/// On-disk handle to the WAL file.
///
/// Append-only; each `append` writes one framed record and returns its LSN.
/// `replay` walks the file and returns every well-formed record (skipping torn writes).
pub struct Wal {
    path: PathBuf,
    writer: BufWriter<File>,
    next_lsn: u64,
}

impl Wal {
    /// Opens (or creates) the WAL at `path`. If the file already exists, its last LSN is
    /// detected and new records continue from `last_lsn + 1`.
    pub fn open(path: impl Into<PathBuf>) -> GraphResult<Self> {
        let path = path.into();
        let next_lsn = if path.exists() {
            last_lsn_in_file(&path)?.unwrap_or(0) + 1
        } else {
            1
        };
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            writer: BufWriter::new(file),
            next_lsn,
        })
    }

    /// Path to the WAL file on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The LSN that the **next** appended record will receive.
    pub fn next_lsn(&self) -> u64 {
        self.next_lsn
    }

    /// Appends one record and flushes the underlying file.
    ///
    /// `flush` is called on every append to provide durability matching the
    /// "every write is durable" contract for embedded use cases; production users
    /// who want higher throughput should batch and call `flush` periodically.
    pub fn append(&mut self, record: WalRecord) -> GraphResult<u64> {
        let lsn = self.next_lsn;
        let payload = bincode::serialize(&record)?;
        let crc = crc32fast::hash(&payload);

        // Frame: [magic(4)][len(4 LE)][crc(4 LE)][payload(len)]
        self.writer.write_all(&WAL_MAGIC)?;
        self.writer
            .write_all(&(payload.len() as u32).to_le_bytes())?;
        self.writer.write_all(&crc.to_le_bytes())?;
        self.writer.write_all(&payload)?;
        self.writer.flush()?;
        // Best-effort fsync; ignore errors on platforms where it's not supported.
        let _ = self.writer.get_ref().sync_all();

        self.next_lsn = self
            .next_lsn
            .checked_add(1)
            .ok_or_else(|| GraphError::Storage("WAL LSN exhausted".into()))?;
        Ok(lsn)
    }

    /// Flushes any buffered data (no-op if already flushed).
    pub fn flush(&mut self) -> GraphResult<()> {
        self.writer.flush()?;
        Ok(())
    }

    /// Reads all records from the file at `path`, returning them in LSN order.
    ///
    /// Torn writes (truncated record, length mismatch, CRC mismatch) are silently
    /// skipped — this is how we survive kill -9 mid-record.
    pub fn replay(path: impl AsRef<Path>) -> GraphResult<Vec<WalEntry>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();
        let mut lsn: u64 = 1;

        loop {
            let mut magic = [0u8; 4];
            match reader.read_exact(&mut magic) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(GraphError::Io(e)),
            }
            if magic != WAL_MAGIC {
                // Torn before magic; stop.
                break;
            }
            let mut len_buf = [0u8; 4];
            if let Err(e) = reader.read_exact(&mut len_buf) {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    break;
                }
                return Err(GraphError::Io(e));
            }
            let len = u32::from_le_bytes(len_buf) as usize;

            let mut crc_buf = [0u8; 4];
            if let Err(e) = reader.read_exact(&mut crc_buf) {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    break;
                }
                return Err(GraphError::Io(e));
            }
            let stored_crc = u32::from_le_bytes(crc_buf);

            let mut payload = vec![0u8; len];
            if let Err(e) = reader.read_exact(&mut payload) {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    // Truncated payload: stop.
                    break;
                }
                return Err(GraphError::Io(e));
            }

            let actual_crc = crc32fast::hash(&payload);
            if actual_crc != stored_crc {
                // Corrupted record: stop. Anything after this may also be corrupt.
                break;
            }

            let record: WalRecord = bincode::deserialize(&payload)
                .map_err(|e| GraphError::Serde(format!("WAL deserialize: {e}")))?;
            entries.push(WalEntry { lsn, record });
            lsn += 1;
        }

        Ok(entries)
    }
}

/// Reads the file and returns the highest LSN of a well-formed record, if any.
fn last_lsn_in_file(path: &Path) -> GraphResult<Option<u64>> {
    let entries = Wal::replay(path)?;
    Ok(if entries.is_empty() {
        None
    } else {
        Some(entries.last().unwrap().lsn)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PropertyValue;
    use tempfile::tempdir;

    fn rec_create_node(id: u64, name: &str) -> WalRecord {
        WalRecord::CreateNode {
            node_id: NodeId(id),
            labels: vec!["Person".into()],
            properties: PropertyMap::from_pairs(vec![("name", name)]),
        }
    }

    #[test]
    fn open_fresh_wal_starts_at_lsn_1() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");
        let w = Wal::open(&path).unwrap();
        assert_eq!(w.next_lsn(), 1);
        assert!(!path.exists() || path.metadata().unwrap().len() == 0);
    }

    #[test]
    fn append_returns_increasing_lsns() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");
        let mut w = Wal::open(&path).unwrap();
        let l1 = w.append(rec_create_node(0, "Alice")).unwrap();
        let l2 = w.append(rec_create_node(1, "Bob")).unwrap();
        assert_eq!(l1, 1);
        assert_eq!(l2, 2);
        assert_eq!(w.next_lsn(), 3);
    }

    #[test]
    fn reopen_continues_lsn_sequence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");
        {
            let mut w = Wal::open(&path).unwrap();
            w.append(rec_create_node(0, "Alice")).unwrap();
            w.append(rec_create_node(1, "Bob")).unwrap();
        }
        let w2 = Wal::open(&path).unwrap();
        assert_eq!(w2.next_lsn(), 3);
    }

    #[test]
    fn replay_returns_records_in_order() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");
        let mut w = Wal::open(&path).unwrap();
        w.append(rec_create_node(0, "Alice")).unwrap();
        w.append(rec_create_node(1, "Bob")).unwrap();
        w.append(rec_create_node(2, "Carol")).unwrap();

        let entries = Wal::replay(&path).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].lsn, 1);
        assert_eq!(entries[2].lsn, 3);
        match &entries[1].record {
            WalRecord::CreateNode { node_id, .. } => assert_eq!(*node_id, NodeId(1)),
            other => panic!("unexpected record: {other:?}"),
        }
    }

    #[test]
    fn replay_handles_truncated_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");
        let mut w = Wal::open(&path).unwrap();
        w.append(rec_create_node(0, "Alice")).unwrap();
        w.append(rec_create_node(1, "Bob")).unwrap();

        // Truncate the file mid-second-record.
        let bytes = std::fs::read(&path).unwrap();
        let truncated = &bytes[..bytes.len() - 5];
        std::fs::write(&path, truncated).unwrap();

        // Replay should return the first record and silently stop at the truncated one.
        let entries = Wal::replay(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].lsn, 1);
    }

    #[test]
    fn replay_returns_empty_for_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.wal");
        assert!(Wal::replay(&path).unwrap().is_empty());
    }

    #[test]
    fn reopen_after_truncation_starts_at_correct_lsn() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");
        let mut w = Wal::open(&path).unwrap();
        w.append(rec_create_node(0, "Alice")).unwrap();
        w.append(rec_create_node(1, "Bob")).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let truncated = &bytes[..bytes.len() - 5];
        std::fs::write(&path, truncated).unwrap();

        // Should detect last_lsn = 1 and start new records at 2.
        let w2 = Wal::open(&path).unwrap();
        assert_eq!(w2.next_lsn(), 2);
    }

    #[test]
    fn wal_record_variants_serialize_round_trip() {
        // Smoke-test for every variant; ensures the enum doesn't break across changes.
        let records = vec![
            WalRecord::CreateNode {
                node_id: NodeId(7),
                labels: vec!["Person".into(), "Friend".into()],
                properties: {
                    let mut pm = PropertyMap::new();
                    pm.insert("name", "Alice".to_string());
                    pm.insert("age", 30_i64);
                    pm
                },
            },
            WalRecord::UpdateNode {
                node_id: NodeId(7),
                labels: vec!["Person".into()],
                properties: PropertyMap::new(),
            },
            WalRecord::DeleteNode { node_id: NodeId(7) },
            WalRecord::CreateEdge {
                edge_id: EdgeId(3),
                source: NodeId(7),
                target: NodeId(8),
                rel_type: "KNOWS".into(),
                properties: PropertyMap::new(),
            },
            WalRecord::UpdateEdge {
                edge_id: EdgeId(3),
                properties: PropertyMap::from_pairs(vec![("since", 2020_i64)]),
            },
            WalRecord::DeleteEdge { edge_id: EdgeId(3) },
            WalRecord::SetNextIds {
                next_node_id: 10,
                next_edge_id: 5,
            },
            WalRecord::SnapshotMark { up_to_lsn: 100 },
        ];
        for r in &records {
            let bytes = bincode::serialize(r).unwrap();
            let back: WalRecord = bincode::deserialize(&bytes).unwrap();
            assert_eq!(&back, r);
            // Ensure PropertyValue types survive (Int via age/since).
            match &back {
                WalRecord::CreateNode { properties, .. } => {
                    assert!(matches!(
                        properties.get("age"),
                        Some(PropertyValue::Int(30))
                    ));
                }
                _ => {}
            }
        }
    }
}
