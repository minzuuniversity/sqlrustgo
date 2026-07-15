# v3.11.0 Feature Checklist

> **Version**: v3.11.0
> **Status**: DRAFT (2026-07-15, 起步于 v3.10.0 GA)
> **Owner**: @openclaw
> **Last update**: 2026-07-15 (DRAFT init)
> **Related**: Issue #3433 (V311-MASTER), STAGE.yaml, V311_DEVELOPMENT_PLAN.md

This document is the **SSOT** for v3.11.0's feature checklist, tracking 23 V311-XX tasks. The framework-level gates (G1-G16) are defined in `docs/governance/STAGE_CONFIG.yaml`; this document enumerates the *application integration* scenarios and the test coverage plan.

---

## 0. Strategic Positioning

**v3.11.0 = 债务清零 + 功能孤岛集成 + 性能突破**

继承 v3.10.0 23 项债务：
- INT/ARCH: 100% 闭环 (v3.10.0)
- SEM: 50% (SEM-1 closed, SEM-3/4 open)
- F-XX ISOLATED: 1/10 (F-16 closed, 9 deferred)
- F-XX NOT_IMPL: 2/5 (T-19/20 closed, F-03/30/36 open)
- Extension Crates: 11 项 (1 PARTIAL, 1 FROZEN, 1 INTERNAL, 8 SCOPE_DEFERRED)
- NEW: PERF-5 (高并发 INSERT lost connection) from v3.10.0 SOAK

---

## 1. F-XX 主路径集成 (9 项 / 480h)

