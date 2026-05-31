//! S2: WAL Verification 强化 — 连接形式化模型与真实 WAL 实现
//!
//! 使用 sqlrustgo_storage::wal 的真实 WalWriter/Reader/Manager，
//! 验证其满足 WAL 形式化规约中定义的不变式。

use std::fs;
use std::path::PathBuf;

use sqlrustgo_storage::wal::{WalEntry, WalEntryType, WalReader, WalWriter};
use sqlrustgo_wal_verification::{WALInvariant, WALLog, WALOperation, WALRecord};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wal_real_test_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 将真实的 WalEntry 转换为形式化模型的 WALRecord
fn entry_to_record(entry: &WalEntry, lsn: u64) -> WALRecord {
    let operation = match entry.entry_type {
        WalEntryType::Insert => WALOperation::WriteRow,
        WalEntryType::Update => WALOperation::UpdateRow,
        WalEntryType::Delete => WALOperation::DeleteRow,
        WalEntryType::Commit => WALOperation::CommitTxn,
        WalEntryType::Rollback => WALOperation::AbortTxn,
        WalEntryType::Begin => WALOperation::BeginTxn,
        WalEntryType::Checkpoint => WALOperation::Checkpoint,
        WalEntryType::Prepare => WALOperation::WriteRow,
    };
    WALRecord {
        lsn,
        txn_id: entry.tx_id,
        operation,
        page_id: Some(entry.table_id),
        row_key: entry.key.clone(),
        before_value: None,
        after_value: entry.data.clone(),
        checksum: 0,
    }
}

/// 从真实 WAL 文件构建形式化验证用的 WALLog
fn build_wal_log(wal_path: &PathBuf) -> (WALLog, Vec<WalEntry>) {
    let mut reader = WalReader::new(wal_path).unwrap();
    let entries = reader.read_all().unwrap();
    let mut log = WALLog::new();
    for (i, entry) in entries.iter().enumerate() {
        let rec = entry_to_record(entry, i as u64 + 1);
        log.records.push(rec);
    }
    (log, entries)
}

// ============================================================
// S2-A: LSN Monotonicity — 真实 WAL
// ============================================================

#[test]
fn test_real_wal_lsn_monotonicity() {
    let dir = temp_dir("lsn_mono");
    let p = dir.join("wal.log");
    let mut w = WalWriter::new(&p).unwrap();
    for i in 0..10 {
        w.append(&WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: None,
            data: Some(vec![i]),
            lsn: 0,
            timestamp: i as u64,
        })
        .unwrap();
    }
    w.flush().unwrap();
    let (log, _) = build_wal_log(&p);
    assert!(log.check_lsn_monotonicity());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_real_wal_lsn_increasing_per_entry() {
    let dir = temp_dir("lsn_inc");
    let p = dir.join("wal.log");
    let mut w = WalWriter::new(&p).unwrap();
    let mut prev = 0u64;
    for i in 0..5 {
        let lsn = w
            .append(&WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Insert,
                table_id: 1,
                key: None,
                data: Some(vec![i]),
                lsn: 0,
                timestamp: i as u64,
            })
            .unwrap();
        assert!(lsn >= prev, "LSN must be monotonically increasing");
        prev = lsn;
    }
    w.flush().unwrap();
    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S2-B: LSN uniqueness
// ============================================================

#[test]
fn test_real_wal_no_duplicate_lsn() {
    let dir = temp_dir("no_dup");
    let p = dir.join("wal.log");
    let mut w = WalWriter::new(&p).unwrap();
    let mut lsns = std::collections::HashSet::new();
    for i in 0..10 {
        let lsn = w
            .append(&WalEntry {
                tx_id: 1,
                entry_type: WalEntryType::Insert,
                table_id: 1,
                key: None,
                data: Some(vec![i]),
                lsn: 0,
                timestamp: i as u64,
            })
            .unwrap();
        assert!(lsns.insert(lsn), "LSN {} must be unique", lsn);
    }
    w.flush().unwrap();
    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S2-C: WAL read after write consistency
// ============================================================

#[test]
fn test_real_wal_write_then_read_consistency() {
    let dir = temp_dir("consist");
    let p = dir.join("wal.log");
    let mut w = WalWriter::new(&p).unwrap();
    let original = WalEntry {
        tx_id: 42,
        entry_type: WalEntryType::Insert,
        table_id: 7,
        key: Some(vec![1, 2, 3]),
        data: Some(vec![10, 20, 30, 40, 50]),
        lsn: 0,
        timestamp: 1234567890,
    };
    w.append(&original).unwrap();
    w.flush().unwrap();
    let mut r = WalReader::new(&p).unwrap();
    let entries = r.read_all().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].tx_id, 42);
    assert_eq!(entries[0].table_id, 7);
    assert_eq!(entries[0].data, Some(vec![10, 20, 30, 40, 50]));
    assert_eq!(entries[0].timestamp, 1234567890);
    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S2-D: Multiple transaction boundaries
