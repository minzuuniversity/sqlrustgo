# ADR-007: WAL Architecture Clarification

## Status

**Accepted** — v3.8.0 GA (2026-06-03)

## Context

v3.8.0 整合了 WAL 核心层（PR-830A~E, PR-2755, PR-2756, PR-2758, PR-2760, PR-2764），
但 4 个核心 WAL 架构问题在多个 PR 中未明确决策，遗留为 ISSUE-2742 阻塞 GA。

这 4 个问题的答案决定：
- DDL 一致性行为
- 应用层 WAL 状态可观测性
- 生产 vs 迁移场景的灵活性
- v3.9 MVCC 基础

本 ADR 决策这 4 个问题，为 F-06 (TransactionalFacade) / F-09 (DML 截获) / v3.9
MVCC 提供架构基础。

## Investigation Findings (2026-06-03)

通过 `git grep` 调研 4 个问题在当前代码中的状态：

### Finding-1: DDL 操作当前完全 bypass WAL

```
$ grep -rn "create_table\|drop_table" --include="*.rs" crates/executor/src/

crates/executor/src/local_executor.rs:1496  .create_table(...)
crates/executor/src/local_executor.rs:2422  .create_table(...)
crates/executor/src/local_executor.rs:2444  .create_table(...)
crates/executor/src/local_executor.rs:2536  .create_table(...)
crates/executor/src/parallel_executor.rs:1102  .create_table(...)
crates/executor/src/parallel_executor.rs:1216  .create_table(...)
crates/executor/src/parallel_executor.rs:1344  .create_table(...)
... (15+ DDL calls bypass WAL)
```

**结论**：所有 CREATE TABLE / DROP TABLE 直接调用 `storage.create_table()`，
不经任何 WAL log。`ddl_executor.rs` 仅处理用户/权限（CREATE USER / GRANT），
不处理表 DDL。

### Finding-2: `is_wal_enabled()` 私有且 hardcode `true`

```
$ grep -rn "is_wal_enabled" --include="*.rs" crates/storage/src/wal_storage.rs

570:    fn is_wal_enabled(&self) -> bool {
571:        true
572:    }
```

**结论**：
- 函数可见性为 `fn`（私有）而非 `pub fn`
- 函数体 hardcode 返回 `true`
- 唯一调用方是 `wal_storage.rs:628` 的 test
- 应用层完全无法查询 WAL 状态

### Finding-3: Bypass 概念已存在于 SGL-005 治理

```
$ grep -rn "bypass" --include="*.rs" crates/

crates/executor/src/trigger.rs:97:
    // P1 FIX (SGL-005): TriggerExecutor storage bypasses wrapped in transaction
```

**结论**：SGL-005 审计已将"绕过事务的 storage 直调"识别为 P1 违规，
但仅在 DML (INSERT/UPDATE/DELETE) 范围治理。DDL bypass 未列入 SGL-005 范围。

### Finding-4: Read-only 模式部分存在

```
$ grep -rn "read_only\|readonly" --include="*.rs" crates/

crates/transaction/src/manager.rs:24:    pub read_only: bool
crates/transaction/src/manager.rs:106:    pub fn begin_read_only()
crates/executor/src/sql_executor.rs:84:
    pub fn is_read_only_statement(stmt: &Statement) -> bool
```

**结论**：
- `TransactionManager` 支持 `begin_read_only()`（基础设施存在）
- `is_read_only_statement` 能识别 SELECT/SHOW/DESCRIBE（语句分类存在）
- 但**没有任何代码路径**对 read-only transaction 的 DML 行为做强制检查
- read-only 模式当前等价于无

## Decision

### Decision-1: DDL 走 WAL（DDL log）

**选项 B**：所有 DDL 操作（CREATE TABLE / DROP TABLE / CREATE INDEX / ALTER TABLE）
必须写 WAL log。

**理由**：
1. **Crash Recovery 一致性**：当前 DDL 直写磁盘，进程崩溃后无 log 可重放，
   破坏 WAL 的 atomicity 保证
2. **Issue-2740 E-2 隐患**：`crash_recovery_test.rs` 仍用 MemoryStorage 测，
   DDL 持久化证据缺失——DDL WAL 是修复 Issue-2740 的前置条件
3. **触发器持久化一致**：`feat(storage): FileStorage trigger persistence`
   (c8ee597ba) 已让 CREATE TRIGGER 持久化，但 CREATE TABLE 时机未走 WAL，
   触发器元数据可能丢失

**实施路径**：
- v3.8.0+1: DDL executor → `wal.log_ddl()` → 存储引擎
- v3.9: DDL log 进入 checkpoint truncation
- 长期: DDL 复用 `begin_transaction()` / `commit_transaction()` 接口

**例外**：system catalog 初始化（`init_system_catalog`）不走 WAL，
作为冷启动的 seed 逻辑。

### Decision-2: `is_wal_enabled()` 公开

**选项 A**：将 `is_wal_enabled()` 提升为 `pub fn`，允许应用层查询。

**理由**：
1. **可观测性**：运维工具、监控、调试需要知道当前是否启用 WAL
2. **Health Check**：应用启动时检查 WAL 是否正确接入（避免静默 fallback）
3. **未来 Readonly 模式支持**：决策-4 需要依据 WAL 状态决定 DML 行为

**实施**：
```rust
impl<S: StorageEngine, T: WalManager> WalStorage<S, T> {
    pub fn is_wal_enabled(&self) -> bool {
        true
    }
}
```

**未决策的扩展**（留待 v3.9 决策）：
- `pub fn is_wal_disabled()` 用于反向检测
- `pub fn wal_status() -> WalStatus` 返回 enum 状态

### Decision-3: DDL bypass 策略——生产拒绝 / 迁移允许

