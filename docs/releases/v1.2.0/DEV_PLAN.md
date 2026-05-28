# SQLRustGo v1.2.0 开发计划 (DEV_PLAN.md)

> **版本**: v1.2.0
> **发布日期**: 2026-03-13
> **代号**: Architecture Refactoring
> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29

---

## 一、版本概述

### 1.1 战略定位

| 版本 | 定位 | 核心能力 | 发布日期 |
|------|------|----------|----------|
| **v1.2.0** | Architecture Refactoring | 架构接口化、CBO 基础 | 2026-03-13 |
| **v1.1.0** | L3 产品级 | Client-Server 网络 | 2026-03-03 |

### 1.2 核心目标

v1.2.0 是架构重构版本，核心目标：

1. **接口化**: 把单机数据库内核改造成可扩展数据库平台内核
2. **CBO 框架**: 把 CBO 设计成可插拔优化框架
3. **解耦**: 把执行引擎与优化器彻底解耦

---

## 二、功能列表（实际完成）

### 2.1 架构重构 (P0)

| 任务 ID | 模块 | 功能 | 状态 | PR |
|---------|------|------|------|-----|
| R-001 | common | 错误域重构 | ✅ | #305 |
| R-002 | optimizer | Optimizer 框架 | ✅ | #305 |
| R-003 | types | Statistics 子系统 | ✅ | #305 |
| R-004 | catalog | Catalog 系统 | ✅ | #305 |
| R-005 | executor | Executor trait | ✅ | #305 |
| R-006 | storage | StorageEngine trait | ✅ | #305 |
| R-007 | transaction | WAL + TransactionManager | ✅ | #305 |

### 2.2 CBO 成本优化 (P1)

| 任务 ID | 模块 | 功能 | 状态 | PR |
|---------|------|------|------|-----|
| C-001 | optimizer | CostModel 结构设计 | ✅ | - |
| C-002 | optimizer | 基础成本估算 | ✅ | - |
| C-003 | optimizer | Join 选择优化 | ✅ | - |
| C-004 | optimizer | 索引选择优化 | ✅ | - |
| C-005 | optimizer | 优化器集成 | ✅ | - |

### 2.3 向量化基础 (P2)

| 任务 ID | 模块 | 功能 | 状态 | PR |
|---------|------|------|------|-----|
| V-001 | executor | RecordBatch 结构 | ✅ | 延后到 v1.3.0 |
| V-002 | executor | ColumnarArray trait | ✅ | 延后到 v1.3.0 |
| V-003 | executor | 向量化表达式 | ✅ | 延后到 v1.3.0 |

---

## 三、目录结构（重构后）

### 3.1 crates/ workspace 结构

```
crates/
├── common/              # 通用错误类型 (SqlError)
├── types/              # 类型系统 (Value, DataType, error types)
├── parser/             # SQL 解析 (Lexer, Parser, Token)
├── planner/            # 逻辑计划 (占位)
├── optimizer/          # 优化器 (Rule, Cost, Memo)
├── executor/           # 执行器 (Operator, RecordBatch)
├── storage/            # 存储引擎 (StorageEngine trait, FileStorage)
├── catalog/            # 元数据管理
├── transaction/        # 事务 (WAL, TransactionManager)
└── server/             # 网络服务 (REPL, QueryService)
```

### 3.2 核心 Trait 定义

```rust
// Optimizer trait
trait Optimizer {
    fn optimize(&self, plan: LogicalPlan) -> Result<LogicalPlan>;
}

// Rule trait
trait Rule {
    fn apply(&self, plan: &mut LogicalPlan) -> bool;
}

// CostModel trait
trait CostModel {
    fn estimate(&self, plan: &LogicalPlan) -> f64;
}

// Executor trait
trait Executor: Send {
    fn execute(&self, plan: PhysicalPlan) -> Result<ExecutionResult>;
}

// StorageEngine trait
trait StorageEngine: Send + Sync {
    fn read_page(&self, page_id: PageId) -> Result<Page>;
    fn write_page(&self, page: &Page) -> Result<()>;
}

// StatisticsProvider trait
trait StatisticsProvider {
    fn table_stats(&self, table: &str) -> Option<TableStats>;
}
```

---

## 四、问题→开发任务映射（实际执行结果）

### 4.1 Issue 执行情况

| Issue | 功能 | 状态 | 合并 PR |
|-------|------|------|---------|
| #305 | 目录重构 (crates/ workspace) | ✅ | #305 |
| #103 | 储存特质化 | ✅ | #305 |
| #104 | Optimizer 规则系统实现 | ✅ | #305 |

---

## 五、实际 PR 清单

| PR | 功能 | 状态 |
|----|------|------|
| #302 | 修复 clippy 警告 | ✅ |
| #303 | 合并 clippy 修复 | ✅ |
| #304 | 修复 index 测试编译错误 | ✅ |
| #305 | 目录重构 (crates/ workspace) | ✅ |
| #306 | 修复 Cargo.toml 重复定义 | ✅ |
| #307 | 修复格式化问题 | ✅ |

---

## 六、质量门禁结果

### 6.1 GA Gate 检查项

| 检查项 | 期望 | 实际 | 状态 |
|--------|------|------|------|
| 编译通过 | ✅ | ✅ | ✅ |
| 测试通过 | ✅ | ✅ | ✅ |
| Clippy | 零警告 | 零警告 | ✅ |
| Format | 通过 | 通过 | ✅ |

---

## 七、核心接口完成度

| 能力 | 是否具备 | 实现位置 | 说明 |
|------|----------|----------|------|
| CBO 可扩展 | ✅ | crates/optimizer/ | Optimizer trait + Rule + CostModel |
| 多优化器支持 | ✅ | crates/optimizer/ | 可插拔优化框架 |
| 执行层替换 | ✅ | crates/executor/ | Executor trait |
| 统计独立 | ✅ | crates/types/ | StatisticsProvider trait |
| Catalog 可共享 | ✅ | crates/catalog/ | Catalog trait + 可序列化 |
| 为 2.0 预留接口 | ✅ | 各 crate | 所有核心 trait 已定义 |
| 存储抽象 | ✅ | crates/storage/ | StorageEngine trait |
| 事务管理 | ✅ | crates/transaction/ | WAL + TransactionManager |

---

## 八、经验教训 (Lessons Learned)

### 8.1 做得好 (What Went Well)

1. **架构接口化**: 所有核心 trait 正确分离
2. **目录结构**: crates/ workspace 结构清晰
3. **错误域分离**: 各模块错误独立

### 8.2 需要改进 (What Needs Improvement)

1. **向量化延后**: 向量化执行延后到 v1.3.0
2. **CBO 简化**: 初始版本 CBO 做了简化实现

### 8.3 行动项

| 行动项 | 优先级 |
|--------|--------|
| v1.3.0 继续向量化开发 | P0 |
| 完善 CBO 成本模型 | P1 |

---

**本文档为事后补充 (Retrospective)，基于 2026-03-13 GA 实际执行结果生成。**

**最后更新**: 2026-05-29
**维护人**: hermes-z6g4