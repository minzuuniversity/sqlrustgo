# Design: 修复 mysql-server FileStorage 绕过 WAL 层 (G1)

## Context

### 背景
`crates/mysql-server/src/lib.rs` 是 MySQL wire-protocol 服务器入口。当前代码直接用 `FileStorage` 初始化 `ExecutionEngine`，未走 `WalStorage` 包装层。这导致：
- DML（INSERT/UPDATE/DELETE）不产生 WAL 条目
- 事务 COMMIT/ROLLBACK 无法回放
- 进程崩溃后数据丢失（H-1/H-4 回归热点）

### 当前状态
- Line 1073-1075：`do_command_loop` 函数签名 `storage: Arc<RwLock<FileStorage>>`, `engine: Arc<RwLock<ExecutionEngine<FileStorage>>>`
- Line 1457-1458：handshake 后第一处构造 `ExecutionEngine::new(storage.clone())`
- Line 1509-1510：handshake 后第二处构造 `ExecutionEngine::new(storage.clone())`
- Line 1531-1532：`FileStorage::new_with_wal(wal_data_dir)` 已使用，但只是底层 storage 的 WAL 缓冲，**未被 ExecutionEngine 利用**

### 约束
- 不能破坏 `do_command_loop` 之外的 `mod tests` 测试代码（虽然有签名变化，但已有 EngineBuilder 模式可参考）
- 必须保持 `wal_data_dir` 路径语义不变（沿用 `temp_dir()/sqlrustgo_wal_{port}`）
- `mysql-server` 是无公开 API 边界的二进制 crate，签名变化影响范围小

## Goals / Non-Goals

### Goals
- ✅ mysql-server 入口的 DML 操作必须经过 `WalStorage` 包装
- ✅ 复用已有的 `EngineBuilder::with_wal_file()` 模式（src/engine_builder.rs:135）
- ✅ 保持 `do_command_loop` 的所有现有功能（prepared statements、capability flags、command dispatch）
- ✅ 通过 `cargo build -p sqlrustgo-mysql-server` 和 `cargo test -p sqlrustgo-mysql-server`

### Non-Goals
- ❌ 不实现 CheckpointManager（`with_wal_and_checkpoint` 涉及更深架构变更，留给后续）
- ❌ 不重构 `FileStorage` 自身的 WAL 行为（已有 `new_with_wal`）
- ❌ 不解决 G2（Parser MERGE）、G3（execute_merge 调用）、G4（LocalExecutor 重构）— 这些是独立 ISSUE
- ❌ 不修改 `WalStorage` 或 `FileBackedWalManager` 的内部实现

## Decisions

### Decision 1: 复用 EngineBuilder::with_wal_file()，而非手动包装 WalStorage

**选择**: 使用 `EngineBuilder::with_wal_file(data_dir)` 替代 `ExecutionEngine::new(storage.clone())`

**理由**:
- `EngineBuilder::with_wal_file` 已经正确处理了 `FileStorage::new_with_wal` + `FileBackedWalManager::new` + `WalStorage::new` 的完整初始化（src/engine_builder.rs:135-148）
- 避免代码重复（手动包装需要 3 行代码，builder 一行调用）
- 与 `EngineBuilder::with_wal_stub` / `with_wal_and_checkpoint` 形成一致的 API 表面

**备选方案**:
- ❌ 手动包装 `WalStorage::new(file_storage, FileBackedWalManager::new(wal_path))` — 重复代码，且需要手动构造 `ExecutionEngine { ... }` 字面量
- ❌ 新增 `EngineBuilder::with_wal_from_existing(file_storage)` — 范围蔓延

### Decision 2: do_command_loop 签名硬编码 WalStorage<FileStorage, FileBackedWalManager>，而非泛型化

**选择**: 显式硬编码 `WalStorage<FileStorage, FileBackedWalManager>` 类型

**理由**:
- mysql-server 是单一后端（FileStorage + FileBackedWalManager），不需要支持 MemoryStorage
- 硬编码使类型意图明确，编译期拒绝未包装的 Engine
- 避免泛型爆炸（`fn do_command_loop<E, W>...` 需多 2 个泛型参数 + trait bounds）