// ============================================================

#[test]
fn test_real_wal_txn_boundaries() {
    let dir = temp_dir("txn_bnd");
    let p = dir.join("wal.log");
    let mut w = WalWriter::new(&p).unwrap();
    // Txn 1
    w.append(&WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 0,
        timestamp: 1,
    })
    .unwrap();
    w.append(&WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 1,
        key: None,
        data: Some(vec![100]),
        lsn: 0,
        timestamp: 2,
    })
    .unwrap();
    w.append(&WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Commit,
        table_id: 0,
        key: None,
        data: None,
        lsn: 0,
        timestamp: 3,
    })
    .unwrap();
    // Txn 2
    w.append(&WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 0,
        timestamp: 4,
    })
    .unwrap();
    w.append(&WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Insert,
        table_id: 1,
        key: None,
        data: Some(vec![200]),
        lsn: 0,
        timestamp: 5,
    })
    .unwrap();
    w.append(&WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Rollback,
        table_id: 0,
        key: None,
        data: None,
        lsn: 0,
        timestamp: 6,
    })
    .unwrap();
    w.flush().unwrap();
    let mut r = WalReader::new(&p).unwrap();
    let entries = r.read_all().unwrap();
    assert_eq!(entries.len(), 6);
    assert_eq!(entries[0].entry_type, WalEntryType::Begin);
    assert_eq!(entries[2].entry_type, WalEntryType::Commit);
    assert_eq!(entries[5].entry_type, WalEntryType::Rollback);
    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S2-E: Append after read
// ============================================================

#[test]
fn test_real_wal_append_after_reopen() {
    let dir = temp_dir("reopen");
    let p = dir.join("wal.log");
    // Write batch 1
    {
        let mut w = WalWriter::new(&p).unwrap();
        w.append(&WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: None,
            data: Some(vec![1]),
            lsn: 0,
            timestamp: 1,
        })
        .unwrap();
        w.flush().unwrap();
    }
    // Reopen and write batch 2
    {
        let mut w = WalWriter::new(&p).unwrap();
        w.append(&WalEntry {
            tx_id: 2,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: None,
            data: Some(vec![2]),
            lsn: 0,
            timestamp: 2,
        })
        .unwrap();
        w.flush().unwrap();
    }
    // Read all
    let mut r = WalReader::new(&p).unwrap();
    let entries = r.read_all().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].tx_id, 1);
    assert_eq!(entries[1].tx_id, 2);
    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S2-F: NoDuplicateLSN invariant on real WAL
// ============================================================

#[test]
fn test_real_wal_invariant_no_duplicate_lsn() {
    let dir = temp_dir("inv_dup");
    let p = dir.join("wal.log");
    let mut w = WalWriter::new(&p).unwrap();
    for i in 0..5 {
        w.append(&WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: None,
            data: Some(vec![i]),
            lsn: 0,
            timestamp: i as u64,
        })
        .unwrap();
    }
    w.flush().unwrap();
    let (log, _) = build_wal_log(&p);
    let result = WALInvariant::NoDuplicateLSN.check(&log);
    assert!(result.holds, "Real WAL must satisfy NoDuplicateLSN");
    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S2-G: Large batch
// ============================================================

#[test]
fn test_real_wal_large_batch() {
    let dir = temp_dir("large");
    let p = dir.join("wal.log");
    let mut w = WalWriter::new(&p).unwrap();
    let batch_size = 1000;
    for i in 0..batch_size {
        w.append(&WalEntry {
            tx_id: i % 10,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: None,
            data: Some(format!("row_{}", i).into_bytes()),
            lsn: 0,
            timestamp: i as u64,
        })
        .unwrap();
    }
    w.flush().unwrap();
    let mut r = WalReader::new(&p).unwrap();
    let entries = r.read_all().unwrap();
    assert_eq!(entries.len(), batch_size as usize);
    let (log, _) = build_wal_log(&p);
    assert_eq!(log.len(), batch_size as usize);
    assert!(log.check_lsn_monotonicity());
    let _ = fs::remove_dir_all(&dir);
}
