# openspec/3172 - SEM-1 Savepoint 真正完成 (P0-4)

> **Issue**: #3172
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 1 (W2-3)
> **工作量**: 20h

## 一、问题分析

### 1.1 SEM-1 跨版本债背景

按 `docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md §4`:
- **Status**: OPEN
- **Since**: v3.0.0 (3 版本受影响)
- **Severity**: P0 (correctness)
- **Impact**: ROLLBACK TO SAVEPOINT doesn't actually restore MVCC state

### 1.2 v3.9.0-alpha1 准入发现的实际状态

实际 (2026-06-05):
- `crates/transaction/src/savepoint.rs`: SavepointManager 完整实现 (249 行, 13 unit tests)
- `crates/parser/src/token.rs:151`: `Token::Savepoint` 已定义
- `crates/parser/src/lexer.rs:364`: "SAVEPOINT" 已识别
- `crates/parser/src/parser.rs`: **未解析** `SAVEPOINT` 语句 (Token 没用)
- `crates/parser/src/ast.rs`: **无 SavepointStatement** AST 节点
- `src/execution_engine.rs`: **无 Savepoint 路由**
- `TransactionManager` (`crates/transaction/src/transaction_manager.rs`): **未集成** SavepointManager

**结论**: SavepointManager 是孤岛 — 没人调用, 也没人解析语义. SQL `SAVEPOINT` 语句被 lexer 识别但被 parser 拒绝.

### 1.3 完整 SAVEPOINT/ROLLBACK/RELEASE 语义 (MySQL 5.7)

```sql
SAVEPOINT sp_name;
  -- Creates a savepoint sp_name in current transaction
  -- If sp_name already exists, REPLACES it (re-set undo position)

ROLLBACK TO SAVEPOINT sp_name;
  -- Rollback all DML changes made AFTER sp_name was set
  -- sp_name remains valid (so subsequent ROLLBACK TO sp_name is no-op)
  -- Savepoints nested AFTER sp_name are discarded

RELEASE SAVEPOINT sp_name;
  -- Removes sp_name from savepoint stack
  -- Undo log entries are KEPT (they may still be needed for outer savepoints)
  -- sp_name can be reused by a new SAVEPOINT sp_name
```

## 二、实施方案 (本次最小修复)

### 2.1 范围限定

按治理 §2.1 最小修改 + alpha1 准入要求:

**本次 PR 范围 (5 步)**:
1. AST: 添加 `SavepointStatement { name, op: SavepointOp }` 节点
2. Parser: 解析 3 形式 (SAVEPOINT/ROLLBACK TO/RELEASE)
3. TransactionManager: 添加 `savepoint/rollback_to/release` 方法
4. Executor: 路由 3 形式到 TransactionManager
5. G5 gate + 6 tests

**延后 (推 v3.10+)**:
- MVCC tuple-level 实际状态回滚 (本次只实现 SAVEPOINT 命名空间管理, 不实现物理回滚)
- 与 WAL 持久化集成
- 嵌套 savepoint 在崩溃后的恢复
- Autocommit 模式下的 savepoint 行为

### 2.2 AST 设计 (parser.rs)

```rust
pub enum SavepointOp {
    Save,         // SAVEPOINT name
    RollbackTo,   // ROLLBACK TO SAVEPOINT name
    Release,      // RELEASE SAVEPOINT name
}

pub enum Statement {
    // ... existing
    SavepointStatement {
        name: String,
        op: SavepointOp,
    },
}
```

### 2.3 Parser 实现 (parser.rs)

- 添加 `parse_savepoint_statement` 函数
- 在主 statement 循环中识别 `Token::Savepoint`
- 3 个分支:
  - `SAVEPOINT <name>` → Save, name = <name>
  - `ROLLBACK TO` + `SAVEPOINT` + <name> → RollbackTo
  - `RELEASE SAVEPOINT <name>` → Release

### 2.4 TransactionManager 集成 (transaction_manager.rs)

```rust
impl TransactionManager {
    pub fn savepoint(&mut self, tx_id: TxId, name: String) -> Result<(), TxError> { ... }
    pub fn rollback_to(&mut self, tx_id: TxId, name: &str) -> Result<(), TxError> { ... }
    pub fn release_savepoint(&mut self, tx_id: TxId, name: &str) -> Result<(), TxError> { ... }
}
```

