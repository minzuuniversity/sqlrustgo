# SQLRustGo 集成债务分析报告

> **报告日期**: 2026-05-30
> **分析范围**: v1.0.0 ~ v3.6.0
> **分析方法**: 仓库结构分析 + git 代码考古 + Issue 交叉验证
> **更新状态**: 初稿

---

## 一、执行摘要

SQLRustGo 存在 **跨版本集成债务**，10 个版本（v1.2.0 ~ v3.6.0）积累下 4 个核心集成缺陷：

| ID | 缺陷 | 跨度 | 状态 |
|----|------|------|------|
| INT-1 | DML 不经过 WAL/TransactionManager | v1.2.0~v3.6.0（6版本） | 开放 |
| INT-2 | ParallelVolcanoExecutor 功能孤岛 | v2.6.0~v3.6.0（4版本） | 开放 |
| INT-3 | expr crate 孤岛 | v3.0.0~v3.6.0（2版本） | 新发现 |
| INT-4 | mysql-server 未与主 server 集成 | v2.6.0~v3.6.0（4版本） | 开放 |

**核心根因**：执行路径分裂（双路径并存）+ 存储层与事务层从未连接。

---

## 二、根因：双执行路径

```
Path 1: PhysicalPlan Pipeline（主协议栈）
  Client → COM_QUERY → server/lib.rs:1106 MemoryExecutionEngine
                          → execute_select/execute_write
                          → LocalExecutor
                          → StorageEngine
                          ❌ 不经过 WAL / TransactionManager

Path 2: Direct Execution Path（bench-cli / benthos）
  bench-cli → ExecutionEngine → MemoryStorage
                          ❌ 绕过事务层，直接操作 MemoryStorage
```

**当前 TPC-H 实测路径**：Path 2（bench-cli 直调 ExecutionEngine），这是为什么 TPC-H 能工作但 DML 没有事务保护的原因。

---

## 三、四大集成缺陷详解

### INT-1：DML 不经过 WAL（v1.2.0 ~ v3.6.0）

**持续时间**：6 年+，跨越 10 个版本

#### 历史演进

| 版本 | Transaction 模块 | WAL 模块 | 集成状态 |
|------|-----------------|---------|---------|
| v1.2.0 | 空壳（1行注释） | 不存在 | ❌ 分离 |
| v2.0.0 | 有 mvcc/coordinator/recovery | storage/wal.rs（2129行）完整 | ❌ 分离 |
| v3.0.0 | 有 SSI（SerializationGraph） | storage/engine.rs 无事务方法 | ❌ 分离 |
| v3.6.0 | 同 v3.0.0 | 同 v3.0.0 | ❌ 分离 |

#### 代码证据

**StorageEngine trait 缺少事务方法**：
```rust
// crates/storage/src/engine.rs - v3.0.0~v3.6.0 相同
pub trait StorageEngine: Send + Sync {
    fn scan(&self, table: &str) -> SqlResult<Vec<Record>>;
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()>;
    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize>;
    fn update(...) -> SqlResult<usize>;
    // ❌ 缺少: begin_transaction / commit / rollback / WAL 方法
}
```

**storage/wal 和 transaction/mvcc 完全隔离**：
```bash
# storage/wal 存在
$ git show origin/archive/develop/v2.0.0:crates/storage/src/lib.rs
pub mod wal;  // ✅ 完整实现

# 但 storage/engine.rs 无任何 WAL 相关代码
$ git grep "wal\|WAL" origin/archive/develop/v3.0.0/crates/storage/src/engine.rs
# 无任何结果 ❌
```

#### 对应 Issue

- **#2576**：`DML 操作不经过 TransactionManager/WAL`（v3.6.0 Alpha Issue）
- **#2571**：`WAL/MVCC/TransactionManager DML 集成缺失`
- **#2579**：`gate_spec 缺少 I-Gate 集成路径检查`（6 版本未修复）

---

### INT-2：ParallelVolcanoExecutor 功能孤岛（v2.6.0 ~ v3.6.0）

**持续时间**：4 版本（v2.6.0, v3.0.0, v3.2.0~v3.6.0）

#### 代码证据

```bash
# v3.6.0: ParallelVolcanoExecutor 仅被以下位置引用
$ git grep -r "ParallelVolcanoExecutor\|ParallelExecutor" crates/ \
  --include="*.rs" | grep -v "test\|tests\|unified-query"
# 无结果 → 主执行路径从未调用

# 仅存在于:
# - 44 个单元测试文件
# - unified-query/engine.rs（唯一非测试调用者）
```

#### 历史状态

| 版本 | PVE 状态 | 调用者 |
|------|----------|--------|
| v2.6.0 | 出现 | 仅 tests + unified-query |
| v3.0.0 | 同一问题 | 仅 unified-query/engine.rs |
| v3.2.0 | 同一问题 | 仅 unified-query |
| v3.6.0 | 同一问题 | 仅 44 单元测试文件 |

#### 对应 Issue

- **#2570**：`ParallelVolcanoExecutor 未集成到主执行链路`
- **#2577**：`ParallelVolcanoExecutor 孤岛`（44 单元测试但从未调用，跨越 v3.1.0~v3.6.0）

---

### INT-3：expr crate 孤岛（v3.0.0 ~ v3.6.0）

**发现时间**：v3.0.0（2026-05-30 分析新发现）

#### 代码证据

```bash
# expr 模块结构完整
$ git ls-tree origin/archive/develop/v3.0.0 crates/expr/src/
eval.rs / expr.rs / lib.rs / op.rs

# 但无任何 crate 导入 expr
$ git grep "use sqlrustgo_expr\|sqlrustgo_expr::" \
  origin/archive/develop/v3.0.0/crates/*/src/*.rs
# 无任何结果 → expr crate 从未被使用
```

