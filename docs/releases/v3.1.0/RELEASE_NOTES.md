# v3.1.0 Release Notes（Retrospective）

> **版本**: v3.1.0 GA
> **发布日期**: 2026-05-11
> **分支**: `develop/v3.1.0`
> **状态**: ✅ GA Released（Retrospective）
> **Truthfulness Score**: 70%

---

## 版本概述

v3.1.0 是 SQLRustGo 的**Beta Gate 系统增强**版本，在 v3.0.0 基础上完善压力测试体系，为 v3.2.0 GMP Framework 奠定基础。

**核心目标**：
- Spill Framework 集成，支持 TPC-H SF=10
- Beta Gate 系统增强（B1-B9 完整门禁）
- 聚合表达式改进 SUM(col1*col2)
- MySQL 协议优化（连接池 + Flush）

---

## 核心变更

### Spill Framework 集成

- **SpillHashJoin**: Hash join with disk spill support
- **SpillAggregate**: Aggregate with disk spill support
- **SpillOperator**: Base spill operator for large result sets
- **TPC-H SF=10**: 磁盘级排序支持多 GB 查询
- PR: #1064, #1049, #1069

### 聚合表达式增强

- `SUM(col1 * col2)` — 聚合支持表达式参数
- `AVG(column / divisor)` — 除法支持
- 复杂聚合参数支持
- PR: #1048

### MySQL 协议优化

| 优化项 | PR | 说明 |
|--------|-----|------|
| 连接池 | #1013 | MAX_CONNECTIONS + 线程池 |
| Flush 优化 | #1059, #1060 | 降低客户端响应延迟 |
| 协议解析 | — | SQL 解析函数优化 |

---

## 门禁通过记录

| 阶段 | 日期 | 状态 |
|------|------|------|
| Alpha | 2026-05-01 | ✅ PASS |
| Beta | 2026-05-05 | ✅ PASS (B1-B9) |
| RC | 2026-05-09 | ✅ PASS (R1-R8 + R-S1~S4) |
| GA | 2026-05-11 | ✅ PASS (G1-G8) |

**Beta Gate 增强记录**：
- B1-B9 完整门禁实现
- 并行覆盖率执行（避免超时）
- 稳定性检查 300s timeout (R-S1~S4)

---

## 升级说明

从 v3.0.0 升级无需数据库迁移：

```bash
cargo update
cargo build --all
```

---

## 相关文档

| 文档 | 说明 |
|------|------|
| [DEV_PLAN.md](./DEV_PLAN.md) | v3.1.0 开发计划（Retrospective） |
| [TEST_PLAN.md](./TEST_PLAN.md) | v3.1.0 测试计划（Retrospective） |
| [README.md](./README.md) | v3.1.0 综合说明（Retrospective） |
| [CHANGELOG.md](./CHANGELOG.md) | v3.1.0 变更日志（Retrospective） |

---

*发布于 2026-05-11（Retrospective 补充于 2026-05-29）*
*文档版本: v1.0*
*创建: hermes-agent（Retrospective）*
*Truthfulness Score: 70%*