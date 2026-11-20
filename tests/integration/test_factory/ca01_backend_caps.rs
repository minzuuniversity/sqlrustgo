//! C01-09 · ca01_backend_caps — Storage 能力契约现状基线（INV-14 预备证据）
//!
//! INV-14 目标契约（ADR-009 批准方向 C：正交能力集+组合态认证）：
//!   1. capability 不足 ⇒ 写前拒绝（先声明能力，不足即拒，不返回空成功）；
//!   2. wrapper 不凭空增加 inner 的回滚能力（WAL 可增 durability 但需完整协议）。
//! 语义：未知产品假设不强求 RED；差距记录供 ADR-009 冻结补充契约输入。

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine};
use tempfile::TempDir;

fn make_binary() -> (BinaryTableStorageV2, TempDir) {
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

/// a) 能力描述符存在性：目标契约要求后端显式声明正交能力集。
/// 当前公共 API 无 capabilities() 查询入口 ⇒ "写前拒绝"没有能力事实来源。
/// 缺口记录（非缺陷判定）；实现后改造为正向断言。
#[test]
fn c01_09a_capability_descriptor_absence_documented() {
    // 目标：capabilities() -> {transactions, durable_commit, undo, ...}
    // 现状：API 表面缺失（结构化记录，供 ADR-009 组合态认证矩阵输入）
}

/// b) INV-14 正向判据（现状基线）：事务能力不足的后端写前必须拒绝。
/// 当前已知（D-004 族，#5914）：返回 Ok——违反"写前拒绝"。
#[test]
fn c01_09b_insufficient_capability_must_refuse_before_write() {
    let (mut s, _d) = make_binary();
    match s.begin_transaction() {
        Ok(_) => panic!(
            "INV-14 baseline gap: non-tx backend accepted begin_transaction \
             (write-before-refuse absent; D-004 family, tracked in #5914)"
        ),
        Err(e) => {
            let msg = format!("{e:?}").to_lowercase();
            assert!(
                msg.contains("unsupported") || msg.contains("not support"),
                "rejection must name capability insufficiency, got: {msg}"
            );
        }
    }
}

/// c) wrapper 诚实性：组合态认证入口缺失记录（目标契约：wrapper 声明 ≤ inner 能力+已验证增量）
#[test]
fn c01_09c_wrapper_capability_honesty_observed() {
    // 观测：parallel_wal 组合的 undo/事务能力声明不存在于 API 表面；
    // ADR-009 目标 C 的组合态认证矩阵实施后，本测试改造为正向断言。
}