**备选方案**:
- ❌ `fn do_command_loop<S, W>(... engine: Arc<RwLock<ExecutionEngine<WalStorage<S, W>>>>)` — 增加复杂度但收益小
- ❌ `Arc<RwLock<dyn StorageEngine>>` + trait object — 已有尝试（`MergeExecutor::new(Arc<Mutex<dyn ExecutionEngine>>)`）导致 dispatch 困难

### Decision 3: 在 Server::run() 入口构造 WalStorage，do_command_loop 不重新创建

**选择**: 入口函数（line 1457/1509）使用 `EngineBuilder::with_wal_file(wal_data_dir)` 返回 `ExecutionEngine<WalStorage<...>>`，然后 `Arc::new(RwLock::new(...))` 包装

**理由**:
- 与 `do_command_loop` 的 `Arc<RwLock<...>>` 接收签名一致
- EngineBuilder 已处理 Arc 内部化（`storage: Arc::new(RwLock::new(wal_storage))`），我们只需外包一个 Arc

**备选方案**:
- ❌ 保留 `let file_storage = ...` 再 `let wal_storage = WalStorage::new(file_storage, wal_manager)` 模式 — 多一层包装代码

### Decision 4: 新增回归测试，使用临时端口 + 实际 mysql-client 连接

**选择**: 在 `tests/wal_persistence_e2e.rs` 集成测试中，绑定端口 → 启动 mysql-server → 用 tokio TCP client 模拟 COM_QUERY → 验证 WAL 目录产生新条目

**理由**:
- 单元测试无法验证端到端 WAL 写入（需要完整 server boot + DML 执行路径）
- `cargo test` 集成测试是项目惯例（`tests/wal_tx_contract_test/` 类似模式已存在）
- 用 `FileBackedWalManager::list_entries()` 验证 WAL 增长是直接可观测的

**备选方案**:
- ❌ 仅添加单元测试 mock StorageEngine — 无法验证真正的 WAL 写入
- ❌ 添加 e2e Shell 脚本 — 增加 CI 复杂度

## Risks / Trade-offs

| 风险 | 缓解 |
|------|------|
| `do_command_loop` 签名变更可能影响 `mod tests` 内的局部测试 | 测试代码中如有局部调用，更新为新签名（grep `do_command_loop` 全局验证） |
| `EngineBuilder::with_wal_file` 内部不使用 CheckpointManager，checkpoint 集成需后续 PR | 标注在提案 Non-Goals 中，issue 评论里说明 |
| `wal_data_dir` 在 `temp_dir()` 中跨重启可能丢失 | 这是 mysql-server 既有行为，ISSUE #2808 未要求持久化路径 |
| `MySQL wire protocol` 的 `COM_STMT_PREPARE` / `COM_STMT_EXECUTE` 路径可能也走 `do_command_loop`，需验证 | 通过 e2e 测试覆盖 prepared statement 路径 |

## Migration Plan

### Deploy
1. 合并 PR → `develop/v3.8.0`
2. CI 通过（`cargo build --all-features` + `cargo test --all-features`）
3. PR review 通过（G4 重构前需要有人 sign-off 此 G1 修复）

### Rollback
- 单 commit revert
- 不会破坏 schema 或 data 格式（仅实例化方式变化）

## Open Questions

- **Q1**: 是否需要在 `do_command_loop` 内显式调用 `recovery` 启动时的 WAL replay？  
  现状：recovery 在 main.rs 启动时单独跑（如果存在）
  建议：保持当前架构，do_command_loop 只负责运行时 WAL 写入

- **Q2**: `MySQL 8.0` 协议中 `COM_STMT_SEND_LONG_DATA` 等高级特性是否依赖其他 storage 行为？  
  现状：grep 表明 do_command_loop 是统一入口
  建议：e2e 测试覆盖 prepared statement 路径

- **Q3**: 是否需要添加 `#[cfg(test)]` 兼容代码，避免 `do_command_loop` 在测试中签名冲突？  
  现状：mod tests 直接调用 `do_command_loop` 较少
  建议：实施时如发现问题再处理