每个方法:
1. 检查 tx 处于 active 状态
2. 找到 tx 的 SavepointManager (ActiveTransaction 字段)
3. 调用 SavepointManager 对应方法

### 2.5 Executor 路由 (execution_engine.rs)

```rust
match statement {
    Statement::SavepointStatement { name, op: SavepointOp::Save } => {
        self.tm.savepoint(current_tx_id, name.clone())?;
    },
    Statement::SavepointStatement { name, op: SavepointOp::RollbackTo } => {
        self.tm.rollback_to(current_tx_id, &name)?;
    },
    Statement::SavepointStatement { name, op: SavepointOp::Release } => {
        self.tm.release_savepoint(current_tx_id, &name)?;
    },
    // ... existing
}
```

### 2.6 G5 Gate

`scripts/gate/check_sem1_savepoint.sh`:
- Parser 解析 SAVEPOINT/ROLLBACK TO SAVEPOINT/RELEASE SAVEPOINT
- TransactionManager 集成 3 个方法
- 6 tests 全部 PASS
- 871 L1 tests 不回归

### 2.7 Tests (tests/sem1_savepoint_test.rs)

| Test | 验证 |
|------|------|
| test_sem1_parse_savepoint | Parser 解析 `SAVEPOINT sp1` |
| test_sem1_parse_rollback_to | Parser 解析 `ROLLBACK TO SAVEPOINT sp1` |
| test_sem1_parse_release | Parser 解析 `RELEASE SAVEPOINT sp1` |
| test_sem1_savepoint_naming | 名字唯一性 (同名覆盖 vs 拒绝) |
| test_sem1_rollback_to_notfound | ROLLBACK TO 不存在的 sp 报错 |
| test_sem1_release_nonexistent | RELEASE 不存在的 sp 不报错 (MySQL 行为) |

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| Parser 修改破坏现有 SQL | TPC-H 22/22 失败 | SAVEPOINT 是新 SQL, 旧 SQL 走 else 分支 |
| TransactionManager API 变更 | 现有事务代码破坏 | 新增方法, 不改现有方法签名 |
| Executor 路由错误 | 主查询失败 | 路由加在 statement match, 已有默认分支 |
| MVCC 物理回滚不实现 | ROLLBACK TO 仅管理命名空间 | 文档明确说明本次是命名空间管理, 物理回滚推 v3.10+ |

## 四、验收标准 (G5 门禁 - 本次范围)

```
✅ cargo check RC=0
✅ cargo test -p sqlrustgo-parser --lib: 110/110 PASS
✅ cargo test -p sqlrustgo-transaction --lib: 105/105 PASS
✅ cargo test -p sqlrustgo-storage --lib: 293/293 PASS
✅ cargo test -p sqlrustgo-executor --lib: 363/363 PASS
✅ wal_tx_contract_test: 26/26 PASS
✅ TPC-H 22/22 (G1 维持)
✅ sem1_savepoint_test: 6/6 PASS
✅ G5 gate: PASS
```

## 五、Subsumed Issues

完成后, 关闭:
- **#3106 (部分)**: SEM-1 ROLLBACK MVCC 存根 - 本次关闭命名空间管理部分

MVCC 物理回滚部分推 v3.10+ (#3106 后续追踪).

## 六、回滚计划

如 TPC-H 回归:
1. Revert commit
2. 检查 parser 主循环是否破坏
3. Re-run G1 22/22

## 七、依赖

**上游**: 无 (基础 SavepointManager 已有)
**下游**: P1-1 Backup/Restore (#3181) — 需要 Savepoint 协调

## 八、参考资料

- Issue #3172
- docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md §4
- docs/openspec/3169-arch3-vtu-main-path.md (P0-1 经验)
- docs/openspec/3170-int3-single-expression-engine.md (P0-2 经验)
- V390_DEVELOPMENT_PLAN.md §P0-4
- V390_TEST_PLAN.md §G5
- crates/transaction/src/savepoint.rs (已实现 SavepointManager 249 行)
- crates/parser/src/token.rs:151 Token::Savepoint
- crates/parser/src/lexer.rs:364 "SAVEPOINT" 识别
