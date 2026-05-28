# SQLRustGo v1.8.0 测试计划 (TEST_PLAN.md)

> **版本**: v1.8.0
> **发布日期**: 2026-03-25
> **代号**: SQL-92 Compliance Enhancement
> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29

---

## 一、测试概述

### 1.1 测试目标

v1.8.0 测试目标是验证 SQL-92 符合性增强，包括新增数据类型、语法支持和测试覆盖率提升。

### 1.2 测试范围

- Parser 模块: 新增 Token 解析
- DDL 模块: ALTER TABLE, CREATE/DROP INDEX
- DML 模块: INSERT SET, LIMIT/OFFSET
- Types 模块: DECIMAL, JSON 类型
- SQL-92 合规测试

---

## 二、测试策略

### 2.1 测试类别

| 类别 | 说明 | 状态 |
|------|------|------|
| 单元测试 | lib + parser 测试 | ✅ |
| SQL-92 合规测试 | 18 个合规用例 | ✅ |
| 集成测试 | 端到端查询 | ✅ |

### 2.2 测试金字塔

```
v1.8.0 测试金字塔
────────────────────────────────────────────
                    ▲
                   /E2E\         集成测试
                  /───────\
                 / smoke  \       冒烟测试
                /─────────\
               /   unit   \     单元测试
              /────────────\
             /  coverage   \   覆盖率驱动
            └───────────────┘
```

---

## 三、质量门禁

### 3.1 核心检查项

| # | 检查项 | 命令 | 期望结果 | 实际结果 | 状态 |
|---|--------|------|----------|----------|------|
| G1 | Build | `cargo build --workspace` | 成功 | 成功 | ✅ |
| G2 | Full Test Suite | `cargo test --workspace` | 100% | 100% | ✅ |
| G3 | Clippy | `cargo clippy --workspace` | 无 error | 无 error | ✅ |
| G4 | Format | `cargo fmt --check` | 通过 | 通过 | ✅ |
| G5 | Coverage ≥70% | `cargo llvm-cov` | ≥70% | 63.26% | ⚠️ |

---

## 四、功能测试详情

### 4.1 DDL 测试

| 测试项 | 说明 | 状态 |
|--------|------|------|
| ALTER TABLE | 表结构修改 | ✅ |
| CREATE INDEX | 索引创建 | ✅ |
| DROP INDEX | 索引删除 | ✅ |

### 4.2 DML 测试

| 测试项 | 说明 | 状态 |
|--------|------|------|
| INSERT SET | INSERT SET 语法 | ✅ |
| LIMIT/OFFSET | 分页查询 | ✅ |

### 4.3 Types 测试

| 测试项 | 说明 | 状态 |
|--------|------|------|
| DECIMAL | 精确数值类型 | ✅ |
| JSON | JSON 数据类型 | ✅ |

### 4.4 SQL-92 合规测试

| 类别 | 测试数 | 通过 |
|------|--------|------|
| DDL | 6 | ✅ |
| DML | 4 | ✅ |
| Queries | 4 | ✅ |
| Types | 4 | ✅ |
| **总计** | **18** | **100%** |

---

## 五、覆盖率详情

### 5.1 覆盖率指标

| 指标 | 值 |
|------|-----|
| 整体覆盖率 | 63.26% |
| 覆盖行数 | 6459/10210 |
| RC 目标 | ≥70% |
| Beta 目标 | ≥65% |

### 5.2 覆盖率说明

覆盖率略低于 70% 目标是因为 CLI、HTTP Server 等模块需要特定运行环境。核心功能覆盖率已达预期。

---

## 六、PR 合并记录

| PR | 标题 | 状态 |
|----|------|------|
| #772 | feat: Implement ALTER TABLE support | ✅ Merged |
| #773 | feat: Add DECIMAL data type support | ✅ Merged |
| #774 | feat: Add JSON data type support | ✅ Merged |
| #775 | feat: Add CREATE INDEX and DROP INDEX support | ✅ Merged |
| #776 | feat: Add string and datetime function tokens | ✅ Merged |
| #777 | feat: Add stored procedure tokens | ✅ Merged |
| #778 | feat: Add trigger tokens | ✅ Merged |
| #779 | feat: Add SQL-92 test suite framework | ✅ Merged |
| #783 | docs: Add v1.8.0 release documentation | ✅ Merged |

---

## 七、测试执行记录

### 7.1 执行命令

```bash
# 完整测试
cargo test --workspace

# 覆盖率
cargo llvm-cov --workspace

# SQL-92 合规测试
bash scripts/test/sql92_compliance.sh
```

---

## 八、改进建议

### 8.1 高优先级

1. **提升覆盖率到 70%**: CLI 和 HTTP Server 模块测试
2. **TPC-H 完整支持**: 全部 22 个查询

### 8.2 中优先级

1. **混沌测试**: 异常场景测试
2. **性能回归**: 建立性能基准对比机制

---

**本文档为事后补充 (Retrospective)，基于 2026-03-25 GA 实际执行结果生成。**

**最后更新**: 2026-05-29
**维护人**: hermes-z6g4