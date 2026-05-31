use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use sqlrustgo_storage::buffer_pool::BufferPool;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::page::Page;
use sqlrustgo_storage::wal::{WalEntry, WalEntryType, LegacyWalManager, WalReader, WalWriter};
use sqlrustgo_storage::{TableData, TableInfo};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_storage_test_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ===== Buffer Pool =====

#[test]
fn test_buffer_pool_basic() {
    let pool = BufferPool::new(10);
    assert_eq!(pool.capacity(), 10);
    assert!(pool.is_empty());
}

#[test]
fn test_buffer_pool_insert_and_get() {
    let pool = BufferPool::new(10);
    pool.insert(Arc::new(Page::new(1)));
    assert_eq!(pool.len(), 1);
    assert!(pool.get(1).is_some());
}

#[test]
fn test_buffer_pool_lru() {
    let pool = BufferPool::new(3);
    for i in 0..5 {
        pool.insert(Arc::new(Page::new(i)));
    }
    assert_eq!(pool.len(), 3);
    assert!(pool.get(0).is_none());
    assert!(pool.get(4).is_some());
}

#[test]
fn test_buffer_pool_remove() {
    let pool = BufferPool::new(10);
    pool.insert(Arc::new(Page::new(1)));
    assert!(pool.remove(1));
    assert!(pool.get(1).is_none());
}

#[test]
fn test_buffer_pool_clear() {
    let pool = BufferPool::new(10);
    for i in 0..5 {
        pool.insert(Arc::new(Page::new(i)));
    }
    pool.clear();
    assert!(pool.is_empty());
}

#[test]
fn test_buffer_pool_prefetch() {
    let pool = BufferPool::new(10);
    pool.prefetch(&[10, 20], |pid| Arc::new(Page::new(pid)));
    assert_eq!(pool.len(), 2);
}

#[test]
fn test_buffer_pool_allocate() {
    let pool = BufferPool::new(10);
    let page = pool.allocate(42);
    assert_eq!(page.page_id(), 42);
}

// ===== File Storage =====

#[test]
fn test_file_storage_create() {
    let dir = temp_dir("fsc");
    let fs = FileStorage::new(dir.clone()).unwrap();
    assert!(fs.table_names().is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_file_storage_insert_and_drop() {
    let dir = temp_dir("fsid");
    let mut fs = FileStorage::new(dir.clone()).unwrap();
    let info = TableInfo {
        name: "t".into(),
        ..Default::default()
    };
    let data = TableData { info, rows: vec![] };
    fs.insert_table("t".into(), data).unwrap();
    assert!(fs.contains_table("t"));
    fs.drop_table("t").unwrap();
    assert!(!fs.contains_table("t"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_file_storage_multiple() {
    let dir = temp_dir("fsm");
    let mut fs = FileStorage::new(dir.clone()).unwrap();
    for n in &["a", "b", "c"] {
        let info = TableInfo {
            name: (*n).into(),
            ..Default::default()
        };
        fs.insert_table((*n).into(), TableData { info, rows: vec![] })
            .unwrap();
    }
    assert_eq!(fs.table_names().len(), 3);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_file_storage_persist() {
    let dir = temp_dir("fsp");
    let mut fs = FileStorage::new(dir.clone()).unwrap();
    let info = TableInfo {
        name: "pt".into(),
        ..Default::default()
    };
    fs.insert_table("pt".into(), TableData { info, rows: vec![] })
        .unwrap();
    fs.flush().unwrap();
    assert!(fs.contains_table("pt"));
    let _ = fs::remove_dir_all(&dir);
}

// ===== WAL =====

#[test]
fn test_wal_entry_round_trip() {
    let e = WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 100,
        key: Some(vec![1]),
        data: Some(vec![10, 20]),
        lsn: 0,
        timestamp: 12345,
    };
    let bytes = e.to_bytes();
    let d = WalEntry::from_bytes(&bytes).unwrap();
    assert_eq!(d.tx_id, 1);
    assert_eq!(d.table_id, 100);
}

#[test]
fn test_wal_append_and_read() {
    let dir = temp_dir("walar");
    let p = dir.join("w.log");
    let mut w = WalWriter::new(&p).unwrap();
    w.append(&WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 1,
        key: None,
        data: Some(vec![99]),
        lsn: 0,
        timestamp: 100,
    })
    .unwrap();
    w.flush().unwrap();
    let mut r = WalReader::new(&p).unwrap();
    assert_eq!(r.read_all().unwrap().len(), 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_wal_multi_entry() {
    let dir = temp_dir("walme");
    let p = dir.join("m.log");
    let mut w = WalWriter::new(&p).unwrap();
    for i in 0..5 {
        w.append(&WalEntry {
            tx_id: i,
            entry_type: WalEntryType::Insert,
            table_id: i,
            key: None,
            data: Some(vec![i as u8]),
            lsn: 0,
            timestamp: i as u64,
        })
        .unwrap();
    }
    w.flush().unwrap();
    let mut r = WalReader::new(&p).unwrap();
    assert_eq!(r.read_all().unwrap().len(), 5);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_wal_recover() {
    let dir = temp_dir("walre");
    let p = dir.join("r.log");
    {
        let mut w = WalWriter::new(&p).unwrap();
        for i in 0..3 {
            w.append(&WalEntry {
                tx_id: i,
                entry_type: WalEntryType::Insert,
                table_id: 1,
                key: None,
                data: Some(vec![i as u8]),
                lsn: 0,
                timestamp: i as u64,
            })
            .unwrap();
        }
        w.flush().unwrap();
    }
    let mut r = WalReader::new(&p).unwrap();
    assert_eq!(r.read_all().unwrap().len(), 3);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_wal_manager_recover() {
    let dir = temp_dir("walmg");
    let p = dir.join("mgr.log");
    {
        let mut w = WalWriter::new(&p).unwrap();
        w.append(&WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: None,
            data: Some(vec![77]),
            lsn: 0,
            timestamp: 999,
        })
        .unwrap();
        w.flush().unwrap();
    }
    let mgr = LegacyWalManager::new(p.clone());
    assert_eq!(mgr.recover().unwrap().len(), 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_wal_lsn() {
    let dir = temp_dir("walls");
    let p = dir.join("l.log");
    let mut w = WalWriter::new(&p).unwrap();
    w.append(&WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 1,
        key: None,
        data: None,
        lsn: 0,
        timestamp: 1,
    })
    .unwrap();
    assert!(w.current_lsn() > 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_page_in_pool() {
    let pool = BufferPool::new(10);
    let mut page = Page::new(1);
    page.write_row(&[sqlrustgo_types::Value::Integer(42)]);
    pool.insert(Arc::new(page));
    let rows = pool.get(1).unwrap().read_rows();
    assert_eq!(rows.len(), 1);
}