**选项 B 细化**：
- **生产模式**（`SQLRUSTGO_MODE=production` 或无环境变量）：所有 storage
  直调（bypass）由编译器警告 + 运行时审计
- **迁移模式**（`SQLRUSTGO_MODE=migration`）：允许绕过 WAL 的直调，
  用于 offline migration 工具

**理由**：
1. **离线迁移需求**：从 SQLite / MySQL 导入数据时需要 bulk load，
   走 WAL 太慢
2. **生产一致性**：默认拒绝确保 DML 都经过事务
3. **可观测**：通过环境变量显式启用，不隐式

**实施**：
```rust
// crates/executor/src/local_executor.rs
fn create_table_direct(&mut self, info: TableInfo) -> SqlResult<()> {
    if std::env::var("SQLRUSTGO_MODE").as_deref() == Ok("migration") {
        self.storage.create_table(&info)?;
    } else {
        // Wrap in transaction (which goes through WAL)
        self.txn.begin_transaction()?;
        self.storage.create_table(&info)?;
        self.txn.commit_transaction()?;
    }
    Ok(())
}
```

**治理**：
- SGL-005 审计脚本增加 migration mode 检测
- CI 默认 production mode 跑测试

### Decision-4: Read-only 模式行为——DML 返回 Err

**选项 A**：read-only transaction 内的任何 DML 返回 `TransactionReadOnly` Err。

**理由**：
1. **数据库语义标准**：PostgreSQL/MySQL 在 read-only transaction 内
   执行 INSERT/UPDATE/DELETE 均返回错误
2. **MVCC 一致性**：v3.9 MVCC 依赖 read-only tx 作为 snapshot 隔离基础
3. **Issue-2743 WAL Recovery 8 个 gap 中至少 2 个依赖此决策**：
   - RECOVERY-006/007/008（read consistency under recovery）

**实施**：
```rust
// crates/transaction/src/manager.rs
pub fn execute_dml(&mut self, stmt: &Statement) -> Result<...> {
    if self.read_only && is_dml_statement(stmt) {
        return Err(TransactionError::ReadOnly);
    }
    // ... normal path
}
```

**API**：
- `BEGIN READ ONLY` 启动 read-only transaction
- 普通 `BEGIN` 默认 read-write
- 错误类型：`TransactionError::ReadOnly(String)`

**未决策的扩展**（留待 v3.9）：
- Session-level read-only 模式
- User/role 级别权限控制

## Consequences

### Positive

1. **WAL 完整性** — 所有 mutation（DDL + DML）经 WAL，crash recovery 一致
2. **可观测性** — `is_wal_enabled()` 公开，应用层可检测
3. **生产安全** — 默认 production mode 强制事务包裹
4. **MVCC 基础** — read-only 模式行为明确，v3.9 可基于此实现 snapshot

### Negative

1. **DDL WAL 工作量** — 需新增 `wal.log_ddl()` 接口 + 现有 DDL 调用点改造
2. **migration 工具改造** — 需检测环境变量，文档化使用方式
3. **backward compatibility** — 旧测试代码可能直调 storage，
   需在 `SQLRUSTGO_MODE=migration` 下兼容

### Risks

- **R-1**：DDL WAL 改造触发 SGL-005 新违规（需审计时同步修复）
- **R-2**：`is_wal_enabled()` 公开后被误用为 control flow（应仅作 status query）
- **R-3**：read-only enforcement 破坏现有 autocommit DML（需在 F-09 完整 DML 截获时同步）

## Implementation Roadmap

| 任务 | 优先级 | 关联 |
|------|--------|------|
| Decision-1 DDL WAL | Task #2746 (F-09 完整 DML) 的扩展 | F-09 |
| Decision-2 `is_wal_enabled()` 公开 | Task #2744 (本 ADR) + minor code | 本 ADR |
| Decision-3 DDL bypass 策略 | Task #2746 + Task #2745 (F-06 STUB) | F-06/F-09 |
| Decision-4 Read-only 行为 | Task #2751 (v3.8.0+1 计划) | v3.8.0+1 |

**预计工作量**：
- Decision-1: 2 天 (DDL executor 改造 + WAL log DDL 接口)
- Decision-2: 半天 (1 行 `pub` + 测试)
- Decision-3: 1 天 (环境变量 + bypass 检测)
- Decision-4: 1 天 (DML 拦截 + 错误类型)

## Related

- **Parent issue**: #2769 (Task #2744)
- **Source ISSUE**: ISSUE-2742 (WAL Architecture Clarification)
- **依赖 PRs**:
  - PR-830A~E (WAL 核心层)
  - PR-2755 (PR-842 UPDATE replay 架构修复)
  - PR-2756 (PR-830F WAL lifecycle)
  - PR-2758 (SPEC-003 WAL replay encode updates)
  - PR-2760 (WAL 集成)
  - PR-2764 (F-09 deep fixes)
- **Blocked by**: 无
- **Blocks**:
  - F-06 TransactionalFacade (Task #2745)
  - F-09 完整 DML 截获 (Task #2746)
  - F-07~F-15 Ghost PR 决策 (Task #2749)
  - v3.8.0+1 Contract Gaps (Task #2751)
- **References**:
  - `docs/audit/issues/ISSUE-2742_wal_architecture_clarification.md`
  - `docs/releases/v3.8.0/RC_GA_GATE_REPORT.md`
  - `docs/releases/v3.8.0/SGL-005_STORAGEBYPASS_AUDIT.md`
  - `docs/audit/V380_RECTIFICATION_PLAN_2026-06-03.md` §3.1.A

## Change History

| 版本 | 日期 | 作者 | 说明 |
|------|------|------|------|
| 1.0 | 2026-06-03 | claude-macmini (architect) | 初始版本：4 个 WAL 架构决策 |
