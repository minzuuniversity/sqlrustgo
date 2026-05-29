# SQLRustGo v3.6.0 变更日志

> **版本**: v3.6.0
> **分支**: develop/v3.6.0
> **日期**: 2026-05-30
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md

---

## 变更概述

v3.6.0 是**知识增强 + 存储可靠性**版本，从 v3.5.0 GA (5720805b) 分支而来。主要变更包括 WALVerifier 框架、SIMD 向量化集成、qmd-bridge Knowledge OS 桥接、以及 parser 修复。

---

## 关键提交日志

按时间顺序列出 (从 v3.5.0 分支到现在)：

### 基础设施与修复

| 提交 | 描述 |
|------|------|
| `86f7cb13` | **fix(v3.6.0-fix2)**: fix sql-corpus compilation — 添加 DropColumn/ModifyColumn 分支，cargo check PASS |
| `c070fd4e` | **feat(v3.6.0-fix3)**: WAL verification integrated — WALVerifier 框架集成 |
| `11ffbb98` | **feat(v3.6.0-fix4)**: SIMD integration + WAL verification + compile fixes — SIMD 向量化集成和 WAL 验证编译修复 |
| `1ff26b7c` | **feat**: Alpha Gate fixes + SSI downgrade + VectorBatch OOM protection — Beta 预备修复 |
| `86ff2a3d` | **fix**: qmd-bridge hybrid test QmdDataType import — Knowledge OS 桥接测试修复 |

### 形式化验证

| 提交 | 描述 |
|------|------|
| `eee696d1` | **fix(formal)**: fix WAL_Recovery.tla infinite set issue for TLC model checking — TLA+ 模型修复 |

### 文档与门禁

| 提交 | 描述 |
|------|------|
| `5bd5a741` | **docs**: v3.6.0 gate contract — Gate 合约文档 |
| `a8b15c42` | **docs**: v3.6.0 Alpha Gate report - CONDITIONAL PASS (81.97%) |
| `6f4ea786` | **docs**: v3.6.0-final: Alpha Gate CONDITIONAL PASS (81.97%) |
| `585e3670` | **docs**: v3.6.0 DEVELOPMENT_PLAN.md — Alpha→Beta 延续任务映射 |
| `04abe688` | **governance**: Beta 入口文档 — TEST_PLAN + QUICK_START + COVERAGE_ANALYSIS + BETA_CHECKLIST |

### 窗口函数修复

| 提交 | 描述 |
|------|------|
| `4a5a7f19` | **fix(window)**: add missing PercentRank/CumeDist and fix test assertion |
| `045d4f3c` | **test**: parser coverage integration tests (32 tests, 0 failed) |
| `1b2a3c71` | **Merge**: fix/window-executor-architecture into develop/v3.6.0 |

---

## 新增组件

| 组件 | 路径 | 说明 |
|------|------|------|
| WALVerifier crate | `crates/wal-verification/` | WAL 验证专用 crate |
| WAL 验证逻辑 | `crates/wal-verification/src/verification.rs` | LSN 连续性、checksum、恢复验证 |
| TLA+ 模型修复 | `specs/WAL_Recovery.tla` | 形式化验证模型 (无限集合修复) |
| SIMD 加速 | `crates/vector/src/simd_explicit.rs` | 显式 SIMD 向量操作 |

---

## 依赖变更

| 依赖 | 版本 | 说明 |
|------|------|------|
| tokio | 1.x (不变) | 异步运行时 |
| simd-json | (新增) | JSON SIMD 解析 (可选 feature) |

---

## 统计

| 指标 | 值 |
|------|-----|
| 提交数 (v3.5.0 → v3.6.0) | 12 |
| 新增 WAL 测试 | 63 |
| 新增 Parser 测试 | 32 |
| 文件变更 | crates/wal-verification/, crates/vector/, crates/parser/ |
| Clippy 警告 | 0 |

---

*SSOT 参考: docs/governance/SSOT_CROSS_CHECK.md*
*更新日期: 2026-05-30*
