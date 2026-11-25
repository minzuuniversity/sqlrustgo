```rust
use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, Value};
use tempfile::TempDir;

fn make() -> (BinaryTableStorageV2, TempDir) {
    let dir = TempDir::new().unwrap();
    let mut s = BinaryTableStorageV2::new(dir.path().to_path_buf()).unwrap();
    s.create_table(
        "t",
        vec![ColumnDefinition {
            name: "id".into(),
            data_type: "BIGINT".into(),
            nullable: false,
            primary_key: true,
            ..Default::default()
        }],
    )
    .expect("create_table should succeed");
    (s, dir)
}

// ─── 1) insert 两行后 scan 必须看到数据 ───────────────────────────────────────

#[test]
fn test_insert_then_scan_detects_stub() {
    let (mut s, _dir) = make();

    let ins1 = s
        .insert("t", &[Value::BigInt(1)])
        .expect("insert row id=1 should not error");
    let ins2 = s
        .insert("t", &[Value::BigInt(2)])
        .expect("insert row id=2 should not error");

    // 如果 insert 返回 Ok 但影响行数为 0，也是桩
    assert!(
        ins1 >= 1 && ins2 >= 1,
        "STUB_SILENT_SUCCESS: insert returned Ok but affected_rows=({ins1},{ins2}) — no real write"
    );

    let rows = s.scan("t").expect("scan should not error");
    assert_eq!(
        rows.len(),
        2,
        "STUB_SILENT_SUCCESS: insert reported Ok but scan sees {} rows (expected 2) — data not persisted",
        rows.len()
    );

    // 验证内容
    let ids: Vec<i64> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::BigInt(v) => *v,
            other => panic!("unexpected value type: {other:?}"),
        })
        .collect();
    assert!(ids.contains(&1) && ids.contains(&2), "STUB_SILENT_SUCCESS: scan rows do not contain inserted ids {ids:?}");
}

// ─── 2) delete 返回值必须反映真实删除数 ──────────────────────────────────────

#[test]
fn test_delete_returns_real_count() {
    let (mut s, _dir) = make();

    s.insert("t", &[Value::BigInt(10)]).unwrap();
    s.insert("t", &[Value::BigInt(20)]).unwrap();
    s.insert("t", &[Value::BigInt(30)]).unwrap();

    // 删除 id=20
    let deleted = s
        .delete("t", &[Value::BigInt(20)])
        .expect("delete should not error");
    assert_eq!(
        deleted, 1,
        "STUB_SILENT_SUCCESS: delete returned {deleted} but expected 1 — no real deletion"
    );

    // 确认 scan 只剩 2 行
    let rows = s.scan("t").unwrap();
    assert_eq!(
        rows.len(),
        2,
        "STUB_SILENT_SUCCESS: after delete, scan still sees {} rows (expected 2)",
        rows.len()
    );

    // 删除不存在的行应返回 0
    let deleted_missing = s
        .delete("t", &[Value::BigInt(999)])
        .expect("delete non-existent should