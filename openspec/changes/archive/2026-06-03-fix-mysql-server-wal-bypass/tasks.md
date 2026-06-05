# Tasks: 修复 mysql-server FileStorage 绕过 WAL 层 (G1)

## 1. 预分析

- [x] 1.1 验证 bug 位置：`grep -n "ExecutionEngine<FileStorage>" crates/mysql-server/src/lib.rs`
- [x] 1.2 确认 WalStorage/FileBackedWalManager 在 storage crate 中已存在
- [x] 1.3 确认 EngineBuilder::with_wal_file 已实现（src/engine_builder.rs:135）

## 2. 代码修改

- [ ] 2.1 修改 `crates/mysql-server/src/lib.rs` imports：新增 `use sqlrustgo::EngineBuilder;`、`use sqlrustgo_storage::{FileBackedWalManager, WalStorage};`
- [ ] 2.2 修改 `do_command_loop` 函数签名（line 1073-1075）：
  - `storage: Arc<RwLock<FileStorage>>` → `storage: Arc<RwLock<WalStorage<FileStorage, FileBackedWalManager>>>`
  - `engine: Arc<RwLock<ExecutionEngine<FileStorage>>>` → `engine: Arc<RwLock<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>>>`
- [ ] 2.3 修改 line 1457-1458：替换为 `let engine = EngineBuilder::with_wal_file(wal_data_dir.clone()).map_err(...)?; let engine = Arc::new(RwLock::new(engine));`
- [ ] 2.4 修改 line 1509-1510：同上模式
- [ ] 2.5 保留 line 1531-1532 的 `FileStorage::new_with_wal(wal_data_dir)` 调用（EngineBuilder 内部会调用，但需要确认其调用链与现有 wal_data_dir 一致）

## 3. 编译验证

- [ ] 3.1 `cargo check -p sqlrustgo-mysql-server --all-features` — 必须零错误
- [ ] 3.2 `cargo build -p sqlrustgo-mysql-server --all-features` — 必须成功
- [ ] 3.3 `cargo clippy -p sqlrustgo-mysql-server --all-features -- -D warnings` — 必须零警告

## 4. 测试

- [ ] 4.1 `cargo test -p sqlrustgo-mysql-server --all-features` — 所有现有测试通过
- [ ] 4.2 新增 e2e 测试 `crates/mysql-server/tests/wal_persistence_e2e.rs`：
  - 启动 server，端口 0（自动分配）
  - tokio TCP client 发送 COM_QUERY: `INSERT INTO t VALUES (1, 'x')`
  - 用 `FileBackedWalManager::list_entries(wal_data_dir)` 验证 ≥1 条 `WalEntryType::Insert`
  - 验证 `tx_id` 非 0
  - 验证 `next_lsn` 增长
- [ ] 4.3 e2e 测试覆盖 UPDATE、DELETE、COMMIT、ROLLBACK 路径
- [ ] 4.4 e2e 测试覆盖 post-crash recovery（kill -9 后重启，验证数据一致性）

## 5. 代码质量

- [ ] 5.1 `cargo fmt --all` 格式化
- [ ] 5.2 `cargo fmt --check --all` 验证无 diff
- [ ] 5.3 更新 `crates/mysql-server/src/lib.rs` 顶部 doc comment（如有），说明 do_command_loop 接收 WalStorage-wrapped 引擎

## 6. 提交与 PR

- [ ] 6.1 `git add .`
- [ ] 6.2 `git diff --staged --stat` 验证变更范围
- [ ] 6.3 `git commit -m "fix(mysql-server): wrap FileStorage in WalStorage to persist DML via WAL (#2808)"`
- [ ] 6.4 `git push -u gitea fix/issue-2808-wal-bypass`
- [ ] 6.5 Gitea API 创建 PR (head=fix/issue-2808-wal-bypass, base=develop/v3.8.0)
- [ ] 6.6 issue #2808 评论追加 PR 链接
- [ ] 6.7 PR 等待 review，merge 后 close issue #2808
- [ ] 6.8 删除本地 + 远程 fix/issue-2808-wal-bypass 分支

## 7. openspec 归档

- [ ] 7.1 PR merge 后 `openspec archive fix-mysql-server-wal-bypass`
- [ ] 7.2 验证 spec 已合并到 `openspec/specs/mysql-server-wal-persistence/spec.md`
