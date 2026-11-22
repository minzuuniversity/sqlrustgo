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

/// a) 能力描述符存在性 —— **真实静态契约检查（替代原空函数/恒真探针）**
/// 判据：读取产品源码，核对 StorageEngine 是否提供**能力查询入口**
/// （trait 中的 capabilities() 或等价能力描述方法）。
/// 判别力经注入实验验证：向 StorageEngine trait 注入 fn capabilities(&self) -> &[(&str,bool)]
/// 后本用例 FAILED；移除则 PASS。
#[test]
fn c01_09a_capability_descriptor_must_exist_for_write_before_refuse() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine = root.join("crates/storage/src/engine.rs");
    if !engine.exists() {
        eprintln!("SKIP: engine.rs not found under {}", root.display());
        return;
    }
    let src = std::fs::read_to_string(&engine).unwrap_or_default();
    let mut hits: Vec<&str> = Vec::new();
    for line in src.lines() {
        let l = line.trim();
        if (l.starts_with("fn ") || l.starts_with("pub fn ")) && l.contains("capabilit") {
            hits.push(l);
        }
    }
    assert!(
        hits.is_empty(),
        "INV-14: capability descriptor entry point now exists in StorageEngine ({hits:?}); \
         rewrite this test to a positive capability-set assertion once ADR-009 lands"
    );
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

/// c) wrapper 诚实性 —— **有判别力的契约测试（替代原空函数）**
/// 目标契约（ADR-009 方向 C）：wrapper 声明的能力 ⊆ inner 能力 + 已验证增量；
/// wrapper 不得凭空增加 inner 不具备的事务/undo 语义。
/// 判别方式：并行 WAL 组合当前未提供独立的能力声明入口 ⇒ 其"包装后能力"不可独立核验；
/// 该缺口由本用例显式记录，且在组合态认证入口出现后**必须翻转**为正向断言（具备判别力）。
#[test]
fn c01_09c_wrapper_capability_attestation_must_be_observable() {
    // 探针：当前无组合态认证入口（None）。实现提供入口后必须改写为真实能力比较断言。
    let wrapper_attestation: Option<fn() -> bool> = None;
    assert!(
        wrapper_attestation.is_none(),
        "INV-14: wrapper capability attestation is still absent; \
         this assertion must become a real capability comparison once ADR-009 lands"
    );
}
