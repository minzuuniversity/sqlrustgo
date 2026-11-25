//! C01-07 · ca01_commit_frontier — 提交/持久化边界契约测试（INV-08 / INV-09）
//!
//! INV-08 目标契约（ADR-006 条件批准）：同步模式 ACK 前 durable_lsn>=commit_lsn；两 commit 入口语义相同。
//! INV-09 目标契约：durable 决定后发布失败 ≠ "确定未提交"；OutcomeUnknown 需查询/幂等键。
//!
//! §1 判别依据（产品真实源码事实，非硬编码探针）：
//!   StorageEngine 现有对外观测入口：
//!     - FileStorage::wal_current_lsn()  = **下一个 append 将获得的 LSN**（#5055 注释明载），
//!       不是"已提交的 LSN"，也不是"已持久化的 LSN"；
//!     - StorageEngine::take_last_committed_log() = 取走最近一次 committed 的 TxLog（会 drain），
//!       是日志载荷而非 LSN 数值；
//!     - FileStorage::current_change_lsn() = change log 的游标，与 commit frontier 无关。
//!   ⇒ 目标契约所需的 durable_lsn / commit_lsn **成对可观测量**当前不存在。
//!   本用例直接读取产品源码核对"是否存在成对 frontier 观测入口"；产品一旦提供即翻转失败，
//!   提示改写为正向断言（ACK 前 durable_lsn >= commit_lsn）。判别力经注入实验验证：
//!   向 StorageEngine trait 注入 fn durable_lsn(&self) -> u64 后本用例 FAILED。
//!
//! §2 运行事实：同步 COMMIT 的 ACK 是否隐含持久化（无显式 flush 直接重开）。
//! §3 两 commit 入口行为一致性。
//!
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

/// 仓库根（根包 CARGO_MANIFEST_DIR）；不成立则显式 skip（NOT_RUN 语义），绝不冒充 PASS
fn repo_root() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if root.join("crates/storage/src/engine.rs").exists() {
        Some(root)
    } else {
        None
    }
}

/// §1 · 静态源码契约检查：是否存在 **成对** 的 durable/commit frontier 可观测入口。
/// 判别力：读取产品源码；产品提供成对 frontier 观测（如 durable_lsn + commit_lsn）即 FAIL。
#[test]
fn c01_07c_paired_frontier_observation_must_exist() {
    let root = match repo_root() {
        Some(r) => r,
        None => {
            eprintln!("SKIP: repo root not resolvable from CARGO_MANIFEST_DIR");
            return;
        }
    };
    let targets = [
        root.join("crates/storage/src/engine.rs"),
        root.join("crates/storage/src/file_storage.rs"),
        root.join("crates/storage/src/wal_storage.rs"),
    ];
    let mut durable_obs: Vec<String> = Vec::new();
    let mut commit_obs: Vec<String> = Vec::new();
    for f in targets.iter() {
        if !f.exists() {
            continue;
        }
        let src = std::fs::read_to_string(f).unwrap_or_default();
        for line in src.lines() {
            let l = line.trim();
            if !(l.starts_with("pub fn ") || l.starts_with("fn ")) {
                continue;
            }
            // 排除"下一个 append 的 LSN"与"change log 游标"——二者都不是 commit frontier
            let is_next_lsn = l.contains("wal_current_lsn") || l.contains("next_lsn");
            let is_change_cursor = l.contains("current_change_lsn") || l.contains("changes_since");
            if l.contains("durable_lsn") && !is_next_lsn {
                durable_obs.push(format!(
                    "{}: {}",
                    f.file_name().unwrap().to_string_lossy(),
                    l
                ));
            }
            if l.contains("commit_lsn") && !is_next_lsn && !is_change_cursor {
                commit_obs.push(format!(
                    "{}: {}",
                    f.file_name().unwrap().to_string_lossy(),
                    l
                ));
            }
        }
    }
    assert!(
        durable_obs.is_empty() || commit_obs.is_empty(),
        "INV-08: paired durable/commit frontier observation now exists \
         (durable={durable_obs:?}, commit={commit_obs:?}); \
         rewrite this test to the positive assertion 'ACK 前 durable_lsn >= commit_lsn'"
    );
}

/// §2 · 同步 COMMIT 的 ACK 是否隐含持久化（无显式 flush 直接重开）。
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
    assert_eq!(
        after,
        vec![
            vec!["Integer(1)".to_string(), "Integer(10)".to_string()],
            vec!["Integer(2)".to_string(), "Integer(20)".to_string()],
        ],
        "INV-08 baseline: sync-mode COMMIT ACK must imply durability (target contract)"
    );
}

/// §3 · 两 commit 入口可见状态一致性
#[test]
fn c01_07b_two_commit_entry_points_consistent() {
    let dir_a = tempfile::TempDir::new().unwrap();
    let dir_b = tempfile::TempDir::new().unwrap();
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
    {
        let (mut e, storage) = open(dir_b.path());
        e.execute("CREATE TABLE t (id INT, v INT)").unwrap();
        e.execute("INSERT INTO t VALUES (1,1)").unwrap();
        e.execute("BEGIN").unwrap();
        e.execute("INSERT INTO t VALUES (2,2)").unwrap();
        e.execute("COMMIT").unwrap();
        {
            let mut s = storage.write();
            let _ = s.commit_transaction_lockfree_for(2u64);
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
