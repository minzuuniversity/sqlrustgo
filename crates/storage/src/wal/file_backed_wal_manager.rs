use crate::engine::SqlResult;
use crate::wal::{WalEntry, WalManager};
use crate::wal_legacy::WalWriter;
use std::fs::OpenOptions;
use std::io::{BufReader, Read};
use std::path::PathBuf;

/// Read every well-formed frame in a WAL file.
///
/// Stops at the first torn or unreadable frame: frames are
/// length-prefixed and written in order, so once one is short there is
/// no way to find where the next one starts. Entries that are complete
/// and precede the tear are still returned — a crash-truncated WAL is
/// exactly the case where partial recovery is the useful behaviour.
fn read_all_entries(wal_path: &PathBuf) -> Vec<WalEntry> {
    let Ok(file) = OpenOptions::new().read(true).open(wal_path) else {
        return Vec::new();
    };
    let mut reader = BufReader::new(file);
    let mut entries = Vec::new();
    loop {
        let mut len_bytes = [0u8; 4];
        if reader.read_exact(&mut len_bytes).is_err() {
            break;
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
    entries
}

/// #5055: the LSN the next append should use, derived from what is
/// already in the file.
///
/// `WalWriter` opens with `OpenOptions::append` and maintains its own
/// counter starting at 0, so a reopened WAL would otherwise be
/// renumbered from scratch on top of the entries already there.
///
/// The high-water mark is taken over *parsed* entries rather than the
/// raw frame count so the two stay consistent even if a future format
/// stamps LSNs non-contiguously; a torn tail contributes nothing,
/// which is right — those bytes were never a complete entry.
fn next_lsn_on_disk(wal_path: &PathBuf) -> u64 {
    read_all_entries(wal_path)
        .iter()
        .map(|e| e.lsn)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0)
}

pub struct FileBackedWalManager {
    wal_path: PathBuf,
    writer: Option<WalWriter>,
    /// #5055: the LSN that the next `append` will stamp into its entry.
    ///
    /// It used to live only inside `WalWriter`, which is the root of
    /// this bug: `WalWriter::append` hands out `self.lsn`, increments
    /// its private copy, and then serializes the **caller's** entry —
    /// whose `lsn` field is whatever the caller happened to set,
    /// usually 0. So the counter tracked the right number and the file
    /// recorded the wrong one: every entry on disk had `lsn == 0`.
    ///
    /// That made the log unorderable by LSN, made `truncate_before`
    /// unable to distinguish entries, and made a reopened WAL look
    /// empty to any LSN-based reasoning. The counter is now owned here
    /// and written into each entry on the way out.
    next_lsn: u64,
}

impl FileBackedWalManager {
    pub fn new(wal_path: PathBuf) -> SqlResult<Self> {
        let next_lsn = next_lsn_on_disk(&wal_path);
        let writer = WalWriter::with_config(&wal_path, false, 100).map_err(|e| {
            crate::engine::SqlError::ExecutionError(format!("WAL writer init failed: {}", e))
        })?;
        Ok(Self {
            wal_path,
            writer: Some(writer),
            next_lsn,
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
        // #5055: the file is gone, so the sequence restarts with it.
        self.next_lsn = 0;
        Ok(())
    }

    /// Append without re-stamping, preserving the entry's own LSN.
    ///
    /// Used by `truncate_before`, which is *moving* existing entries
    /// to a new file. Renumbering them there would silently change the
    /// LSNs a checkpoint already refers to — the exact failure the
    /// LSN is supposed to make impossible.
    fn append_preserving_lsn(&mut self, entry: &WalEntry) -> SqlResult<()> {
        if self.writer.is_none() {
            self.writer = Some(WalWriter::with_config(&self.wal_path, false, 100).map_err(
                |e| {
                    crate::engine::SqlError::ExecutionError(format!(
                        "WAL writer init failed: {}",
                        e
                    ))
                },
            )?);
        }
        if let Some(writer) = &mut self.writer {
            writer.append(entry).map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!("WAL append failed: {}", e))
            })?;
        }
        Ok(())
    }
}

impl WalManager for FileBackedWalManager {
    /// #5055: stamp `next_lsn` into the entry before it reaches the
    /// file, then advance.
    fn append(&mut self, entry: WalEntry) -> SqlResult<()> {
        let mut entry = entry;
        entry.lsn = self.next_lsn;
        self.append_preserving_lsn(&entry)?;
        self.next_lsn += 1;
        Ok(())
    }

    fn flush(&mut self) -> SqlResult<()> {
        if let Some(writer) = &mut self.writer {
            writer.flush().map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!("WAL flush failed: {}", e))
            })?;
        }
        Ok(())
    }

    fn sync(&mut self) -> SqlResult<()> {
        if let Some(writer) = &mut self.writer {
            writer.flush().map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!("WAL flush failed: {}", e))
            })?;
            // Call sync_data on the underlying file for true durability.
            // Without this, only BufWriter's buffer is flushed to kernel
            // page cache, not to durable storage.
            writer.get_mut().get_ref().sync_data().map_err(|e| {
                crate::engine::SqlError::ExecutionError(format!("WAL sync_data failed: {}", e))
            })?;
        }
        Ok(())
    }

    fn recover(&mut self) -> SqlResult<Vec<WalEntry>> {
        Ok(read_all_entries(&self.wal_path))
    }

    fn truncate_before(&mut self, lsn: u64) -> SqlResult<()> {
        let entries = self.recover()?;
        let retained: Vec<_> = entries.into_iter().filter(|e| e.lsn >= lsn).collect();
        self.truncate()?;
        // #5055: preserve the surviving entries' own LSNs, then resume
        // numbering after the highest of them.
        for entry in &retained {
            self.append_preserving_lsn(entry)?;
        }
        self.next_lsn = retained
            .iter()
            .map(|e| e.lsn)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        Ok(())
    }

    /// The LSN the next `append` will stamp — i.e. one past the highest
    /// LSN written so far, matching the pre-#5055 meaning of
    /// `WalWriter::current_lsn`.
    fn current_lsn(&self) -> u64 {
        self.next_lsn
    }

    fn size(&self) -> SqlResult<u64> {
        Ok(std::fs::metadata(&self.wal_path)?.len())
    }

    fn is_batch_mode(&self) -> bool {
        self.writer
            .as_ref()
            .map(|w| w.is_batch_mode())
            .unwrap_or(false)
    }

    fn flush_threshold(&self) -> usize {
        self.writer
            .as_ref()
            .map(|w| w.flush_threshold())
            .unwrap_or(100)
    }

    fn set_batch_mode(&mut self, enable: bool) {
        if let Some(writer) = &mut self.writer {
            writer.enable_batch_mode(enable);
        }
    }

    fn set_flush_threshold(&mut self, threshold: usize) {
        if let Some(writer) = &mut self.writer {
            writer.set_flush_threshold(threshold);
        }
    }
}
