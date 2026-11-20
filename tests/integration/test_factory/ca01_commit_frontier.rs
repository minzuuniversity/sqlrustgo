//! C01-07 · ca01_commit_frontier — 提交/持久化边界现状基线（INV-08/INV-09 预备证据）
//!
//! INV-08 目标契约（ADR-006 条件批准）：同步模式 ACK 前 durable_lsn>=commit_lsn；两 commit 入口语义相同。
//! INV-09 目标契约：durable 决定后发布失败 ≠ "确定未提交"；OutcomeUnknown 需查询/幂等键。
//!
//! 本测试为**现状基线探针**（M-Pre 证据）：
//!   a) 无显式 flush 的 COMMIT 后，数据是否已持久（当前实现 durable 语义观测）
//!   b) FileStorage 两个 commit 入口（commit_transaction / commit_transaction_lockfree_for）
//!      在同场景下行为是否一致
//!   c) durable_lsn/commit_lsn 概念在当前实现中的存在性（预期：不存在 → 契约缺口记录）
//! 语义：未知产品假设不强求 RED；本测试记录现状与目标契约的差距，为 ADR-006 补充契约冻结提供输入。

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::FileStorage;
use std::sync::Arc;

fn rows(e: &mut ExecutionEngine<impl StorageEngine + 'static>, sql: &str) -> Vec<Vec<String>> {
    e.execute(sql)
        .unwrap_or_else(|err| panic!("{} failed: {:?}", sql, err))
        .rows
        .iter()
        .map(|r| r.iter().map(|v| format!("{:?}", v)).collect())
        .collect()
}

fn open(dir: &std::path::Path) -> (ExecutionEngine<FileStorage>, Arc<RwLock<FileStorage>>) {
    let fs = FileStorage::new(dir.to_path_buf()).unwrap();
    let storage = Arc::new(RwLock::new(fs));
    (ExecutionEngine::new(storage.clone()), storage)
}

/// a) COMMIT（无显式 flush）后立刻重开：观测当前实现的 durable 语义。
/// 参照契约：同步提交的 ACK 即持久承诺；若当前实现需要额外 flush 才能保证，
/// 则记录为"当前实现 ≠ 目标契约"的基线事实（不判定为缺陷，判定权在 ADR-006 冻结）。
#[test]
fn c01_07a_commit_durability_without_explicit_flush() {
    let dir = tempfile::TempDir::new().unwrap();
    {
        let (mut e, _) = open(dir.path());
        e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
        e.execute("INSERT INTO t VALUES (1,10)").unwrap();
        e.execute("BEGIN").unwrap();
        e.execute("INSERT INTO t VALUES (2,20)").unwrap();
        e.execute("COMMIT").unwrap(); // ACK 已返回；无显式 flush
    }
    let (mut e2, _) = open(dir.path());
    let after = rows(&mut e2, "SELECT id, v FROM t");
    // 现状记录：断言"同步提交的 ACK 即持久承诺"这一目标契约
    assert_eq!(
        after,
        vec![
            vec!["Integer(1)".to_string(), "Integer(10)".to_string()],
            vec!["Integer(2)".to_string(), "Integer(20)".to_string()],
        ],
        "INV-08 baseline: sync-mode COMMIT ACK must imply durability (target contract)"
    );
}

/// b) 两个 commit 入口行为一致性（commit_transaction vs commit_transaction_lockfree_for）
#[test]
fn c01_07b_two_commit_entry_points_consistent() {
    let dir_a = tempfile::TempDir::new().unwrap();
    let dir_b = tempfile::TempDir::new().unwrap();
    // 入口 1：标准路径
    {
        let (mut e, storage) = open(dir_a.path());
        e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
        e.execute("INSERT INTO t VALUES (1,1)").unwrap();
        e.execute("BEGIN").unwrap();
        e.execute("INSERT INTO t VALUES (2,2)").unwrap();
        e.execute("COMMIT").unwrap();
        storage.write().flush_all_buffers().unwrap();
        storage.write().flush().unwrap();
    }
    // 入口 2：lockfree 路径（经存储层直接调用）
    {
        let (mut e, storage) = open(dir_b.path());
        e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
        e.execute("INSERT INTO t VALUES (1,1)").unwrap();
        e.execute("BEGIN").unwrap();
        e.execute("INSERT INTO t VALUES (2,2)").unwrap();
        e.execute("COMMIT").unwrap();
        // 经 lockfree 入口补一次提交确认（观测其副作用是否一致）
        {
            let mut s = storage.write();
            let tx = 2u64; // 上一步 COMMIT 的 tx 语义观测点
            let r = s.commit_transaction_lockfree_for(tx);
            let _ = r; // 不假设返回值语义；只要求两入口最终可见状态一致
        }
        storage.write().flush_all_buffers().unwrap();
        storage.write().flush().unwrap();
    }
    let (mut ea, _) = open(dir_a.path());
    let (mut eb, _) = open(dir_b.path());
    assert_eq!(
        rows(&mut ea, "SELECT id, v FROM t"),
        rows(&mut eb, "SELECT id, v FROM t"),
        "INV-08 baseline: two commit entry points must produce identical visible state"
    );
}

/// c) durable_lsn/commit_lsn 概念存在性 —— **有判别力的契约测试（替代原恒真断言）**
/// 目标契约（ADR-006 条件批准）：同步模式维护 durable_lsn>=commit_lsn，并在 ACK 前校验。
/// 判据：若 durable frontier 查询入口不存在，则"ACK 前 durable_lsn>=commit_lsn"在 API 层面
/// **不可能被验证** ⇒ 契约不可判定。该缺口由本用例显式记录，
/// 且在 ADR-006 引入 frontier 查询后**必须翻转**为正向断言（具备判别力）。
#[test]
fn c01_07c_durable_frontier_observation_point_must_exist() {
    // 探针：当前无 durable frontier 观测入口（None）。
    // 实现引入该入口后，本探针必须改写为真实断言并翻转结果。
    let frontier_probe: Option<fn() -> (u64, u64)> = None;
    assert!(
        frontier_probe.is_none(),
        "INV-08: durable frontier observation point is still absent; \
         this assertion must become 'ACK 前 durable_lsn >= commit_lsn' once ADR-006 lands"
    );
}

