# SQLRustGo v3.7.0 版本计划

> **版本**: v3.7.0
> **分支**: develop/v3.7.0
> **HEAD**: af886c46d
> **日期**: 2026-05-30
> **状态**: Alpha 阶段
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md

---

## 版本概述

v3.7.0 是 **执行遥测 + 执行器架构重构** 版本，基于 v3.6.0 GA 基线。主要目标是引入 Execution Telemetry v2（分布式追踪、性能指标采集）、执行器模块重构、以及治理体系持续改进。

---

## 阶段目标

| 阶段 | 目标 | 关键里程碑 |
|------|------|-----------|
| Alpha | 基础架构完成 | Execution Telemetry API、executor 模块清理 |
| Beta | 功能集成 | Telemetry 与 query pipeline 集成、CI/CD 完善 |
| RC | 稳定化 | 回归测试、性能基线对齐 |
| GA | 发布 | 文档完善、发布准备 |

---

## 关键功能

### 1. Execution Telemetry v2

| 组件 | 说明 |
|------|------|
| trace_id | 全局链路追踪 ID，跨 executor/ storage 传播 |
| span 生命周期 | OpenTelemetry 兼容的 span 管理 |
| 指标采集 | query_latency_ms, rows_scanned, cache_hit_rate |
| 导出器 | OTLP/gRPC + 本地日志双输出 |

**分支**: `origin/feature/execution-telemetry-v2`

### 2. Executor 模块重构

- 执行器上下文传递链清理
- 无效代码和死代码移除
- Clippy 零警告基线

### 3. WAL DML 集成技术报告

已在 `docs/formal/WAL_DML_INTEGRATION.md` 归档，验证了 WAL 与 DML 操作的双写一致性。

---

## 从 v3.6.0 继承的稳定功能

| 功能 | 状态 |
|------|------|
| WALVerifier | ✅ 稳定，GA 基线 |
| SIMD 向量化 | ✅ 稳定 |
| Knowledge OS 集成 | ✅ 稳定 |
| Parser 窗口函数 | ✅ 稳定 |

---

## 开发计划

### Alpha (当前)

- [x] develop/v3.7.0 分支创建
- [x] v3.6.0 → v3.7.0 合并 (PR #2608)
- [ ] Alpha Gate 通过（文档 + 门禁检查）
- [ ] Execution Telemetry v2 核心 API

### Beta 入口条件

- Alpha Gate: 14/14 PASS
- Coverage >= 50%
- clippy --all-features -- -D warnings: 0 errors
- 文档: 8 个必需文档已创建

---

## 依赖关系

```
v3.6.0 GA (5720805b)
     └── develop/v3.7.0 (af886c46d)
              ├── feature/execution-telemetry-v2
              └── feature/merge-v360-into-v370
```

---

## 统计

| 指标 | 值 |
|------|-----|
| 从 v3.6.0 合并 | 1 merge commit |
| 新功能分支 | 1 (execution-telemetry-v2) |
| 文档缺口 | 8 个 (当前创建中) |