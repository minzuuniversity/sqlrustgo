//! Test Factory · T4 Storage 能力：非事务后端的"静默成功"检测
//!
//! 已知架构审计（docs/v410/5057_OTHER_ENGINES_AUDIT.md）声明：
//!   BinaryTableStorageV2 的 scan/delete/update 等 15 个方法是桩。
//! 本测试**验证该声明**：若这些方法返回 Ok 但实际未生效，即为"假成功"。
//!
//! 结果语义：
//!   - 测试失败 ⇒ 已复现"静默成功"缺陷（STUB_SILENT_SUCCESS）
//!   - 测试通过 ⇒ 该方法真实工作（与审计结论不符，需更新审计）

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, Value};
use tempfile::TempDir;

fn make() -> (BinaryTableStorageV2, TempDir) {
    let dir = TempDir::new().unwrap();
    let mut s = BinaryTableStorageV2::new(dir.path().to_path_buf()).unwrap();
    s.create_table("t", vec![ColumnDefinition {
        name: "id".into(),
        data_type: "BIGINT".into(),
        nullable: false,
        primary_key: true,
        ..Default::default()
    }]).expect("create_table should succeed");
    (s, dir)
}

/// T4-01：insert 后 scan 必须能看到数据。
/// 桩实现返回 Ok(vec![]) ⇒ 本测试失败 ⇒ 暴露"写入成功但读不到"。
#[test]
fn tf_t4_01_scan_after_insert_sees_rows() {
    let (mut s, _d) = make();
    s.insert("t", vec![vec![Value::Integer(1)], vec![Value::Integer(2)]])
        .expect("insert should not error");
    let rows = s.scan("t").expect("scan should not error");
    assert_eq!(rows.len(), 2,
               "STUB_SILENT_SUCCESS: insert Ok but scan returned {} rows (expected 2)",
               rows.len());
}

/// T4-02：delete 必须报告真实删除行数。
/// 桩实现恒返回 Ok(0) ⇒ 本测试失败 ⇒ 暴露"删除成功但未删"。
#[test]
fn tf_t4_02_delete_reports_real_count() {
    let (mut s, _d) = make();
    s.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
    let deleted = s.delete("t", &[]).expect("delete should not error");
    assert_eq!(deleted, 1,
               "STUB_SILENT_SUCCESS: delete Ok(0) though 1 row was inserted");
}

/// T4-03：update 必须报告真实更新行数。
#[test]
fn tf_t4_03_update_reports_real_count() {
    let (mut s, _d) = make();
    s.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
    let updated = s.update("t", &[], &[(0, Value::Integer(9))])
        .expect("update should not error");
    assert_eq!(updated, 1,
               "STUB_SILENT_SUCCESS: update Ok(0) though 1 row exists");
}

/// T4-04：非事务后端必须在 begin_transaction 上显式拒绝，而非假装成功。
#[test]
fn tf_t4_04_non_tx_backend_must_reject_or_document() {
    let (mut s, _d) = make();
    let r = s.begin_transaction();
    match r {
        Ok(_) => panic!("UNSUPPORTED_NOT_REJECTED: begin_transaction Ok on a \
                         backend whose scan/delete are stubs — must be UNSUPPORTED"),
        Err(e) => {
            let msg = format!("{e:?}").to_lowercase();
            assert!(msg.contains("unsupported") || msg.contains("not support"),
                    "rejection reason should name UNSUPPORTED, got: {msg}");
        }
    }
}
