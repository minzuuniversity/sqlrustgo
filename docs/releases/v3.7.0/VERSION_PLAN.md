# SQLRustGo v3.7.0 版本计划 — GA Final

> **版本**: v3.7.0 GA
> **分支**: `origin/develop/v3.7.0` (commit `83d70e7c`)
> **日期**: 2026-05-30
> **状态**: GA ✅ — 开发冻结

---

## 1. 版本概述

v3.7.0 GA 是 **Stable SQL Execution Engine Release**，基于 v3.6.0 GA 构建。

### 核心交付

| 交付项 | 状态 |
|--------|------|
| P0-1: Session-level engine cache | ✅ GA |
| P0-2: SKIP_AUTH=false | ✅ GA |
| GA Score: 65/100 | ✅ GA |
| TPC-H SF=1: 22/22 | ✅ GA |

---

## 2. 版本阶段（已完成）

| 阶段 | 状态 | 里程碑 |
|------|------|--------|
| Alpha | ✅ PASS | v3.7.0-RC1 tag (`3e647254`) |
| Beta | ✅ PASS | P0-1, P0-2 修复完成 |
| RC | ✅ PASS | GA Score 65/100 |
| GA | ✅ **2026-05-30** | 正式 GA |

---

## 3. 版本生命周期（GA 后）

| 阶段 | 目标版本 | 说明 |
|------|----------|------|
| v3.7.0 GA | **当前** | Stable Execution Engine GA |
| v3.7.x | 后续 | Stabilization patch (SHOW TABLES, auth edge case) |
| v3.8.0 Alpha | 规划中 | INT-1~INT-4 架构重构 |
| v3.8.0 GA | 规划中 | Architecture Unification GA |

---

## 4. 版本定性

> **v3.7.0 GA = MySQL-compatible SQL Execution Engine with Session-level Transaction Semantics**

### GA 包含范围

- MySQL wire protocol
- SQL DDL/DML
- Session-level transaction (BEGIN/COMMIT)
- Authentication
- Parser (93 tests)
- TPC-H SF=1 (22/22)

### GA 排除范围（→ v3.8.0）

- WAL / crash recovery
- MVCC / full ROLLBACK
- VTU integration
- Single execution path
- expr crate 收敛
- execution_engine.rs 拆分

---

## 5. SSOT

- SSOT: `docs/governance/SSOT_CROSS_CHECK.md`
- Issue Tracking: Gitea Issues (`#2583`, `#2584`, `INT-1~INT-4`)

---

## 6. 参考文档

- [RELEASE_SUMMARY.md](RELEASE_SUMMARY.md) — GA 发布总结
- [RELEASE_NOTES.md](RELEASE_NOTES.md) — 发布说明
- [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) — 开发完成报告
- [../v3.8.0/VERSION_PLAN.md](../v3.8.0/VERSION_PLAN.md) — v3.8.0 计划