//! Ignored measurement: how long does an explicit `flush()` hold the internal
//! `with_write_lock`? Run against pre-B2.2 and post-B2.2 builds.
use sqlrustgo_storage::mvcc_storage::MvccStorage;
use sqlrustgo_storage::{ColumnDefinition, FileStorage, Record, StorageEngine, TableInfo, Value};
use std::time::Instant;

fn table_info(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "a".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

#[test]
#[ignore]
fn b22_flush_wall_time() {
    let dir = tempfile::tempdir().unwrap();
    let inner =
        FileStorage::new_with_buffer_config(dir.path().to_path_buf(), usize::MAX, true).unwrap();
    let mut storage = MvccStorage::new(inner);
    storage.create_table(&table_info("t")).unwrap();

    let prefill = 20_000usize;
    for chunk in 0..prefill {
        storage
            .insert(
                "t",
                vec![vec![
                    Value::Integer(chunk as i64),
                    Value::Text("x".repeat(32)),
                ]],
            )
            .unwrap();
    }
    storage.flush().unwrap();

    // 20 rounds, each writing a 2000-row delta
    let mut lats = Vec::new();
    for r in 0..20 {
        let base = (prefill + r * 2000) as i64;
        let rows: Vec<Record> = (0..2000)
            .map(|i| {
                vec![
                    Value::Integer(base + i as i64),
                    Value::Text(format!("y{}", "z".repeat(48))),
                ]
            })
            .collect();
        storage.insert("t", rows).unwrap();
        let t0 = Instant::now();
        storage.flush().unwrap();
        lats.push(t0.elapsed().as_secs_f64() * 1e3);
    }
    lats.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "B22_FLUSH_WALL n={} p50={:.3}ms mean={:.3}ms max={:.3}ms",
        lats.len(),
        lats[lats.len() / 2],
        lats.iter().sum::<f64>() / lats.len() as f64,
        lats[lats.len() - 1],
    );
}
