# SQLRustGo v3.11.0 版本计划 — 债务清零 + 功能孤岛集成

> **本文件** 是 v3.11.0 的版本入口. 详细内容见 `plans/V311_VERSION_PLAN.md`.

## 快速链接

- 战略定位: `plans/V311_VERSION_PLAN.md` § 1
- 任务清单: `plans/V311_DEVELOPMENT_PLAN.md` (~22 任务 / ~720h / 4 阶段)
- 债务追踪: `plans/V311_DEBT_CLOSURE_PLAN.md` (基于 `docs/governance/debt/debt-registry.yaml`)
- 文档架构: `plans/V311_DOCS_RESTRUCTURE_PLAN.md`
- 当前阶段: `STAGE.yaml` (DRAFT)
- 变更历史: `CHANGELOG.md`

## 1. 战略定位 (简版)

**v3.11.0 = 债务清零 + 功能孤岛集成**

- 彻底解决 v3.6.0→v3.10.0 所有遗留债务 (23 项)
- 完成 9 项 F-XX ISOLATED 主路径集成
- 完成 3 项 F-XX NOT IMPLEMENTED (GIS / SEQUENCE / 列权限)
- 解决 11 个 extension crate 的产品决策
- 实现 v3.10.0 实测发现的 Q4 相关子查询性能瓶颈
- 文档架构全面整理 (合并 VERSION_PLAN.md 到 VERSION_PLAN.md)

**不做**: 新架构 (分布式/Cypher/SIMD 重写), 重大协议变更

详细见 `plans/V311_VERSION_PLAN.md`.

## 2. 版本演进关系

```
v3.9.0:  Production Readiness (Single-Node Production Candidate)
v3.10.0: MySQL 5.7 替代 — 功能稳定 + 基本性能 (INT/ARCH/SEM 100% 闭环)
    ↓
v3.11.0: 债务清零 + 功能孤岛集成 ← 当前 DRAFT
    ↓ (计划)
v3.12+:  分布式 + 新语法扩展
```

## 3. 与 v3.10.0 关系

| 维度 | v3.10.0 | v3.11.0 |
| --- | --- | --- |
| 主题 | MySQL 5.7 替代 | **债务清零 + 功能集成** |
| 核心任务 | 26 项 (~500h) | **~22 项 (~720h)** |
| 债务闭环率 | ~70% | **目标 100%** |
| F-XX ISOLATED | 1/10 集成 (F-16) | **目标 10/10 集成** |
| F-XX NOT IMPL | 2/5 闭环 (T-19/20) | **目标 5/5 闭环** |
| Extension Crates | 11 项未决 | **目标产品决策完成** |
| Q4 相关子查询 | 占 96% 时间 | **目标 Hash Semi Join (1.5x+ 加速)** |

---

*Created: 2026-07-13 (DRAFT stage init)*
*Author: openclaw (基于 v3.10.0 LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md 撰写)*
