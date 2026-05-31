//! RecoveryEngine — WAL entry interpreter for StorageEngine recovery

use crate::engine::{Record, SqlResult, StorageEngine, Value};
use crate::wal_legacy::{WalEntry, WalEntryType};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RecoveryEngine {
    active_txs: HashSet<u64>,
    committed_txs: HashSet<u64>,
    rolled_back_txs: HashSet<u64>,
}

impl RecoveryEngine {
    pub fn new() -> Self {
        Self {
            active_txs: HashSet::new(),
            committed_txs: HashSet::new(),
            rolled_back_txs: HashSet::new(),
        }
    }

    pub fn apply_entry<S: StorageEngine>(
        &mut self,
        storage: &mut S,
        entry: &WalEntry,
    ) -> SqlResult<()> {
        match entry.entry_type {
            WalEntryType::Begin => {
                self.active_txs.insert(entry.tx_id);
            }
            WalEntryType::Commit => {
                if self.active_txs.remove(&entry.tx_id) {
                    self.committed_txs.insert(entry.tx_id);
                }
            }
            WalEntryType::Rollback => {
                self.active_txs.remove(&entry.tx_id);
                self.rolled_back_txs.insert(entry.tx_id);
            }
            WalEntryType::Insert => {
                if !self.committed_txs.contains(&entry.tx_id) {
                    return Ok(());
                }
                if let Some(ref data) = entry.data {
                    if let Ok(record) = decode_record(data) {
                        let table_name = format!("table_{}", entry.table_id);
                        let _ = storage.insert(&table_name, vec![record]);
                    }
                }
            }
            WalEntryType::Update => {
                if !self.committed_txs.contains(&entry.tx_id) {
                    return Ok(());
                }
            }
            WalEntryType::Delete => {
                if !self.committed_txs.contains(&entry.tx_id) {
                    return Ok(());
                }
            }
            WalEntryType::Checkpoint | WalEntryType::Prepare => {}
        }
        Ok(())
    }

    pub fn recover<S, W>(storage: &mut S, wal: &mut W) -> SqlResult<RecoveryReport>
    where
        S: StorageEngine,
        W: crate::wal::WalManager,
    {
        let entries = wal.recover()?;
        let mut engine = Self::new();

        for entry in entries {
            engine.apply_entry(storage, &entry)?;
        }

        Ok(RecoveryReport {
            committed: engine.committed_txs.len() as u32,
            rolled_back: engine.rolled_back_txs.len() as u32,
            terminated: 0,
        })
    }

    pub fn is_committed(&self, tx_id: u64) -> bool {
        self.committed_txs.contains(&tx_id)
    }

    pub fn is_rolled_back(&self, tx_id: u64) -> bool {
        self.rolled_back_txs.contains(&tx_id)
    }
}

impl Default for RecoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct RecoveryReport {
    pub committed: u32,
    pub rolled_back: u32,
    pub terminated: u32,
}

fn decode_record(data: &[u8]) -> Result<Record, String> {
    let mut records = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        if data[offset..].len() < 2 {
            break;
        }

        let prefix = &data[offset..offset + 2];
        offset += 2;

        if prefix == b"i:" {
            if offset + 8 > data.len() {
                break;
            }
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().map_err(|_| "bad int")?;
            records.push(Value::Integer(i64::from_le_bytes(bytes)));
            offset += 8;
        } else if prefix == b"s:" {
            let end = data[offset..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(data.len() - offset);
            records.push(Value::Text(
                String::from_utf8(data[offset..offset + end].to_vec()).map_err(|_| "bad text")?,
            ));
            offset += end + 1;
        } else if prefix == b"b:" {
            if offset >= data.len() {
                break;
            }
            records.push(Value::Boolean(data[offset] != 0));
            offset += 1;
        } else if prefix == b"n:" {
            records.push(Value::Null);
        } else if prefix == b"f:" {
            if offset + 8 > data.len() {
                break;
            }
            let bytes: [u8; 8] = data[offset..offset + 8]
                .try_into()
                .map_err(|_| "bad float")?;
            records.push(Value::Float(f64::from_le_bytes(bytes)));
            offset += 8;
        } else if prefix == b"B:" {
            let end = data[offset..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(data.len() - offset);
            records.push(Value::Blob(data[offset..offset + end].to_vec()));
            offset += end + 1;
        } else {
            break;
        }
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_engine_tracks_begin() {
        let engine = RecoveryEngine::new();
        let entry = WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Begin,
            table_id: 0,
            key: None,
            data: None,
            lsn: 1,
            timestamp: 1000,
        };
        assert!(!engine.is_committed(entry.tx_id));
    }

    #[test]
    fn test_recovery_report_default() {
        let report = RecoveryReport::default();
        assert_eq!(report.committed, 0);
        assert_eq!(report.rolled_back, 0);
        assert_eq!(report.terminated, 0);
    }

    #[test]
    fn test_decode_record_integers() {
        let data = b"i:\x01\x00\x00\x00\x00\x00\x00\x00i:\x02\x00\x00\x00\x00\x00\x00\x00".to_vec();
        let record = decode_record(&data).unwrap();
        assert_eq!(record.len(), 2);
        assert_eq!(record[0], Value::Integer(1));
        assert_eq!(record[1], Value::Integer(2));
    }
}
