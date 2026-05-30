# SQLRustGo v3.7.0 发布说明

> **版本**: v3.7.0
> **分支**: develop/v3.7.0
> **HEAD**: af886c46d
> **日期**: 2026-05-30
> **状态**: Alpha

---

## 版本概述

v3.7.0 是 **执行遥测 + 执行器架构重构** 版本，基于 v3.6.0 GA。

### 主要变更

1. **Execution Telemetry v2**: 分布式追踪 + 性能指标采集
2. **Executor 模块清理**: 死代码移除、clippy 零警告
3. **WAL DML 集成**: 技术报告归档，验证双写一致性

### 继承自 v3.6.0

- WALVerifier 框架
- SIMD 向量化加速
- Knowledge OS 集成 (qmd-bridge)
- Parser 窗口函数完整支持

---

## 关键提交

| 提交 | 描述 |
|------|------|
| `af886c46d` | Merge PR #2608: v3.6.0 → v3.7.0 合并 |
| `88c500525` | docs: WAL DML integration technical report |
| `f048ef029` | Merge PR #2594: v3.6.0 治理体系改进 + Build Fixes |

---

## 统计

| 指标 | 值 |
|------|-----|
| 新功能 | 1 (Execution Telemetry v2) |
| Bug 修复 | 继承 v3.6.0 所有修复 |
| CLI 变更 | 无 |
| breaking | 无 |

---

## 升级路径

v3.7.0 → v3.8.0（Beta → RC → GA）