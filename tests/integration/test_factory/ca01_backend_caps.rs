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

// a) 能力描述符存在性 —— **缺口记录（非测试）**
// 目标契约要求后端显式声明正交能力集（capabilities() -> {transactions, durable_commit, undo, ...}）；
// 当前公共 API 无此查询入口 ⇒ "写前拒绝"没有能力事实来源。
// 说明：初版此处曾是一个空函数（无任何断言），属 AFP Type A 风险（空实现冒充覆盖），
// 已于 2026-11-21 复查中删除，缺口记录改写为本注释。ADR-009 实施时应新增真实正向断言：
// 声明的能力集与组合态认证结果一致，且不足能力在写前拒绝。

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

// c) wrapper 诚实性 —— **缺口记录（非测试）**
// 目标契约：wrapper 声明 ≤ inner 能力 + 已验证增量；parallel_wal 组合的 undo/事务能力声明
// 当前不存在于 API 表面。说明：初版此处为空函数（无断言），已于 2026-11-21 复查中删除。
// ADR-009 组合态认证矩阵实施后，应新增真实断言（wrapper 声明能力 ⊆ inner+已验证增量）。