| ID | 任务 | F-XX | 工作量 | 优先级 | 状态 | 测试方法 |
|----|------|------|--------|--------|------|----------|
| V311-01 | Clustered Index 主路径集成 | F-23 | 80h | P0 | ✅ DONE v1 2026-07-15 (PR #3461) | `tests/clustered_table_v1_test` |
| V311-02 | Adaptive Hash Index 主路径集成 | F-24 | 60h | P0 | ✅ DONE v3 2026-07-15 (PR #3465/#3476/#3478) | `tests/adaptive_hash_main_path_test` |
| V311-03 | Change Buffer 主路径集成 | F-25 | 40h | P0 | ⏳ TODO | `tests/change_buffer_main_path_test` |
| V311-04 | Double-Write Buffer 主路径集成 | F-26 | 50h | P0 | ⏳ TODO | `tests/double_write_main_path_test` |
| V311-05 | Row-Level Security 主路径集成 | F-29 | 40h | P1 | ⏳ TODO | `tests/row_level_security_test` |
| V311-06 | Performance Schema hooks | F-31 | 30h | P1 | ✅ DONE v1 2026-07-15 (trait + Noop + Counting) | `tests/instrumentation_hooks_test` |
| V311-07 | MySQL Admin 与 mysql-server 集成 | F-32 | 30h | P1 | ✅ DONE (fix/v311-07-f-32-admin-wire-integration) | `tests/admin_e2e_test` |
| V311-08 | Password Rotation 主路径集成 | F-35 | 20h | P1 | ⏳ TODO | `tests/password_rotation_test` |
| V311-12 | Table Compression (LZ4/zstd) | F-27 | 50h | P1 | ⏳ TODO | `tests/compression_lz4_test` |

---

## 2. F-XX NOT_IMPLEMENTED (3 项 / 140h)

| ID | 任务 | F-XX | 工作量 | 优先级 | 状态 | 测试方法 |
|----|------|------|--------|--------|------|----------|
| V311-09 | 列级权限实现 | F-36 | 40h | P0 | ✅ DONE 2026-07-15 (PR #3457) | `tests/column_privilege_test` (12/12) |
| V311-10 | CREATE SEQUENCE 实现 | F-30 | 20h | P1 | ⏳ TODO | `tests/sequence_test` |
| V311-11 | GIS 空间数据类型 (POINT + WITHIN) | F-03 | 80h | P1 | ⏳ TODO | `tests/gis_basic_test` |

---

## 3. SEM 债务 (2 项 / 80h)

| ID | 任务 | SEM | 工作量 | 优先级 | 状态 | 测试方法 |
|----|------|-----|--------|--------|------|----------|
| V311-13 | ALTER TABLE RENAME/MODIFY 完整 | SEM-3 | 20h | P0 | ✅ DONE 2026-07-15 (PR #3444/#3449) | `tests/alter_table_test` |
| V311-14 | 覆盖率 ≥85% | SEM-4 | 60h | P0 | ⏳ TODO | `cargo llvm-cov --workspace` |

---

## 4. 性能优化 (4 项 / 210h) - 核心

| ID | 任务 | PERF | 工作量 | 优先级 | 状态 | 测试方法 |
|----|------|------|--------|--------|------|----------|
| V311-15 | Q4 相关子查询 Hash Semi Join 算子 | PERF-1 | 80h | P0 | ✅ DONE 2026-07-15 (PR #3455) | `tests/q4_hash_semi_join_test` |
| V311-16 | Decorrelation optimizer pass | PERF-4 | 60h | P1 | ✅ DONE v2 2026-07-15 (decorrelate() rewrite) | `tests/decorrelation_v2_test` |
| V311-17 | Hash Anti Join 算子 | PERF-2 | 40h | P1 | ✅ DONE 2026-07-15 | `tests/anti_join_main_path_test` |
| V311-18 | CTE 物化 | PERF-3 | 30h | P1 | ⏳ TODO | `tests/cte_materialize_test` |

---

## 5. 性能债务 (1 项 / 60h) - NEW from v3.10.0 SOAK

| ID | 任务 | PERF | 工作量 | 优先级 | 状态 | 测试方法 |
|----|------|------|--------|--------|------|----------|
| V311-23 | High-concurrency INSERT lost connection 修复 | PERF-5 | 60h | P1 | ✅ DONE (2026-07-15) | `tests/stress/concurrent_insert_test.rs` |

---

## 6. Extension Crate 决策 (1 项 / 84h)

| ID | 任务 | 工作量 | 优先级 | 状态 | 详情 |
|----|------|--------|--------|------|------|
| V311-19 | Extension Crate 决策实施 | 84h | P1 | ✅ DONE 2026-07-15 | 5 删 + 3 归档 + 1 集成 + 1 保留 |

### 6.1 删除 (5)

| Crate | 决策 | 原因 |
|-------|------|------|
| agentsql | DELETE | NL2SQL gateway, no production callers |
| rag | DELETE | Depends on gmp |
| qmd-bridge | DELETE | 0 调用方 |
| evidence-graph | DELETE | Gate tool independent use |
| unified-query | DELETE | Depends on graph/vector |
| unified-storage | DELETE | Depends on graph |

### 6.2 归档 (3)

| Crate | 决策 | 归档路径 |
|-------|------|---------|
| gmp | ARCHIVE | `archive/v3.11/gmp/` |
| graph | ARCHIVE | `archive/v3.11/graph/` (Cypher 推到 v3.12+) |
| distributed | FROZEN (原本) | 保留在 workspace, 不参与 build |

### 6.3 集成 (1)

| Crate | 决策 | 详情 |
|-------|------|------|
| admin | INTEGRATE | `sqlrustgo-admin` ↔ `mysql-server` 通过 wire protocol (V311-07) |

### 6.4 保留 (1)

| Crate | 决策 | 详情 |
|-------|------|------|
| vector | RETAIN | HNSW/IVF/PQ 是 storage 内部能力 |

---

## 7. 基础设施 (3 项 / 172h)

| ID | 任务 | 工作量 | 优先级 | 状态 | 测试方法 |
|----|------|--------|--------|------|----------|
| V311-20 | TPC-H SF=1.0 baseline (#3423) | 80h | P0 | ⏳ TODO | TPC-H 22/22 @ SF=1 |
| V311-21 | 168h SOAK v3.11.0 (#3648) | (Hermes 协作) | P1 | ⏳ TODO | 168h 持续监控 |
| V311-22 | 文档架构整理 (5 plans → 3 plans) | 12h | P2 | ✅ DONE | `plans/INDEX.md` |

---

## 8. SQL 92 子集覆盖 (v3.11.0 增加)

| 类别 | v3.10.0 | v3.11.0 增加 | 总计 |
|------|---------|-------------|------|
| DML | INSERT/UPDATE/DELETE/MERGE | ALTER RENAME/MODIFY (V311-13) | 全 |
| DDL | CREATE/ALTER/DROP/TRUNCATE | + SEQUENCE (V311-10), GIS (V311-11) | 全 |
| 权限 | GRANT/REVOKE | + 列级权限 (V311-09) | 全 |
| 索引 | B+ Tree | + Clustered (V311-01), Adaptive Hash (V311-02) | 强 |
| 优化 | CBO | + Decorrelation (V311-16) | 强 |
| 执行 | Volcano | + Hash Semi Join (V311-15), Hash Anti Join (V311-17) | 强 |
| CTE | WITH (non-materialized) | + 物化 (V311-18) | 全 |
| 函数 | 标准 | + ST_WITHIN (V311-11) | 扩展 |

---

## 9. 测试矩阵 (5 维度)

| 维度 | v3.10.0 状态 | v3.11.0 目标 | 测试方法 |
|------|------------|-------------|----------|
| **功能完整性** | DML/ACID 完整 | + ALTER/SEQUENCE/GIS/RBAC/索引 | `cargo test --all-features` |
| **MySQL 5.7 兼容性** | 6/10 @ SF=1 | 22/22 @ SF=1 | `tests/tpch_sf1_test.rs` |
| **性能** | Q1 1.27x @ SF=1 | Q1 1.5x+ @ SF=1 | `perf/PERFORMANCE_BASELINE.md` |
| **稳定性** | 168h SOAK (Issue #3434) | 168h SOAK v3.11.0 | `scripts/stability/run_168h_soak.sh` |
| **覆盖率** | 14.71% lib | ≥85% per crate | `cargo llvm-cov --workspace` |

---

## 10. 质量门禁

| 门禁 | 阈值 | v3.10.0 状态 | v3.11.0 目标 |
|------|------|-------------|-------------|
| Cargo build | 0 errors | ✅ PASS | ✅ 保持 |
| Cargo clippy | 0 errors | ✅ PASS | ✅ 保持 |
| Cargo fmt | 0 diffs | ✅ PASS | ✅ 保持 |
| `cargo test --lib` | 0 failures | 28/28 | ≥50/50 |
| `#[ignore]` 数量 | ≤10 | 10 | ≤5 |
| 覆盖率 (lib) | ≥80% per crate | 14.71% | ≥85% per crate |
| TPC-H SF=0.1 | 22/22 | ✅ PASS | ✅ 保持 |
| TPC-H SF=1 | 6/10 | ⚠️ 部分 | **22/22** (V311-20) |
| 168h SOAK | 0 crashes | ✅ PASS | ✅ 保持 (V311-21) |
| E2E 8/8 | PASS | ✅ PASS | ✅ 保持 |
| 高并发 INSERT | 0 errors | ❌ PERF-5 | ✅ PASS (V311-23) |

---

## 11. 任务完成度跟踪

| 阶段 | 任务数 | 完成 | 进行中 | TODO |
|------|--------|------|--------|------|
| ALPHA (P0) | 9 (V311-01/02/03/04/09/13/15/19/20) | **6** (01/02/09/13/15/19) | 0 | 3 (03/04/20) |
| BETA (P1) | 10 (V311-05/06/07/08/10/11/12/16/17/18) | **4** (06/07/16/17) | 0 | 6 (05/08/10/11/12/18) |
| RC/Others | 5 (V311-14/20/21/22/23) | **2** (22/23) | 0 | 3 (14/20/21) |
| **总计** | **24** | **12** | **0** | **12** |

```
完成度: 50.0% (12/24)
总工作量: ~1156h
ALPHA 起点: 2026-07-15 (DRAFT init)
GA 目标: 2026-10-01
```

### 已完成 (12)

| # | 任务 | PR/证明 |
|---|------|---------|
| V311-01 | F-23 Clustered Index 主路径集成 | PR #3461 |
| V311-02 | F-24 Adaptive Hash Index 主路径集成 | PR #3465/#3476/#3478 |
| V311-06 | F-31 Performance Schema hooks | trait + Noop + Counting |
| V311-07 | F-32 MySQL Admin 与 mysql-server 集成 | fix/v311-07-f-32-admin-wire-integration |
| V311-09 | F-36 列级权限实现 | PR #3457 |
| V311-13 | SEM-3 ALTER TABLE RENAME/MODIFY | PR #3444/#3449 |
| V311-15 | PERF-1 Hash Semi Join 算子 | PR #3455 |
| V311-16 | PERF-4 Decorrelation optimizer pass | rewrite v2 |
| V311-17 | PERF-2 Hash Anti Join 算子 | PR |
| V311-19 | Extension Crate 决策 | 5 删 + 3 归档 + 1 集成 + 1 保留 |
| V311-22 | 文档架构整理 | plans/INDEX.md |
| V311-23 | PERF-5 高并发 INSERT 修复 | 从 v3.10.0 SOAK 修复 |

### 剩余 (12)

| # | 任务 | 工作量 | 优先 |
|---|------|--------|------|
| V311-03 | F-25 Change Buffer | 40h | P0 |
| V311-04 | F-26 Double-Write Buffer | 50h | P0 |
| V311-05 | F-29 Row-Level Security | 40h | P1 |
| V311-08 | F-35 Password Rotation | 20h | P1 |
| V311-10 | F-30 CREATE SEQUENCE | 20h | P1 |
| V311-11 | F-03 GIS (POINT + WITHIN) | 80h | P1 |
| V311-12 | F-27 Table Compression | 50h | P1 |
| V311-14 | SEM-4 覆盖率 ≥85% | 60h | P0 |
| V311-18 | CTE 物化 | 30h | P1 |
| V311-20 | TPC-H SF=1.0 baseline | 80h | P0 |
| V311-21 | 168h SOAK v3.11.0 | — | P1 |

---

## 12. 跨引用

| 文档 | 路径 |
|------|------|
| 战略计划 | `docs/releases/v3.11.0/plans/V311_VERSION_PLAN.md` |
| 开发计划 | `docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md` |
| 债务清零 | `docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md` |
| 文档架构 | `docs/releases/v3.11.0/plans/V311_DOCS_RESTRUCTURE_PLAN.md` |
| 阶段状态 | `docs/releases/v3.11.0/STAGE.yaml` |
| 战略入口 | `docs/releases/v3.11.0/VERSION_PLAN.md` |
| 债务基线 | `docs/governance/debt/debt-registry.yaml` |
| Issue #3433 (V311-MASTER) | http://192.168.0.250:3000/openclaw/sqlrustgo/issues/3433 |
| Issue #3434 (PERF-5) | http://192.168.0.250:3000/openclaw/sqlrustgo/issues/3434 |

---

## 13. 状态更新日志

| 日期 | 更新 | 操作人 |
|------|------|-------|
| 2026-07-15 | 初始 DRAFT 创建 | openclaw |

---

*Created: 2026-07-15 (DRAFT init) — v3.11.0 ALPHA 起点*
*Last update: 2026-07-15*
*Author: openclaw (基于 v3.10.0 LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md + V311_DEVELOPMENT_PLAN.md)*