#### 影响

- executor 内联了表达式求值逻辑（重复代码）
- expr crate 的类型系统和求值优化无法被利用
- 每次表达式变更需要在多处维护

---

### INT-4：mysql-server 未集成（v2.6.0 ~ v3.6.0）

**持续时间**：4 版本

#### 代码证据

```rust
// crates/mysql-server/src/lib.rs - 完整的 COM_QUERY 处理
// 行 1106: MemoryExecutionEngine 创建
// 行 1109/1130: execute_select/execute_write 调用

// 但 server/src/lib.rs 创建的是另一个独立实现
// 两者是完全独立的协议栈，从未互相调用
```

#### 对应 Issue

- **#2583**：`DML 执行路径统一到 PhysicalPlan pipeline`

---

## 四、历史孤岛 Crate 汇总（v1.0.0 ~ v3.0.0）

| 版本 | 孤岛 Crate | 证据 | 持续状态 |
|------|-----------|------|---------|
| v1.2.0 | `transaction` | lib.rs 仅1行注释 | 全版本持续 |
| v2.0.0 | `distributed` | 无其他 crate 导入 | — |
| v2.0.0 | `sqlancer` | 未被主流程调用 | — |
| v2.6.0 | `gmp` | 从未被导入 | v2.6.0~v3.0.0 |
| v2.6.0 | `rag` | 未与 executor 连接 | v2.6.0~v3.0.0 |
| v2.6.0 | `unified-query` | 有 engine.rs 但未被 server 使用 | v2.6.0~v3.0.0 |
| v2.6.0 | `mysql-server` | 独立实现 | v2.6.0~v3.6.0 |
| v3.0.0 | `expr` | 无任何导入 | **v3.0.0~v3.6.0 新发现** |
| v3.0.0 | `code-graph` | 未集成 | — |
| v3.0.0 | `code-graph-cli` | 仅 CLI 工具 | — |
| v3.0.0 | `fuzz` | 仅测试目的 | — |

---

## 五、修复策略

### P0（v3.7.0 Alpha 前必须修复）

#### INT-1：WAL 集成

**目标**：DML 操作经过 TransactionManager/WAL

**路径**：
1. StorageEngine trait 增加事务方法：`begin_transaction/commit/rollback`
2. 实现 WAL-backed StorageEngine，包装现有 storage
3. DML 执行路径改为：LocalExecutor → TransactionManager → WAL-backed Storage
4. 保留 Direct Execution Path（bench-cli 用）通过 feature flag

**依赖**：
- `storage/wal.rs`（已有，2129行）
- `transaction/mvcc.rs`（已有）
- 需要：连接两者的桥接层

**验收标准**：
- INSERT/UPDATE/DELETE 经过 WAL
- COMMIT/ROLLBACK 正确持久化
- crash recovery 后数据一致

#### INT-2：ParallelVolcanoExecutor 集成

**目标**：主执行链路可选择并行执行

**路径**：
1. LocalExecutor 增加并行模式开关
2. QueryRouter 增加执行器选择逻辑
3. TaskScheduler 与 Rayon 集成
4. 统一 LocalExecutor 和 ParallelVolcanoExecutor 的调用接口

**验收标准**：
- `--parallel` 参数启用 ParallelVolcanoExecutor
- TPC-H Q1~Q22 在并行模式下正确执行
- 覆盖率提升 ≥ 10pp

---

### P1（v3.7.0 计划）

#### INT-3：expr crate 整合

**路径**：
1. executor 内部表达式求值逻辑改为调用 expr crate
2. 移除 executor 内联重复代码
3. expr crate 添加 `no_std` 支持（可选）

#### INT-4：mysql-server 协议统一

**路径**：
1. mysql-server 的 query 处理路径作为主协议栈
2. 移除 server/src/lib.rs 中的重复实现
3. 统一 COM_QUERY 入口

---

## 六、版本影响范围

| 版本 | 影响缺陷 | 跨度 |
|------|---------|------|
| v1.2.0 | INT-1 | 1 |
| v2.0.0 | INT-1 | 2 |
| v2.6.0 | INT-1, INT-2, INT-4 | 3 |
| v3.0.0 | INT-1, INT-2, INT-3, INT-4 | 4 |
| v3.2.0 | INT-1, INT-2, INT-3, INT-4 | 4 |
| v3.3.0 | INT-1, INT-2, INT-3, INT-4 | 4 |
| v3.4.0 | INT-1, INT-2, INT-3, INT-4 | 4 |
| v3.5.0 | INT-1, INT-2, INT-3, INT-4 | 4 |
| v3.6.0 | INT-1, INT-2, INT-3, INT-4 | 4 |

**跨越版本数**：10 个版本（v1.2.0 ~ v3.6.0）
**持续时间**：6 年+

---

## 七、相关文档

- Issue #2570：`ParallelVolcanoExecutor 未集成`
- Issue #2571：`WAL/MVCC/TransactionManager DML 集成缺失`
- Issue #2576：`DML 不经过 TransactionManager/WAL`
- Issue #2577：`ParallelVolcanoExecutor 孤岛`（44 tests isolated）
- Issue #2578：`execution_engine.rs 4658→6829 行膨胀`
- Issue #2579：`gate_spec 缺少 I-Gate 集成路径检查`
- Issue #2583：`DML 执行路径统一到 PhysicalPlan pipeline`
- Issue #2572：`PhysicalPlan→LocalExecutor 双执行路径`
- Issue #2573：`v3.6.0 Beta 需增加 I-Gate 集成路径检查`

---

*报告生成：2026-05-30*
*分析方法：git show 历史版本代码结构分析 + Issue 交叉验证*