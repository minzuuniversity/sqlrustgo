# Proposal: 修复 mysql-server FileStorage 绕过 WAL 层 (Issue #2808, G1)

## Why

`crates/mysql-server/src/lib.rs` 在 `do_command_loop`（line 1077）和两处入口（lines 1457、1509）直接使用 `ExecutionEngine<FileStorage>`，导致 DML 操作（INSERT/UPDATE/DELETE）绕过 `WalStorage` 封装层，无法保证事务持久化和 ACID。`FileStorage::new_with_wal` 仅启用存储层 WAL 缓冲，但 `ExecutionEngine::new(FileStorage)` 实例化时未走 `WalStorage` 包装，使事务原子性失效（H-1/H-4 回归热点未根治）。

追踪 ISSUE: #2807（v3.8.0 历史遗留问题全面评估）

## What Changes

- **BREAKING**: `do_command_loop` 函数签名中 `storage` / `engine` 参数类型从 `Arc<RwLock<FileStorage>>` / `Arc<RwLock<ExecutionEngine<FileStorage>>>` 变更为 `Arc<RwLock<WalStorage<FileStorage, FileBackedWalManager>>>` / `Arc<RwLock<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>>>`
- 入口 A（line 1457）：使用 `EngineBuilder::with_wal_file(wal_data_dir)` 替换 `ExecutionEngine::new(storage.clone())`
- 入口 B（line 1509）：同上
- 入口 C（line 1531-1532 附近）：保留 `FileStorage::new_with_wal` 作为底层存储初始化，但将 `file_storage` 包装为 `WalStorage::new(file_storage, FileBackedWalManager::new(wal_path))`
- 新增回归测试：验证 DML 操作在 mysql-server 端确实生成 WAL 条目

## Capabilities

### New Capabilities

- `mysql-server-wal-persistence`: mysql-server 入口的所有 DML 操作必须经过 `WalStorage` 包装层，确保 WAL 写入与事务原子性

### Modified Capabilities

（无现有 spec 涉及此行为变更）

## Impact

### 代码影响
- `crates/mysql-server/src/lib.rs`:
  - 第 1073-1075 行：`do_command_loop` 函数签名（2 个参数类型变更）
  - 第 1457-1458 行：handshake 后第一处引擎构造
  - 第 1509-1510 行：handshake 后第二处引擎构造
  - 第 1531-1532 行：FileStorage 初始化 → 包装为 WalStorage
  - imports: 新增 `WalStorage`、`FileBackedWalManager`、`EngineBuilder`

### 依赖影响
- `sqlrustgo-storage` 已暴露 `WalStorage` / `FileBackedWalManager`（无需新增）
- `sqlrustgo::engine_builder::EngineBuilder::with_wal_file` 已存在

### API 影响
- `do_command_loop` 是私有函数（无公开 API 破坏）
- 但这是 `mod tests` 中所有测试都会经过的路径，回归测试必须覆盖

### 风险
- **低**：改动仅在 mysql-server 内部，未涉及公开 API
- 已有 `EngineBuilder::with_wal_file` 和 `with_wal_and_checkpoint` 验证过此模式（src/engine_builder.rs:135-148）
