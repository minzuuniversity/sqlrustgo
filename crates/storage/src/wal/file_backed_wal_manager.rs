use crate::engine::SqlResult;
use crate::wal::{WalEntry, WalManager};
use crate::wal_legacy::WalWriter;
use std::fs::OpenOptions;
use std::io::{BufReader, Read};
use std::path::PathBuf;

pub struct FileBackedWalManager {
    wal_path: PathBuf,
    writer: Option<WalWriter>,
}

impl FileBackedWalManager {
    pub fn new(wal_path: PathBuf) -> SqlResult<Self> {
        let writer = WalWriter::with_config(&wal_path, false, 100).map_err(|e| {
            crate::engine::SqlError::ExecutionError(format!("WAL writer init failed: {}", e))
        })?;
        Ok(Self {
            wal_path,
            writer: Some(writer),
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.wal_path
    }

    pub fn size(&self) -> SqlResult<u64> {
        Ok(std::fs::metadata(&self.wal_path)?.len())
    }

    pub fn truncate(&mut self) -> SqlResult<()> {
        std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.wal_path)?;
        self.writer = None;
        Ok(())
    }
}

impl WalManager for FileBackedWalManager {
    fn append(&mut self, entry: WalEntry) -> SqlResult<()> {
        if self.writer.is_none() {
            self.writer = Some(WalWriter::with_config(&self.wal_path, false, 100).map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!("WAL writer init failed: {}", e))
            })?);
        }
        if let Some(ref mut writer) = self.writer {
            writer.append(&entry).map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!("WAL append failed: {}", e))
            })?;
        }
        Ok(())
    }

    fn flush(&mut self) -> SqlResult<()> {
        if let Some(ref mut writer) = self.writer {
            writer.flush().map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!("WAL flush failed: {}", e))
            })?;
        }
        Ok(())
    }

    fn sync(&mut self) -> SqlResult<()> {
        if let Some(ref mut writer) = self.writer {
            writer.flush().map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!("WAL sync failed: {}", e))
            })?;
        }
        Ok(())
    }

    fn recover(&mut self) -> SqlResult<Vec<WalEntry>> {
        let file = match OpenOptions::new().read(true).open(&self.wal_path) {
            Ok(f) => f,
            Err(_) => return Ok(Vec::new()),
        };
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();
        loop {
            let mut len_bytes = [0u8; 4];
            match reader.read_exact(&mut len_bytes) {
                Ok(_) => {}
                Err(_) => break,
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            if len > 1024 * 1024 * 1024 {
                break;
            }
            let mut data = vec![0u8; len];
            if reader.read_exact(&mut data).is_err() {
                break;
            }
            if let Some(entry) = WalEntry::from_bytes(&data) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }
}