//! S3: End-to-End Storage Integration
//!
//! Tests the complete storage pipeline: write → persist → reload → read.
//! Simulates real usage patterns including restart scenarios.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::buffer_pool::BufferPool;
use sqlrustgo_storage::page::Page;
use sqlrustgo_storage::{TableData, TableInfo};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_e2e_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_table(name: &str) -> TableData {
    let info = TableInfo { name: name.into(), ..Default::default() };
    TableData { info, rows: vec![] }
}

// ============================================================
// S3-A: Write then reload (simulating server restart)
// ============================================================

#[test]
fn test_e2e_write_and_reload() {
    let dir = temp_dir("e2e_wr");
    let tbl_name = "persist_test";

    // Write phase
    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        fs.insert_table(tbl_name.into(), make_table(tbl_name)).unwrap();
        fs.persist_table(tbl_name).unwrap();
        fs.flush().unwrap();
    }

    // Reload phase (simulating restart)
    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        assert!(fs.contains_table(tbl_name));
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_e2e_multiple_tables_persist() {
    let dir = temp_dir("e2e_mt");
    let tables = vec!["users", "orders", "products", "reviews"];

    // Write all tables
    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        for t in &tables {
            fs.insert_table(t.to_string(), make_table(t)).unwrap();
        }
        for t in &tables {
            fs.persist_table(t).unwrap();
        }
        fs.flush().unwrap();
    }

    // Verify all survive restart
    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        let names = fs.table_names();
        for t in &tables {
            assert!(names.contains(&t.to_string()), "Table {} should survive restart", t);
        }
        assert_eq!(names.len(), tables.len());
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_e2e_drop_and_reload() {
    let dir = temp_dir("e2e_dr");

    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        fs.insert_table("temp".into(), make_table("temp")).unwrap();
        fs.insert_table("keep".into(), make_table("keep")).unwrap();
        fs.persist_table("temp").unwrap();
        fs.persist_table("keep").unwrap();
        fs.drop_table("temp").unwrap();
        fs.flush().unwrap();
    }

    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        assert!(!fs.contains_table("temp"));
        assert!(fs.contains_table("keep"));
    }

    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S3-B: BufferPool + FileStorage coherency
// ============================================================

#[test]
fn test_e2e_buffer_pool_with_file_storage() {
    let dir = temp_dir("e2e_bp");
    let pool = BufferPool::new(10);
    let mut fs = FileStorage::new(dir.join("data")).unwrap();

    // Insert table via FileStorage
    fs.insert_table("bp_test".into(), make_table("bp_test")).unwrap();
    fs.persist_table("bp_test").unwrap();

    // Use BufferPool for page caching
    for i in 0..5 {
        pool.insert(Arc::new(Page::new(i)));
    }
    assert_eq!(pool.len(), 5);

    // Both systems should be consistent
    assert!(fs.contains_table("bp_test"));
    assert!(pool.get(0).is_some());
    assert!(pool.get(4).is_some());

    fs.flush().unwrap();
    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S3-C: Index persistence across restart
// ============================================================

#[test]
fn test_e2e_index_survives_restart() {
    let dir = temp_dir("e2e_idx");

    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        fs.insert_table("idx_test".into(), make_table("idx_test")).unwrap();
        fs.create_index("idx_test", "id", 0).unwrap();
        fs.flush().unwrap();
    }

    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        assert!(fs.has_index("idx_test", "id"));
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_e2e_index_drop_across_restart() {
    let dir = temp_dir("e2e_idxd");

    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        fs.insert_table("idx2".into(), make_table("idx2")).unwrap();
        fs.create_index("idx2", "name", 0).unwrap();
        fs.drop_index("idx2", "name").unwrap();
        fs.flush().unwrap();
    }

    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        assert!(!fs.has_index("idx2", "name"));
    }

    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S3-D: Table metadata persistence
// ============================================================

#[test]
fn test_e2e_table_metadata_after_reload() {
    let dir = temp_dir("e2e_meta");

    let original_info = TableInfo {
        name: "meta_test".into(),
        ..Default::default()
    };

    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        fs.insert_table("meta_test".into(), TableData {
            info: original_info.clone(),
            rows: vec![],
        }).unwrap();
        fs.persist_table("meta_test").unwrap();
        fs.flush().unwrap();
    }

    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        let table = fs.get_table("meta_test").unwrap();
        assert_eq!(table.info.name, "meta_test");
    }

    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S3-E: Stress — many tables
// ============================================================

#[test]
fn test_e2e_many_tables() {
    let dir = temp_dir("e2e_many");
    let count = 50;

    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        for i in 0..count {
            let name = format!("table_{}", i);
            fs.insert_table(name.clone(), make_table(&name)).unwrap();
        }
        for i in 0..count {
            fs.persist_table(&format!("table_{}", i)).unwrap();
        }
        fs.flush().unwrap();
    }

    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        assert_eq!(fs.table_names().len(), count);
    }

    let _ = fs::remove_dir_all(&dir);
}

// ============================================================
// S3-F: Sequential create/write/read cycle
// ============================================================

#[test]
fn test_e2e_create_write_read_cycle() {
    let dir = temp_dir("e2e_cycle");

    // Create
    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        fs.insert_table("cycle".into(), make_table("cycle")).unwrap();
        fs.flush().unwrap();
    }

    // Verify exists
    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        assert!(fs.contains_table("cycle"));
    }

    // Drop
    {
        let mut fs = FileStorage::new(dir.join("data")).unwrap();
        fs.drop_table("cycle").unwrap();
        fs.flush().unwrap();
    }

    // Verify gone
    {
        let fs = FileStorage::new(dir.join("data")).unwrap();
        assert!(!fs.contains_table("cycle"));
    }

    let _ = fs::remove_dir_all(&dir);
}
