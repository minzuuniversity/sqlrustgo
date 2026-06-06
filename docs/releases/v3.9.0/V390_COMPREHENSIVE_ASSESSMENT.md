# SQLRustGo v3.9.0 综合评估报告 (Comprehensive Assessment v1.0 — RC2 Post-Audit)

> **Date**: 2026-06-07 (RC2 Post-Audit 视角)
> **Version**: v3.9.0 (develop/v3.9.0, RC2 后期, 准备 RC3)
> **Author**: Hermes Agent
> **Baseline HEAD**: `7f8ea7c1` (TPC-H Failure Matrix v1, #3257, RC2 末) / `0852e42e` (clippy G2 fixes, #3254)
> **Status**: **RC2 ✅ (form-only) / RC3 ⏳ (P0 cut 待启动)**
> **Type**: **Production Readiness Release** (工程化版本, 非功能版本)
> **Theme**: Single-Node Production Candidate
> **GA Target**: 2026-09-23 (per V390 plan, at risk due to Z6G4 dependency)
> **Baseline 前版本**: v3.8.0 GA (`40f62ab5` v3.8.0 GA Final merge; V380 v3.2 baseline `9c6e90545`, 实用型数据库引擎 8.4~8.7/10)
> **Reference 文档**: `docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md` (v3.2)
> **互补文档**: `docs/audit/status/2026-06-07-v390-comprehensive-assessment.md` (claude-macmini, PR #3255, gate-by-gate 评估, 35%→60% test authenticity)
> **本 v1.0 评估原则**: 仿照 V380 §0-§20 结构, 但 v3.9.0 视角重点呈现
> **(a) v3.9.0 战略反转 (Production Readiness vs Feature Release)**
> **(b) Alpha1 → Beta → RC1 → RC2 完整阶段历程**
> **(c) RC2 form-only 真相 (RC3_PLAN 揭示的 35% 真实生产覆盖率)**
> **(d) 13 critical-path items + 调整后 RC3/RC4/GA 计划**
> **(e) v3.8.0 → v3.9.0 baseline 继承 + 仍需关闭的 4 跨版本债**
> **取最大集方式**: 保留 V380 §0/§1/§6/§11/§17/§18/§19/§20 8 个核心维度, 重写其他章节为 v3.9.0 视角, 整合 RC1/RC2_GATE_REPORT + RC3_PLAN + BETA_RELEASE_NOTES + claude-macmini audit 关键数据
> **双视角说明**:
> - **本文件** (`docs/releases/v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md`): V380 仿照形式, 阶段历程 + 资源分配 + 综合评分 (12 维度), 偏 release-management 视角
> - **互补文件** (`docs/audit/status/2026-06-07-v390-comprehensive-assessment.md`): gate-by-gate 评估 + 5 corrective paths + Z440-actionable items, 偏 audit 视角
> - 两份评估**不冲突**, 互补: 本文件给战略反转 + 资源分配, audit 文件给 gate form vs substance 细节

---

## 0. 总体结论 (TL;DR)

**v3.9.0 = Production Readiness Release (工程化版本) — RC2 ✅ form-only → RC3 ⏳ P0 cut 待启动**:

- **核心反转**: 从"还能加什么 SQL" → "数据库死了以后还能不能回来"
- **资源分配**: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / **新 SQL 0%**
- **阶段完成度**: 16/16 子任务 + Alpha1 ✅ + Beta ✅ + RC1 ✅ + RC2 ✅ (form-only)
- **门禁状态**: G1-G10 10/10 form-only PASS + G11-G15 infrastructure ready + G16 5/7
- **⚠️ 关键发现 (RC3_PLAN)**: **真实生产级覆盖率 ~35%** (form-only 验证, 非真实运行)
  - G1 TPC-H gate: 0/6 步骤实际跑 TPC-H (仅检查文件/编译/commit)
  - Soak tests: 模拟 CPU 循环 (合成延迟, 非真实查询)
  - 77 `#[ignore]` tests, 43 TBD perf placeholders, 0/22 wire TPC-H
- **调整后计划**: rc2 → rc3 (2-3 周) → rc4 (3-4 周 Z6G4) → ga (2-3 周 168h soak)
- **13 critical-path items**: 11 follow-up issues (#3221-#3231) 需关闭
- **GA 风险**: 2026-09-23 目标 at risk, 依赖 Z6G4 硬件 W14 之前可用

### 综合评级 (RC2 后期视角)

| 评级 | 结论 | 依据 |
|------|------|------|
| **v3.9.0 启动准备度** | ✅ READY (Phase 0 完成) | 分支 + 5 计划 + 6 文档就绪 |
| **Form-only 验证** | ✅ PASS (10/10 G1-G10) | 形式门禁全过, 真实运行待 rc3+ |
| **真实生产准备度** | ⏳ 35% (RC3_PLAN 揭示) | form-only 阶段不能算 production-ready |
| **Production Readiness** | ⏳ IN PROGRESS (延 7-10 周) | 13 critical-path items 关闭 + Z6G4 真实运行 |
| **v3.8.0 baseline 继承** | ✅ 完整继承 | 22/22 TPC-H in-process + 5 债关闭 + 188 文档 |
| **工程化战略** | ✅ 已批准 (v3.9.0 type) | ChatGPT 架构师 2026-06-05 评审通过 |

### 关键指标对照 (v3.8.0 → v3.9.0 目标 → 现状)

| 维度 | v3.8.0 (GA) | v3.9.0 目标 (GA) | v3.9.0 现状 (RC2 后期) |
|------|-------------|------------------|----------------------|
| **SQL Engine 综合** | 9.0/10 | ≥ 9.0 (不退化) | 9.0/10 (form-only 继承) |
| **TPC-H** | 22/22 PASS | 22/22 保持 | 22/22 in-process (PR #3213, Q11/Q14/Q22 fix) |
| **ACID / DML 主路径** | 8.5/10 | ≥ 9.0 (G4 强化) | G4 ARCH-3 form-only PASS |
| **跨版本债 OPEN** | 4 项 (INT-2/3, ARCH-3, SEM-1) | 0 项 (G2/G3/G4/G5) | **G2-G5 form-only PASS, 仍需真实验证** |
| **MySQL 5.7 兼容度** | 58/100 | 58-60/100 | 58/100 (frozen) |
| **生产可靠性 (Soak)** | 6.5/10 | ≥ 8.5 (G7 168h) | 6.5/10 (G7 compressed-time, real pending) |
| **GMP 审计能力** | 5.0/10 | ≥ 8.0 (G10) | 5.0/10 (G10 form-only PASS) |
| **真实生产级覆盖率** | 60-70% | ≥ 90% | **~35% (RC3_PLAN 揭示)** |
| **综合评分** | 8.4~8.7/10 | ≥ 8.7/10 | 7.0~7.5/10 (form-only 校准) |

---

## 1. 战略反转 (Strategic Inversion)

### 1.1 v3.8.0 之前 vs v3.9.0 之后

| 维度 | v3.7.0 之前 | v3.8.0 | v3.9.0 开始 |
|------|------------|--------|-------------|
| **核心问题** | SQL 能力够不够? | SQL 是否 GA? | **数据库死了还能不能回来?** |
| **关注指标** | 多少 TPC-H PASS | 22/22 TPC-H + INT-1 | **Soak 24h + Crash Matrix + Backup/Restore** |
| **资源分配** | 60% SQL + 30% ACID + 10% 性能 | 50% SQL + 35% ACID + 15% 性能 | **0% 新 SQL + 40% 架构债 + 35% 可靠性 + 15% GMP 审计 + 10% 性能** |
| **门禁体系** | L1-L6 (8 维) | L1-L6 (8 维, GA 73+/80) | **G1-G16 (16 维, 取代 L1-L6)** |
| **类型** | Feature Release | SQL Capability GA | **Production Readiness Release (工程化)** |

### 1.2 拒绝的版本定位

❌ **Feature Release** (继续堆 SQL 功能: Window Function / CTE / 高级函数, 收益下降)
❌ **Distributed Database** (v3.9.0 阶段分布式太早, 单节点可靠性都没验证)
❌ **Replication First** (单节点 master-slave 也需等 Soak Test 验证)

✅ **Production Readiness Release** (工程化 + 可靠性优先)
✅ **Single-Node Production Candidate** (单机生产就绪)
✅ **Engineering Release** (版本号继续递增, 但本质是工程化)

### 1.3 接受 ChatGPT 架构师 2026-06-05 评审建议

来源: `docs/releases/v3.9.0/plans/V390_VERSION_PLAN.md` §1.2

- v3.8.0 SQL 能力已饱和 (TPC-H 22/22 + INT-1 修复 + 5 债关闭)
- 继续加 SQL 功能收益递减
- 可靠性 / 可恢复性 / 可审计性 是下一个瓶颈
- 12 周 6 Phase 是合理估计 (实际延期 7-10 周)

---

## 2. v3.8.0 → v3.9.0 Baseline 继承关系

### 2.1 v3.8.0 收口时的核心状态 (继承基线)

来源: `docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md` v3.2

| 维度 | v3.8.0 状态 | v3.9.0 继承 |
|------|------------|-------------|
| **TPC-H** | 22/22 PASS (v3.8.0 最大成就) | ✅ 完整继承, G1 form-only 验证保持 |
| **SQL Engine 综合** | 9.0/10 | ✅ 完整继承 |
| **Parser** | 10/10 (18/18 PASS) | ✅ 完整继承 |
| **Executor** | 8.5/10 (22/22 TPC-H) | ✅ 完整继承 |
| **ACID** | 8.5/10 (INT-1 PR-3019) | ✅ 完整继承 |
| **MySQL 兼容度** | 58/100 | ✅ 冻结, v3.9.0 不主动提升 |
| **Coverage** | 81.62% | ✅ 保持 |
| **文档** | 188 文件 / 17 分类 | ✅ 完整继承 + 19 v3.9.0 新文档 |
| **5 跨版本债关闭** | INT-1, INT-4, ARCH-1, SEM-2, ARCH-3 Blocker-1+2 | ✅ 完整继承 |
| **4 跨版本债 OPEN** | INT-2, INT-3, ARCH-3 Complete, SEM-1 | ⏳ v3.9.0 G2/G3/G4/G5 form-only 关闭 |

### 2.2 v3.9.0 启动到 RC2 关键 commit 链

| Commit | 主题 | PR | 关联 |
|--------|------|----|----- |
| `3ed72a3e` | Phase 0 启动 (chore: v3.9.0 Production Readiness Release) | — | Phase 0 |
| `9eec6eca` | check_cross_version_debt.sh: 3 new code-reality checks | #3136 | 治理 |
| `b06226b0` | test authenticity audit (李哥 信任 concern, post-rc2) | (audit) | 关键发现 |
| `2e0e82b3` | RC2 阶段文档 + G11-G15 + 6 perf 报告 | #3218 | RC2 |
| `6a3823b4` | unignore 16 L3 acceptance tests | #3221 (in progress) | rc3 待完成 |
| `56009754` | TPC-H 22/22 PASS in-process on canonical SF=0.01 (Q11/Q14/Q22) | #3213 | 关键修复 |
| `8999b2a5` | [fix/tpch] TPC-H 22/22 in-process | #3213 | TPC-H |
| `d0e24b33` | G1 TPC-H 22/22 baseline hash + CI gate | #3186, #3198 | G1 门禁 |
| `5ac92629` | bypass SQL parser in bulk_insert — 60000-row 5min → 1ms (300000× faster) | #3233 | 性能优化 |
| `2c27a61d` | storage/wal_storage: Phase 1 of #3223 — active_txs HashMap tracking | #3240 | TX/WAL |
| `b02a7057` | tests/tx_wal: #3223 Phase 4 — ignore TX-004/005 (Sprint 3 autocommit conflict) | #3244 | TX/WAL (待 fix) |
| `26dfc6b4` | tests/tx_wal: Phase 2/3 #3223 — unignore 9 RECOVERY tests | #3243 | TX/WAL |
| `0f64e907` | eval_predicate Expression::Exists / NotExists — TPC-H Q4 matches | #3250 | TPC-H Q4 |
| `5f208f63` | Q22 NOT EXISTS (SELECT *) regression coverage | #3215 | TPC-H Q22 |
| `ae3e6afa` | TPC-H Q15 subquery in comma-join — cartesian JoinClause + DERIVED_ALIASES | #3241 | TPC-H Q15 |
| `4bc2d1bf` | clippy -D warnings: 12 fixes in admin + mysql-server (G2 lib gate) | #3254 | G2 |
| `d21ea4db` | docs(v3.9.0): Adjusted release plan (rc2 → rc3 → rc4 → ga, post-audit) | — | RC3 计划 |
| `0852e42e` (HEAD) | Merge #3254 clippy fixes | #3254 | RC2 末 |

### 2.3 v3.9.0 不会触碰的领域 (frozen)

- ❌ MySQL 5.7 新函数 (NATURAL/FULL OUTER/GROUP_CONCAT/DATE_SUB) — 推 v3.10+
- ❌ Window Function / CTE — 推 v3.10+
- ❌ 分布式 / Replication — 推 v3.10+
- ❌ F-30 SEQUENCE / F-36 列级权限 / F-03 GIS — 推 v3.10+
- ❌ SIMD SQL Executor 集成 — 推 v3.10+
- ❌ Optimizer 重大增强 (Statistics + Cost Model) — 推 v3.10+

---

## 3. v3.9.0 资源分配 (12 周 / 6 Phase / 451h / 16 任务)

来源: `docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md` §0

| 优先级 | 任务数 | 工作量 | 占比 | 阶段 | 主题 |
|--------|--------|--------|------|------|------|
| **P0 (必须)** | 4 | 130h | 29% | Phase 1-2 (W1-4) | 跨版本债关闭 (架构债 40%) |
| **P1 (生产可靠性)** | 4 | 168h | 37% | Phase 3-4 (W5-8) | Backup/Restore/Soak/Upgrade |
| **P2 (GMP 能力)** | 3 | 68h | 15% | Phase 5 (W9-10) | Audit + Time Travel |
| **P3 (性能优化)** | 5 | 85h | 19% | Phase 6 (W11-12) | Prepared Stmt / Stats / Cost / Parallel / SIMD |
| **合计** | **16** | **451h** | **100%** | **12 周 / 65 天** | — |

### 3.1 资源分配 vs 战略方向 (ChatGPT 建议)

| 方向 | 占比 | 工作量 (12 周) | 对应 P-level |
|------|------|----------------|-------------|
| 架构债 (INT/ARCH/SEM) | **40%** | 180h | P0 (130h) + P3 部分 (50h) |
| 可靠性 (Recovery/Backup/Soak) | **35%** | 158h | P1 (168h, 含部分 GMP) |
| GMP 审计能力 | **15%** | 68h | P2 (68h) |
| 性能优化 | **10%** | 45h | P3 部分 (35h) |
| **新 SQL 功能** | **0%** | **0h** | — (冻结) |
| **合计** | **100%** | **~451h** | — |

### 3.2 P0 任务 (4 项, 130h, 40%)

| 任务 | 主题 | 工作量 | Issue | 状态 (RC2 末) | 门禁 |
|------|------|--------|-------|--------------|------|
| **P0-1** | ARCH-3 完整闭环 (VtuGuard 主路径强制) | 40h | #3109 | ✅ G4 form-only PASS | G4 |
| **P0-2** | INT-3 收敛 (Single Expression Engine) | 32h | #3146 | ✅ G3 form-only PASS (14/14 delegation, PR #3200) | G3 |
| **P0-3** | INT-2 ParallelExecutor 主路径集成 | 30h | #3108 | ✅ G2 form-only PASS (clippy 12 fixes #3254) | G2 |
| **P0-4** | SEM-1 Savepoint MVCC 真实还原 | 28h | #3146 | ✅ G5 form-only PASS (8/8) | G5 |

**P0 总计: 130h, 全部 form-only 完成**

### 3.3 P1 任务 (4 项, 168h, 35%)

| 任务 | 主题 | 工作量 | Issue | 状态 (RC2 末) | 门禁 |
|------|------|--------|-------|--------------|------|
| **P1-1** | Backup/Restore 实现 (100+ 场景) | 40h | 待创建 | ✅ G6 form-only PASS (6/6) | G6 |
| **P1-2** | Crash Test Framework (100+ scenarios) | 40h | 待创建 | 🟡 G8 form-only PASS, real run pending | G8 |
| **P1-3** | Soak Test (24h / 72h / 168h) | 48h | 待创建 | 🟡 G7 compressed-time 10/10, real 24h/72h/168h pending | G7 |
| **P1-4** | Upgrade Test (v3.8 → v3.9) | 40h | 待创建 | ✅ G9 form-only PASS (5/5) | G9 |

**P1 总计: 168h, form-only 阶段, 真实运行 deferred to RC4/GA**

### 3.4 P2 任务 (3 项, 68h, 15%)

| 任务 | 主题 | 工作量 | Issue | 状态 (RC2 末) | 门禁 |
|------|------|--------|-------|--------------|------|
| **P2-1** | Audit Log (审计日志 + 系统表) | 24h | 待创建 | ✅ G10 form-only PASS (1 non-blocking warn) | G10 |
| **P2-2** | 时间旅行查询 (MVCC + AS OF TIMESTAMP) | 24h | 待创建 | (G10 包含) | G10 |
| **P2-3** | 不可篡改审计链 (Hash Chain) | 20h | 待创建 | (G10 包含) | G10 |

**P2 总计: 68h, form-only 阶段, 真实审计待 RC3**

### 3.5 P3 任务 (5 项, 85h, 10%)

| 任务 | 主题 | 工作量 | Issue | 状态 (RC2 末) | 备注 |
|------|------|--------|-------|--------------|------|
| **P3-1** | Prepared Statement Cache (LRU ≥ 80% 命中率) | 12h | 待创建 | ⏳ GA 前 | 12h 小任务 |
| **P3-2** | Statistics (ANALYZE TABLE) | 16h | 待创建 | ⏳ GA 前 | 基础统计 |
| **P3-3** | Cost Optimizer (基于 Statistics) | 24h | 待创建 | ⏳ GA 前 | JOIN 顺序 |
| **P3-4** | INT-2 ParallelExecutor 优化 (Worker 池 + 分片) | 18h | 待创建 | ⏳ GA 前 | 在 P0-3 集成基础上 |
| **P3-5** | SIMD 集成 SQL Executor (filter/aggregate) | 15h | 待创建 | ⏳ GA 前 | 谨慎收益验证 |

**P3 总计: 85h, GA 末 (W15-W16)**

---

## 4. G1-G16 门禁现状 (RC2 后期)

来源: `docs/releases/v3.9.0/rc/RC2_GATE_REPORT.md` 完整继承

### 4.1 门禁总览 (G1-G16)

| Gate | 主题 | 类型 | 状态 (RC2) | PR | 备注 |
|------|------|------|------------|-----|------|
| **G1** | 22/22 TPC-H 保持 | 回归 | ✅ PASS (6/6) | #3186 | ⚠️ form-only, 0/6 实际跑 TPC-H |
| **G2** | INT-2 ParallelExecutor 集成 | e2e + perf | ✅ PASS (6/6) | #3187 | + clippy #3254 |
| **G3** | INT-3 Single Expression | 单测 + 回归 | ✅ PASS (4/4) | #3188, #3200 | 14/14 delegation |
| **G4** | ARCH-3 Complete (VtuGuard 主路径) | 单元 + gate | ✅ PASS (4/4) | #3189 | 主路径强制 |
| **G5** | SEM-1 Savepoint (MVCC 真实还原) | 单元 + e2e | ✅ PASS (8/8) | #3190 | 8/8 tests |
| **G6** | Backup/Restore 100+ scenarios | 单元 + e2e | ✅ PASS (6/6) | #3191 | form-only |
| **G7** | 24h Soak | 长周期 | ✅ PASS (7/7) | #3192 | ⚠️ compressed-time, real 24h pending |
| **G8** | Crash Matrix 100+ scenarios | 单元 + e2e | ✅ PASS (7/7) | #3193 | ⚠️ form-only, real 8 categories pending |
| **G9** | Upgrade Test 50+ scenarios | 自动化 | ✅ PASS (5/5) | #3194 | form-only |
| **G10** | Audit Log + 时间旅行 | 单元 + e2e | 🟡 PASS (7/7+1warn) | #3195 | non-blocking |
| **G11** | QPS/TPS Baseline | perf | 🟡 infra ready | — | real 60min pending Z6G4 |
| **G12** | Sysbench OLTP | perf | 🟡 infra ready | — | real pending sysbench binary |
| **G13** | 24h 真实稳定性 | 长周期 | 🟡 infra ready | — | real pending Z6G4 |
| **G14** | 真实崩溃 8 类 | 长周期 | 🟡 infra ready | — | real pending Z6G4 |
| **G15** | 汇总报告 | 报告 | ✅ PASS (4/4) | — | 6 perf reports committed |
| **G16** | Compatibility v3.8 → v3.9 | 集成 | 🟡 5/7 PASS | — | TPC-H step + REPORT step pending |
| **合计** | — | — | **10/10 form-only + G11-G15 infra + G16 5/7** | — | **真实生产级 35%** |

### 4.2 G1 — 22/22 TPC-H 保持 (form-only 已知问题)

**状态**: ✅ PASS (6/6 form-only steps)
- 22 TPC-H query files (q1..q22) ✅
- `tests/tpch_full_22_test.rs` Q1..Q22 runner ✅
- v3.8.0 GA_GATE_REPORT.md baseline ✅
- TPC-H test compilation ✅
- `tpch_22_queries_wire_test` 22 refs ✅
- Recent commit regression guard ✅ INFO

**⚠️ RC3_PLAN 关键发现**: **G1 TPC-H gate 0/6 steps 实际跑 TPC-H** (仅检查文件存在、编译、commit log)

**PR #3213 fix**: Q11/Q14/Q22 → 22/22 in-process on canonical SF=0.01
- `56009754` Merge PR #3213
- `8999b2a5` [fix/tpch] TPC-H 22/22 in-process

**未完成**:
- 0/22 wire TPC-H (queries not in wire protocol)
- real 22/22 SHA-256 baseline (item #3231, RC4)

### 4.3 G2 — INT-2 ParallelExecutor 集成 (PASS)

**状态**: ✅ PASS (6/6)
- 单元 + e2e + perf 7+ tests
- TPC-H Q1 SF=1 4 worker ≤ sequential 1.5x
- `src/execution_engine.rs` grep "ParallelExecutor" ≥ 1
- PR #3254: clippy -D warnings 12 fixes (G2 lib gate)

### 4.4 G3 — INT-3 Single Expression (PASS)

**状态**: ✅ PASS (4/4)
- 14 个委托分支各 1 个测试
- TPC-H 22/22 不退化
- `src/expr_utils.rs` 行数 < 533

**PR #3200**: INT-3 Literal branch delegation (1/14) — 完成全 14/14
- `736861db` Merge PR #3200
- `d0e24b33` G1 TPC-H 22/22 baseline hash + CI gate

### 4.5 G4 — ARCH-3 Complete (PASS)

**状态**: ✅ PASS (4/4)
- 5+ tests PASS
- `grep -r "storage\.insert\|storage\.update\|storage\.delete"` = 0 匹配
- `check_arch2_no_bypass.sh` 全部 PASS
- TPC-H 22/22 不退化

### 4.6 G5 — SEM-1 Savepoint MVCC (PASS)

**状态**: ✅ PASS (8/8)
- 8+ tests PASS
- TPC-H 22/22 不退化
- ROLLBACK TO SAVEPOINT 真正还原 tuple 状态

### 4.7 G6 — Backup/Restore (PASS)

**状态**: ✅ PASS (6/6 form-only)
- 100+ tests PASS
- CLI `sqlrustgo backup/restore/verify` 全部可用 (form-only)
- PITR 时间精度 ≤ 1s

### 4.8 G7 — 24h Soak (compressed-time 已知问题)

**状态**: ✅ PASS (7/7) — **但 RC3_PLAN 揭示真实问题**:
- 10 soak tests PASS at 24h/72h/168h
- Memory growth < 10%
- FD growth = 0
- Lock growth = 0
- p99 latency bounded

**⚠️ RC3_PLAN 关键发现**: **Soak tests: 模拟 CPU 循环** (合成延迟, 非真实查询)

**未完成**:
- 24h real wall-clock soak (RC4 + GA-final, Z6G4)
- 72h real wall-clock soak (RC4)
- 168h real wall-clock soak (GA-final, 7 days)

### 4.9 G8 — Crash Matrix (form-only 已知问题)

**状态**: ✅ PASS (7/7 form-only)
- 8 类崩溃注入 × 10+ 变体 模拟通过
- Crash 后数据一致性验证

**⚠️ RC3_PLAN 关键发现**: Crash matrix 模拟通过, real crash 8 categories pending

**未完成** (RC4 + GA-final):
- SIGKILL real (kill -9)
- SIGTERM real
- Power loss real
- Disk full real
- OOM real
- WAL corruption real
- Clock skew real
- Network partition (v3.10+)

### 4.10 G9 — Upgrade Test (PASS)

**状态**: ✅ PASS (5/5)
- 50+ tests PASS
- v3.8.0 dump → v3.9.0 load → 数据完整

### 4.11 G10 — GMP Audit (PASS with warn)

**状态**: 🟡 PASS (7/7 + 1 sub-gate WARN non-blocking)
- Audit log 系统表
- 时间旅行查询
- Hash chain 完整性

**Sub-warn**: P2-2 smoke step 7 timeout (180s+), deferred to v3.9.1+

### 4.12 G11-G15 — Performance Gates (infrastructure ready)

| Gate | 状态 | Real run 计划 |
|------|------|--------------|
| **G11** QPS/TPS Baseline | 🟡 infra ready | GA-final on Z6G4 (60+ min) |
| **G12** Sysbench OLTP | 🟡 infra ready | GA-final (sysbench binary install) |
| **G13** 24h 真实稳定性 | 🟡 infra ready | GA-final on Z6G4 |
| **G14** 真实崩溃 8 类 | 🟡 infra ready | GA-final on Z6G4 |
| **G15** 汇总报告 | ✅ PASS (4/4) | 6 perf reports committed |

### 4.13 G16 — Compatibility v3.8 → v3.9

**状态**: 🟡 5/7 PASS
- 4 case scripts present
- Rollback script present
- Tests registered in Cargo.toml
- 18 compat tests pass
- 5 harness tests pass
- TPC-H 22/22 maintained
- COMPATIBILITY_REPORT.md exists (pending)

**Auto-OK by inheritance** for TPC-H step + REPORT step

---

## 5. 完整阶段历程 (Alpha1 → RC2)

来源: `docs/releases/v3.9.0/rc/RC2_GATE_REPORT.md` §6 完整继承

| Stage | Tag | Commit | Sub-tasks | G-gates | Soak | Date |
|-------|-----|--------|-----------|---------|------|------|
| **Alpha1** | `v3.9.0-alpha1` | `61509ae5` | 0/16 (entry) | - | - | 2026-06-05 |
| **Beta** | `v3.9.0-beta` | `c71b609f` | 16/16 | 10/10 | 10/10 | 2026-06-05 |
| **RC1** | `v3.9.0-rc1` | `29e2475f` | 16/16 | 10/10 + G11/G12/G16 infra | 10/10 | 2026-06-05 |
| **RC2** | `v3.9.0-rc2` | `76efe391` / `82b82204` | 16/16 | 10/10 + G11-G15 infra + 6 perf reports | 10/10 (compressed) | 2026-06-05 |
| **RC3** | (planned) | ⏳ | TBD | P0 关闭 + G1/G8/G10 real | real 24h+ | 2026-06-20 估计 |
| **RC4** | (planned) | ⏳ | TBD | P1 关闭 + G7/G11-G15 real | real 72h+ | 2026-07-15 估计 |
| **GA** | (planned) | ⏳ | 16/16 | 10/10 + 168h real | 168h | 2026-09-23 (at risk) |

### 5.1 Beta 阶段详情 (16/16 子任务完成)

来源: `docs/releases/v3.9.0/beta/BETA_RELEASE_NOTES.md`

- 16/16 sub-tasks completed (100%)
- 10/10 G1-G10 gates (G10 non-blocking warn)
- 72h soak compressed-time: 10/10 tests PASS (1,440× compression)
- TPC-H 22/22 baseline PASS (inherited from v3.8.0-rc1)
- Doc gates PASS (check_docs_consistency.sh + check_docs_links.sh)

### 5.2 RC1 阶段详情 (10/10 G1-G10 + G11/G12/G16 infra)

来源: `docs/releases/v3.9.0/rc/RC1_GATE_REPORT.md`

- G1-G10 baseline: 10/10 PASS
- G11 QPS infra: ready
- G12 Sysbench infra: ready
- G16 Compatibility: 5/7 PASS (TPC-H step + REPORT step pending)

### 5.3 RC2 阶段详情 (10/10 + G11-G15 infra + 6 perf reports)

来源: `docs/releases/v3.9.0/rc/RC2_GATE_REPORT.md`

- G1-G10: 10/10 PASS
- G11-G15: infrastructure ready
- 6 perf reports (1018 lines): QPS/SYSBENCH/STABILITY/CRASH/COMPATIBILITY/PERFORMANCE_BASELINE
- 72h Soak: 10/10 PASS (compressed)
- 168h Soak: planned for GA-final

### 5.4 RC2 后期 (现状) — 关键发现

**RC3_PLAN 揭示的真相**:

v3.9.0-rc2 (`76efe391` / `82b82204`) 是 **form-only validation milestone**, 不是 production-ready:

- G1 TPC-H gate: **0/6 steps actually run TPC-H** (checks file existence, compilation, commit log)
- Soak tests: **simulated CPU loop** (synthetic latencies, not real queries)
- 77 `#[ignore]` tests
- 43 TBD perf placeholders
- 0/22 wire TPC-H
- **Real production-equivalent coverage: ~35%** (not 90%+ implied by previous reports)

**v3.9.0-rc2 tag is NOT reverted** (per user direction). It is documented as a **form-only validation milestone**, not a production-ready candidate.

---

## 6. 13 Critical-Path Items (RC3 → RC4 → GA)

来源: `docs/releases/v3.9.0/rc/RC3_PLAN.md` §3 完整继承

### 6.1 13 Critical-Path Items 总览

| # | Item | Issue | Priority | Phase | 状态 (RC2 末) |
|---|------|-------|----------|-------|--------------|
| 1 | Server `LOAD DATA` perf | #3222 | P0 | rc3 | ⏳ open |
| 2 | Replace corrupt SF=0.01 fixture | #3227 | P0 | rc3 | ⏳ open |
| 3 | L3 acceptance + 15 e2e unignore | #3221 | P0 | rc3 | 🟡 in progress (PR #3246 已 merge 15 e2e + 1 L3) |
| 4 | Unignore 10 long_run_stability | #3228 | P1 | rc4 | ⏳ open |
| 5-7 | Real 24h/72h/168h soak | #3225, #3229 | P1 | rc4 + ga | ⏳ open |
| 8-9 | QPS bench + fill perf baseline | #3224 | P1 | rc4 | ⏳ open |
| 10 | TX/WAL fix #2870 | #3223 | P0 | rc3 | 🟡 in progress (Phase 1-4 推进, #3244 Phase 4 ignore TX-004/005) |
| 11 | Unignore 8 wire_smoke_sf | #3230 | P0 | rc3 | ⏳ open |
| 12 | TPC-H 22/22 SHA-256 capture | #3231 | P1 | rc4 | ⏳ open |
| 13 | l3_canonical_binary acceptance | #3221 (combined) | P0 | rc3 | 🟡 in progress |
| Extra | TPC-H Q8/Q9 真 bug fix | #3226 | P2 | (any) | ⏳ open |

**Total: 11 follow-up issues covering 13+ items.**

### 6.2 RC3 cut criteria (P0 cut)

**目标**: All P0 governance blockers closed (no `#[ignore]` for critical correctness, no TX/WAL gaps).

**Cut criteria**:
- [ ] G1 gate re-engineered to actually run TPC-H 22/22 (with `TPCH_FORCE=1`)
- [ ] G8 Crash Matrix + G16 Compatibility (real, not simulated)
- [ ] All P0 `#[ignore]` tests either closed (un-ignored + passing) or explicitly deferred
- [ ] Doc gates PASS

**Required closed issues** (5 P0):
- [ ] #3221 L3 acceptance + unignore 15 e2e_canonical
- [ ] #3222 Server `LOAD DATA` perf fix
- [ ] #3223 Storage tx tracking + unignore 9 TX/WAL
- [ ] #3227 Replace corrupt SF=0.01 fixture
- [ ] #3230 Unignore 8 wire_smoke_sf (value-correctness)

**Estimated**: 2-3 weeks of focused work on server perf + L3 implementation.

### 6.3 RC4 cut criteria (P1 cut, after Z6G4 real runs)

**目标**: All Z6G4 real-run critical-path items closed.

**Required closed issues** (4 + 1 partial):
- [ ] #3224 Z6G4 perf measurement + fill PERFORMANCE_BASELINE.md (43 TBD → 0)
- [ ] #3225 Real 24h/72h wall-clock soak
- [ ] #3228 Unignore 10 long_run_stability
- [ ] #3229 Real 168h wall-clock soak (7 days)
- [ ] #3231 Capture TPC-H 22/22 SHA-256 baseline

**Cut criteria**:
- [ ] 24h real soak PASS (memory < 10%, FD = 0, lock = 0)
- [ ] 72h real soak PASS
- [ ] 168h real soak kick-off (continued into GA)
- [ ] PERFORMANCE_BASELINE.md 43 TBD → 0 TBD
- [ ] TPC-H 22/22 SHA-256 captured on Z6G4 canonical SF=0.01
- [ ] All G11/G12/G13/G14/G15 (perf + crash) actually run, not just infra

**Estimated**: 3-4 weeks on Z6G4 hardware (depends on availability).

### 6.4 GA cut criteria (final, after all 13 closed + 168h real soak)

**目标**: Production-ready release.

**Required closed issues**: ALL 13 critical-path items + 1 extra (#3226 TPC-H Q8/Q9 fix).

**Cut criteria**:
- [ ] All 11 follow-up issues closed (#3221-#3231)
- [ ] 168h real soak completed successfully
- [ ] All G1-G16 gates run real validation (not form-only)
- [ ] Doc gates PASS (real data, not TBD)
- [ ] GA_RELEASE_NOTES.md + GA_GATE_REPORT.md written
- [ ] `release/v3.9.0` branch cut (maintenance)

**Estimated GA date**: 2026-09-23 (per V390 plan) — at risk if Z6G4 hardware not available by W14.

---

## 7. 性能报告 (W11-W12 阶段产出)

来源: `docs/releases/v3.9.0/perf/*` 完整继承

| 报告 | 行数 | 状态 | 备注 |
|------|------|------|------|
| `docs/releases/v3.9.0/perf/PERFORMANCE_REPORT.md` (master) | 235 | ✅ | W12 master |
| `docs/releases/v3.9.0/perf/QPS_REPORT.md` | 115 | ✅ |  |
| `docs/releases/v3.9.0/perf/SYSBENCH_REPORT.md` | 85 | ✅ |  |
| `docs/releases/v3.9.0/perf/STABILITY_REPORT.md` | 101 | ✅ |  |
| `docs/releases/v3.9.0/perf/CRASH_TEST_REPORT.md` | 96 | ✅ |  |
| `docs/releases/v3.9.0/perf/COMPATIBILITY_REPORT.md` | 110 | ✅ |  |
| `docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md` | 176 | 🟡 | 43 TBD placeholders |
| `docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md` | TBD | ✅ | 4-Way TPC-H horizontal |

**Total**: 7 perf docs, ~918 lines (1018 if all sub-reports included).

### 7.1 关键性能数据 (form-only / compressed)

- **TPC-H 22/22 in-process** (PR #3213)
- **bulk_insert bypass** (PR #3233): 60000-row lineitem LOAD DATA 5min → 1ms (300000× faster)
- **72h Soak (compressed)**: 10/10 PASS, memory < 10%, FD = 0, lock = 0, p99 bounded
- **TPC-H Q4/Q15/Q22 fix**: PR #3250 (Q4 Exists), #3241 (Q15 comma-join), #3215 (Q22 NOT EXISTS)

### 7.2 性能报告待 real run (RC4/GA)

- G11 QPS bench: real run 60+ min, deferred to GA-final on Z6G4
- G12 Sysbench: install + run, deferred to GA-final
- G13 24h real stability: W15-W16 (1 day wall-clock) on Z6G4
- G14 8 real crash categories: W15-W16 on Z6G4
- G15 PERFORMANCE_BASELINE: 43 TBD → 0 TBD (rc4)

---

## 8. 6 Phase 路线图 (12 周 / 65 工作日 / 451h)

来源: `docs/releases/v3.9.0/ROADMAP.md` §0 完整继承

```
W0      W1    W2    W3    W4    W5    W6    W7    W8    W9    W10   W11   W12   W13  W14  W15  W16
|-------|------|------|------|------|------|------|------|------|------|------|------|------|----|----|----|
Phase0  Phase1      Phase2      Phase3      Phase4      Phase5      Phase6      [RC3][RC4][GA ]
 启动    ARCH-3      INT-2       Backup      Soak        Audit       性能+       调整后
         INT-3       Savepoint   Crash       Upgrade     TimeTrav    GA
```

| Phase | 周 | 工作日 | 工时 | 主题 | 关键任务 | 门禁 | 状态 |
|-------|----|----|----|------|----------|------|------|
| 0 | W0 | 5 | 30h | 启动 (分支 + SPEC) | 5 plan 文档 + 启动 commit | — | ✅ DONE |
| 1 | W1-2 | 10 | 80h | ARCH-3 + INT-3 | VTU 主路径 + expr 完整合并 | G3, G4 | ✅ form-only |
| 2 | W3-4 | 10 | 90h | INT-2 + Savepoint | TransactionManager 集成 + Savepoint 完整 | G2, G5 | ✅ form-only |
| 3 | W5-6 | 10 | 95h | Backup/Restore + Crash Matrix | 100+ 场景 + Crash Injector | G6, G8 | 🟡 form-only |
| 4 | W7-8 | 10 | 70h | Soak + Upgrade | 24h 浸泡 + 50+ 升级 | G7, G9 | 🟡 form-only + compressed |
| 5 | W9-10 | 10 | 50h | Audit + Time Travel | GMP 审计 + 历史快照 | G10 | 🟡 form-only |
| 6 | W11-12 | 10 | 36h | 性能优化 + GA 收口 | 性能调优 + GA 报告 | G1 保持 | 🟡 partial |
| **总计** | **12 周** | **65 天** | **451h** | — | — | — | **form-only** |
| **RC3** | W13-14 | 10-15 | — | P0 cut | 5 issues closed + G1/G8/G10 real | — | ⏳ planned |
| **RC4** | W15-18 | 15-20 | — | P1 cut | 4 issues closed + G7/G11-G15 real on Z6G4 | — | ⏳ planned |
| **GA** | W19-22 | 10-15 | — | GA cut | All 13 + 168h real soak | — | ⏳ planned |

### 8.1 时间表 (调整后)

| 日期 | 阶段 | 目标 | 状态 |
|------|------|------|------|
| W0 (2026-06-05) | Phase 0 收口 | develop/v3.9.0 创建, 5 SPEC 完成 | ✅ DONE |
| W2 (2026-06-05) | Phase 1 收口 | ARCH-3 + INT-3 关闭 (form-only) | ✅ DONE |
| W4 (2026-06-05) | Phase 2 收口 | INT-2 + Savepoint 关闭 (form-only) | ✅ DONE |
| W6 (2026-06-05) | Phase 3 收口 | Backup/Restore + Crash Matrix (form-only) | ✅ form-only |
| W8 (2026-06-05) | Phase 4 收口 | Soak + Upgrade (form-only + compressed) | ✅ form-only |
| W10 (2026-06-05) | Phase 5 收口 | GMP 审计 + 时间旅行 (form-only) | 🟡 form-only |
| W12 (2026-06-05) | Phase 6 收口 | 性能优化 + 6 perf 报告 | 🟡 partial |
| **2026-06-05** | **RC2 cut** | form-only validation milestone | ✅ DONE |
| **2026-06-20** | RC3 cut (estimate) | 5 P0 issues closed | ⏳ planned |
| **2026-07-15** | RC4 cut (estimate) | 4 P1 issues + Z6G4 real runs | ⏳ planned |
| **2026-09-23** | GA cut (estimate) | All 13 + 168h real soak | ⏳ at risk |

---

## 9. v3.9.0 风险评估 (更新)

### 9.1 高风险项 (新增 + 调整)

| 风险 | 等级 | 缓解 | 状态 |
|------|------|------|------|
| **Z6G4 硬件不可用** (W14 之前) | 🟠 中-高 | RC4 计划延 1-2 周, GA 顺延 | ⏳ 待确认 |
| **#3223 TX/WAL Phase 4 autocommit conflict** | 🟠 中-高 | Sprint 3 autocommit 修复 (TX-004/005 当前 ignored) | ⏳ fix in RC3 |
| **77 #[ignore] tests 关闭** | 🟠 中-高 | RC3/RC4 强制 unignore + 验证 | ⏳ in progress |
| **43 TBD perf placeholders** | 🟡 中 | RC4 real run on Z6G4 | ⏳ planned |
| **168h real soak (7 days)** | 🟠 中-高 | RC4 启动, GA-final 完成 | ⏳ planned |
| **TPC-H 22/22 wire protocol (0/22)** | 🟠 中-高 | wire TPC-H 实现, RC4 验证 | ⏳ open |
| **G10 P2-2 smoke step 7 timeout** | 🟢 低 | P2-2 perf optimization, v3.9.1+ | ⏳ deferred |
| **Form-only 验证不充分** | 🔴 已发生 | RC3_PLAN 已揭示, 调整计划 | ✅ mitigated |

### 9.2 不在本版本范围 (Frozen for v3.9.0)

- ❌ Window Function / CTE (推 v3.10+)
- ❌ 分布式 / Replication (推 v3.10+)
- ❌ F-30 SEQUENCE / F-36 列级权限 / F-03 GIS (推 v3.10+)
- ❌ SIMD 集成 (推 v3.10+)
- ❌ MySQL 5.7 新函数 (NATURAL/FULL OUTER/GROUP_CONCAT/DATE_SUB) — 推 v3.10+

### 9.3 中等风险项

| 风险 | 等级 | 缓解 |
|------|------|------|
| Backup/Restore 100+ 场景真实运行 | 🟡 中 | 并行化测试, RC4 Z6G4 |
| 168h Soak 性能不达标 | 🟡 中 | GA-final 性能优化预留 2 周 |
| Upgrade 兼容 v3.6/3.7/3.8 三版本 | 🟡 中 | v3.9.0 只保证 v3.8 → v3.9 |
| L3 acceptance test (l3_canonical_binary) | 🟡 中 | #3221 持续推进 |
| bulk_insert 60000-row 性能 | 🟢 已优化 | 5min → 1ms (300000× faster) |

---

## 10. 历史债务 (v3.8.0 → v3.9.0)

### 10.1 v3.8.0 关闭的 5 跨版本债 (继承)

| Issue | 主题 | 关闭 PR | v3.9.0 状态 |
|-------|------|---------|-------------|
| **#2966** | INT-1 DML Bypass (P0 Release Blocker) | PR-3019 | ✅ 完整继承 |
| **#3129** | ARCH-3 Blocker-1+2 (MemoryStorage TX) | PR-3152 | ✅ 完整继承 |
| **#3100** | check_int_debt.sh 路径 BUG | PR-3112 | ✅ 完整继承 |
| **#3101** | check_arch2_no_bypass.sh FAIL | PR-3118 | ✅ 完整继承 |
| **#3104-107, #3111** | 文档一致性 | PR-3134 | ✅ 完整继承 |

### 10.2 v3.9.0 form-only 关闭的 4 跨版本债 (G2/G3/G4/G5)

| Issue | 主题 | v3.9.0 任务 | 门禁 | 状态 (RC2 末) |
|-------|------|------------|------|--------------|
| **#3108** | INT-2/INT-3 集成 | P0-2 (INT-3) + P0-3 (INT-2) | G2, G3 | ✅ form-only PASS (PR #3187, #3188, #3200, #3254) |
| **#3109** | ARCH-3 VTU 主路径集成 | P0-1 | G4 | ✅ form-only PASS (PR #3189) |
| **#3117** | openclaw_endpoints.rs:2208/2288 真实 DML bypass | (含 P0-1 范围) | G4 | ✅ form-only PASS |
| **#3146** | INT-3 expr 完整合并 + INT-2 ParallelExecutor 集成 | P0-2 (INT-3) + P0-3 (INT-2) | G2, G3 | ✅ form-only PASS (14/14 delegation) |

**⚠️ 校准**: form-only 关闭 ≠ 真实生产级关闭. RC3/RC4/GA 需真实运行验证.

### 10.3 v3.9.0 仍 OPEN 的 11 follow-up issues (RC3 → GA)

| Issue | 主题 | 优先级 | 阶段 |
|-------|------|--------|------|
| **#3221** | L3 acceptance + unignore 15 e2e + 1 L3 | P0 | rc3 |
| **#3222** | Server `LOAD DATA` perf | P0 | rc3 |
| **#3223** | Storage tx tracking + unignore 9 TX/WAL | P0 | rc3 |
| **#3224** | Z6G4 perf baseline + fill PERFORMANCE_BASELINE | P1 | rc4 |
| **#3225** | Real 24h/72h wall-clock soak | P1 | rc4 |
| **#3226** | TPC-H Q8/Q9 真 bug fix | P2 | (any) |
| **#3227** | Replace corrupt SF=0.01 fixture | P0 | rc3 |
| **#3228** | Unignore 10 long_run_stability | P1 | rc4 |
| **#3229** | Real 168h wall-clock soak (7 days) | P1 | ga |
| **#3230** | Unignore 8 wire_smoke_sf (value-correctness) | P0 | rc3 |
| **#3231** | TPC-H 22/22 SHA-256 baseline | P1 | rc4 |

---

## 11. 文档治理 (v3.9.0 视角)

### 11.1 v3.9.0 文档完整度 (RC2 末)

```
v3.9.0/
├── README.md                          ✅ DONE (1.0)
├── CHANGELOG.md                       ✅ DONE (1.0)
├── ROADMAP.md                         ✅ DONE (1.0)
├── V390_COMPREHENSIVE_ASSESSMENT.md   ✅ NEW (本文件, RC2 后期视角)
│
├── alpha/                             (待创建或 alpha1 报告已 merge)
├── beta/
│   ├── BETA_RELEASE_NOTES.md          ✅ DONE
│   └── SOAK_72H_REPORT.md             ✅ DONE
├── rc/
│   ├── RC1_GATE_REPORT.md             ✅ DONE
│   ├── RC1_RELEASE_NOTES.md           ✅ DONE
│   ├── RC2_GATE_REPORT.md             ✅ DONE
│   ├── RC2_RELEASE_NOTES.md           ✅ DONE
│   └── RC3_PLAN.md                    ✅ DONE (key findings)
├── ga/                                ⏳ (RC4/GA 前待创建)
├── debt/                              ⏳ (待创建)
│
├── plans/
│   ├── V390_VERSION_PLAN.md           ✅ DONE
│   ├── V390_DEVELOPMENT_PLAN.md       ✅ DONE
│   ├── V390_TEST_PLAN.md              ✅ DONE
│   ├── V390_TEST_PLAN_ROUND2_REVIEW.md ✅ DONE
│   └── V390_TEST_PLAN_SUPPLEMENT_PERF.md ✅ DONE
│
└── perf/
    ├── PERFORMANCE_REPORT.md          ✅ DONE
    ├── QPS_REPORT.md                  ✅ DONE
    ├── SYSBENCH_REPORT.md             ✅ DONE
    ├── STABILITY_REPORT.md            ✅ DONE
    ├── CRASH_TEST_REPORT.md           ✅ DONE
    ├── COMPATIBILITY_REPORT.md        ✅ DONE
    ├── PERFORMANCE_BASELINE.md        🟡 (43 TBD)
    └── FOUR_WAY_TPCH_REPORT.md        ✅ DONE
```

**文档完整度**: 19+ v3.9.0 文档 (vs V380 文档 188 文件, v3.9.0 体量小但聚焦)

### 11.2 强制执行流程

v3.9.0 严格遵守:
- `docs/governance/DOC_CHECK_CORRECTION_RULES.md` (5 步流程)
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md` (Issue 关闭验证)
- `docs/governance/AI_COLLABORATION.md` (AI 协作规范)
- `docs/governance/ANTI_FABRICATION_POLICY.md` (真实数据零容忍)

### 11.3 5-类文档 (v3.9.0 SPEC 体系)

按 v3.8.0 治理经验, v3.9.0 每 P0/P1/P2 任务必须有:
- SPEC (规格)
- TEST_PLAN (测试计划)
- TEST_DESIGN (测试设计)
- REVIEW (评审)
- ACCEPTANCE (验收)

---

## 12. 规则治理 (继承 v3.8.0 10/10 + G1-G16 扩展)

来源: V380 §11 完整继承

| 规则 | 状态 |
|------|------|
| **5-原则** (有计划/有测试/必审/必集成/未过必记) | ✅ 100% 部署 |
| **9 维门禁** (D1-D9) | ✅ 100% 部署 |
| **G1-G16 新门禁** (v3.9.0) | ✅ 100% 部署 |
| **Issue 关闭验证** | ✅ 强制 PR 关联 |
| **DOC 修改 5 步流程** | ✅ 强制执行 |
| **AI Agent Task Claim Protocol** | ✅ 部署 |
| **ANTI_FABRICATION_POLICY** | ✅ 部署 (Truthfulness 零容忍) |
| **Gate Effectiveness Matrix** | ✅ 部署 (TP/FP/FN/TN 框架) |
| **CODEOWNERS** | ✅ Multi-reviewer |
| **PR 模板** | ✅ 5-类文档 + 5-原则 |
| **CI YAML 强制门禁** | ✅ 部署 |
| **G-Gate 区分 (form-only vs real)** (v3.9.0) | ✅ 新增 (RC3_PLAN 揭示) |

**规则治理覆盖率**: **12/12 = 100%** ✅ (v3.8.0 10/10 + v3.9.0 G1-G16 + form/real 区分)

---

## 13. v3.8.0 验证基线 (v3.9.0 不退化承诺)

来源: V380 §18 (GA_GATE_REPORT.md 2026-06-05) 完整继承

| # | GA 维度 | v3.8.0 状态 | v3.9.0 形式 | v3.9.0 真实 (校准) |
|---|---------|------------|------------|---------------------|
| 1 | L1 Unit Correctness | ✅ 10/10 | ✅ G3 + G4 保持 | ✅ |
| 2 | L2 Execution Consistency | ✅ 5/5 (TPC-H 22/22) | ✅ G1 形式 | 🟡 in-process 22/22, 0/22 wire |
| 3 | L3 ACID Verification | ✅ 49/49 | ✅ G4 + G5 形式 | 🟡 #3223 仍 in progress |
| 4 | L4 Architecture | ✅ 5/5 | ✅ G4 形式 | ✅ |
| 5 | L5 Performance | ✅ TPC-H 22/22 + 84% coverage | ✅ G1 形式 | 🟡 perf baseline 43 TBD |
| 6 | L6 Documentation | ✅ 5/5 | ✅ 5 步流程 | ✅ |
| 7 | CV Cross-Version Debt | ⚠️ 2 CLOSED + 2 ACTIVE | ✅ G2 + G3 形式 | 🟡 form-only 关闭 |
| **新增 G7** | 24h Soak | ❌ 未跑 | ✅ G7 形式 | ❌ compressed only, real 24h pending |
| **新增 G8** | Crash Matrix | ❌ 未跑 | ✅ G8 形式 | ❌ mock only, real 8 categories pending |
| **新增 G9** | Upgrade Test | ❌ 未跑 | ✅ G9 形式 | 🟡 form-only |
| **新增 G10** | Audit + Time Travel | ❌ 未实现 | ✅ G10 形式 | 🟡 form-only |
| **新增 G11-G15** | 性能门禁 (G11-G15) | — | ✅ infra ready | ❌ real run pending Z6G4 |
| **新增 G16** | Compatibility v3.8→v3.9 | — | 🟡 5/7 | 🟡 TPC-H + REPORT step pending |

**v3.8.0 GA Gate 总分**: 73+/80 ≥ 56 → ✅ PASS (继承)
**v3.9.0 形式 GA Gate 总分**: 80+/100 (G1-G16 形式全 PASS) → ✅ FORM PASS
**v3.9.0 真实 GA Gate 总分目标**: 80+/100 (G1-G16 真实全 PASS) → ⏳ GA cut 时验证

---

## 14. ChatGPT 评审对齐 (v3.9.0 战略接受)

来源: ChatGPT 架构师 2026-06-05 评审

### 14.1 v3.8.0 ChatGPT 评审核心 (继承)

- TPC-H 22/22 = v3.8.0 最大成就
- INT-1 (PR-3019) 修复 = Release Blocker 解除 (比 TPC-H 更重要)
- ARCH-3 严格分级: Blocker-1/2 CLOSED, Complete OPEN
- 并行能力 3.5/10 (I-12 capability ≠ integration)
- MySQL 5.7 兼容度 58/100 (校准)
- v3.8.0 综合 8.4~8.7/10 (实用型数据库引擎)

### 14.2 v3.9.0 ChatGPT 评审 (2026-06-05, 战略反转)

| 评审 # | 主题 | 采纳 |
|--------|------|------|
| #1 | SQL 能力饱和, 转向工程化 | ✅ 采纳 → v3.9.0 = Production Readiness Release |
| #2 | 资源分配 40% 架构债 / 35% 可靠性 / 15% GMP / 10% 性能 / 0% 新 SQL | ✅ 采纳 |
| #3 | 双后端架构 (SQLRustGo + PostgreSQL) | ⏳ v3.10+ 规划 |
| #4 | 12 周 6 Phase 计划 | ✅ 采纳 (调整后延 7-10 周) |
| #5 | G1-G10 新门禁 | ✅ 采纳 (扩展为 G1-G16) |
| #6 | 不再新加 SQL 功能 | ✅ 采纳 (frozen for v3.9.0) |
| #7 | 核心问题反转 (数据库死了还能不能回来) | ✅ 采纳 |

### 14.3 v3.9.0 自审 (RC2 后期, 2026-06-07)

| 维度 | v3.8.0 评审结论 | v3.9.0 形式完成 | v3.9.0 真实评估 (校准) |
|------|---------------|---------------|----------------------|
| 形式门禁全过 ≠ production-ready | — | ⚠️ RC3_PLAN 揭示 | **G1/G7/G8/G10 form-only** |
| 77 `#[ignore]` + 43 TBD + 0/22 wire TPC-H | — | ⚠️ RC3_PLAN 揭示 | **真实生产级 35%** |
| Z6G4 依赖 | — | ⏳ 待 W14 确认 | **GA 风险 at risk** |
| 13 critical-path items | — | ⏳ 11 follow-up issues open | **需 RC3+ 关闭** |

---

## 15. v3.9.0 治理收口 (RC4 + GA 目标)

### 15.1 GA 收口标准 (W22 末, 调整后)

- [ ] G1: 22/22 TPC-H 保持 PASS (REAL, not form-only)
- [ ] G2: INT-2 关闭 (REAL)
- [ ] G3: INT-3 关闭 (REAL)
- [ ] G4: ARCH-3 关闭 (REAL)
- [ ] G5: SEM-1 关闭 (REAL)
- [ ] G6: Backup/Restore 完整实现 + 100+ scenarios (REAL)
- [ ] G7: 24h REAL wall-clock Soak Test PASS
- [ ] G8: Crash Matrix 100+ REAL scenarios PASS
- [ ] G9: Upgrade Test PASS (v3.8 → v3.9 数据可读, REAL)
- [ ] G10: Audit Log + 时间旅行查询实现 (REAL)
- [ ] G11: QPS real run (60+ min, Z6G4)
- [ ] G12: Sysbench real run (sysbench binary installed)
- [ ] G13: 24h 真实稳定性 (REAL wall-clock)
- [ ] G14: 8 real crash categories (REAL)
- [ ] G15: 汇总报告 (real data, 0 TBD)
- [ ] G16: Compatibility v3.8 → v3.9 (full PASS)
- [ ] 168h real soak completed
- [ ] PERFORMANCE_BASELINE: 0 TBD
- [ ] All 11 follow-up issues closed

### 15.2 预期产物 (Phase 6 末 → GA cut)

- `docs/releases/v3.9.0/ga/GA_GATE_REPORT.md` (16 门禁全 REAL PASS 报告)
- `docs/releases/v3.9.0/ga/GA_RELEASE_NOTES.md` (GA 公告)
- `docs/releases/v3.9.0/ga/V390_GA_CLOSURE_TECHNICAL_DEBT_AND_ROADMAP.md` (债收口)
- `docs/releases/v3.9.0/beta/SOAK_72H_REPORT.md` (REAL 72h)
- `docs/releases/v3.9.0/beta/SOAK_168H_REPORT.md` (REAL 168h, GA-final)
- Tag: `v3.9.0` (GA final)
- GitHub/Gitea Release Notes

### 15.3 GA 评级 (v3.9.0 目标, 调整后)

| 评级 | v3.8.0 | v3.9.0 目标 | v3.9.0 RC2 末 (校准) |
|------|--------|-------------|----------------------|
| **GA (General Availability)** | ✅ PASS | ✅ TARGET | 🟡 form-only (35%) |
| **Production Ready** | ⚠️ 中小规模场景 | ✅ 中小规模 7×24h | ❌ real 24h+ 未跑 |
| **Production Grade** | ❌ NOT YET | ⚠️ TARGET (需 Soak 验证) | ❌ NOT YET |
| **Single-Node Production Candidate** | ❌ NOT YET | ✅ TARGET (v3.9.0 主题) | ⏳ form-only 完成, real pending |

---

## 16. 综合评分 (RC2 后期视角, 12 维度)

### 16.1 v3.9.0 RC2 末综合评分 (按真实生产级校准)

| 维度 | v3.8.0 评分 | **v3.9.0 RC2 形式** | v3.9.0 真实 (校准) | 目标 v3.9.0 GA | 关键依据 |
|------|------------|-------------------|-------------------|----------------|----------|
| **SQL Engine** | 9.0/10 | **9.0/10** | 9.0/10 | ≥ 9.0 | 22/22 TPC-H 继承 |
| **Parser** | 10/10 | **10/10** | 10/10 | ≥ 10 | 18/18 继承 |
| **Executor** | 8.5/10 | **8.5/10** | 8.5/10 | ≥ 8.5 | 22/22 in-process |
| **ACID / DML** | 8.5/10 | **9.0/10** | 8.5/10 (form-only) | ≥ 9.0 (G4 强化) | INT-1 + G4 form-only |
| **Testing** | 9/10 | **9.0/10** | 6.0/10 (form-only, 77 #[ignore], 43 TBD) | ≥ 9.5 (REAL) | ⚠️ RC3_PLAN 揭示 |
| **Governance** | 9.5/10 | **10/10** | 9.5/10 (form-only 区分) | ≥ 10 (G1-G16) | 10/10 rules + G1-G16 |
| **Documentation** | 9/10 | **9.5/10** | 9.5/10 | ≥ 9.5 (Phase 0 文档化) | 19+ v3.9.0 文档 |
| **Performance** | 6.5/10 | **7.0/10** | 6.0/10 (form-only perf, 43 TBD) | ≥ 7.0 (P3-4/5 优化) | bulk_insert 300000× faster |
| **Parallelism** | 3.5/10 | **6.0/10** | 4.0/10 (G2 form-only) | ≥ 6.0 (G2 集成) | INT-2 14/14 delegation |
| **MySQL Compatibility** | 58/100 (5.8/10) | **5.8/10** | 5.8/10 | 5.8/10 (frozen) | Frozen for v3.9.0 |
| **Architecture Completeness** | 7.5/10 | **9.0/10** | 7.0/10 (form-only 关闭 4 债) | ≥ 9.0 (G2/G3/G4/G5 REAL) | ⚠️ form vs real 区分 |
| **Production Readiness** (新维度) | ❌ NOT YET | **5.0/10** | 3.5/10 (form-only, real pending) | ≥ 8.0 (G6/G7/G8/G9 REAL) | ⚠️ Soak/Crash/Upgrade 未真实 |
| **GMP Audit Capability** (新维度) | 5.0/10 | **8.0/10** | 6.0/10 (G10 form-only) | ≥ 8.0 (G10 REAL) | Audit/Time Travel form-only |
| **综合** | **8.4~8.7/10** | **8.5/10 (form-only)** | **7.0/10 (真实校准)** | **≥ 8.7/10 (REAL)** | ⚠️ form-only 校准后 |

### 16.2 v3.9.0 评分标准

- **10/10**: Production Grade (与 MySQL/PostgreSQL 同等)
- **9/10**: Single-Node Production Candidate (v3.9.0 目标)
- **8/10**: Production Ready (中小规模 7×24h)
- **7/10**: GA (开发/测试/中小规模)
- **6/10**: Beta (核心功能完整)
- **5/10**: Alpha (核心 SQL + ACID)
- **3-4/10**: Capability Exists (未集成)
- **0-2/10**: 雏形/缺失

### 16.3 v3.9.0 综合评级 (RC2 末校准)

**RC2 形式评级**: **8.5/10 — Form-Only Validation Milestone** ✅
- 10/10 G1-G10 form-only PASS
- G11-G15 infra ready
- 16/16 子任务完成

**RC2 真实评级**: **7.0/10 — Beta+ (真实生产级 35%)** 🟡
- 77 #[ignore] tests
- 43 TBD perf placeholders
- 0/22 wire TPC-H
- 0/6 G1 TPC-H 实际跑
- Soak tests: 模拟 CPU 循环

**Phase 6 末目标 (GA)**: **≥ 8.7/10 — Single-Node Production Candidate**

**与成熟数据库等级对比**:
- CockroachDB / TiDB / PostgreSQL / MySQL: 9.5-10/10 (Production Grade)
- v3.9.0 GA 目标: ≥ 8.7/10 (Single-Node Production Candidate, REAL)
- v3.8.0 当前: 8.4~8.7/10 (实用型数据库引擎)
- v3.9.0 RC2 末 (形式): 8.5/10 (form-only, 校准)
- v3.9.0 RC2 末 (真实): 7.0/10 (35% 真实生产级, 校准)

---

## 17. 下一步 (v3.9.0+)

### 17.1 RC3 启动 (W13-14, 2026-06-20 估计)

| 任务 | 工作量 | 优先级 | Issue |
|------|--------|--------|-------|
| Phase 0 Gate 验证 (D1-D5 + D6 + D9) | 2h | 高 | — |
| Gitea milestone 公告 + 通知 | 1h | 高 | — |
| **#3221 L3 acceptance + unignore 15 e2e + 1 L3** | 8h | P0 | #3221 |
| **#3222 Server `LOAD DATA` perf** | 16h | P0 | #3222 |
| **#3223 Storage tx tracking + unignore 9 TX/WAL** | 24h | P0 | #3223 |
| **#3227 Replace corrupt SF=0.01 fixture** | 8h | P0 | #3227 |
| **#3230 Unignore 8 wire_smoke_sf (value-correctness)** | 12h | P0 | #3230 |
| G1 TPC-H 22/22 real run re-engineering | 16h | P0 | (RC3_PLAN) |
| G8 Crash Matrix real scenarios | 16h | P0 | (RC3_PLAN) |
| G10 真实审计验证 | 8h | P0 | (RC3_PLAN) |
| Phase Gate 验证 (D1-D5 + D6 + D7+D8+D9 + G3 G4 real) | 5h | 高 | — |

**RC3 总工时: ~116h (2-3 周)**

### 17.2 RC4 启动 (W15-18, 2026-07-15 估计, Z6G4 依赖)

| 任务 | 工作量 | 优先级 | Issue |
|------|--------|--------|-------|
| **#3224 Z6G4 perf baseline + fill PERFORMANCE_BASELINE** | 24h | P1 | #3224 |
| **#3225 Real 24h/72h wall-clock soak** | 72h | P1 | #3225 |
| **#3228 Unignore 10 long_run_stability** | 16h | P1 | #3228 |
| **#3231 TPC-H 22/22 SHA-256 baseline** | 8h | P1 | #3231 |
| G11 QPS real run (60+ min) | 16h | P1 | (infra) |
| G12 Sysbench install + run | 16h | P1 | (infra) |
| G13 24h 真实稳定性 | 24h | P1 | (infra) |
| G14 8 real crash categories | 32h | P1 | (infra) |
| G15 汇总报告 (real data, 0 TBD) | 8h | P1 | (perf) |
| G16 Compatibility full PASS | 8h | P1 | (compat) |
| **#3226 TPC-H Q8/Q9 真 bug fix** | 16h | P2 | #3226 |
| Phase Gate 验证 (D1-D5 + D6 + D7+D8+D9 + ALL G real) | 5h | 高 | — |

**RC4 总工时: ~245h (3-4 周 on Z6G4)**

### 17.3 GA 收口 (W19-22, 2026-09-23 估计, at risk)

| 任务 | 工作量 | 优先级 | Issue |
|------|--------|--------|-------|
| **#3229 Real 168h wall-clock soak (7 days)** | 168h | P1 | #3229 |
| 168h soak continue + 监控 | 168h | P1 | (continuation) |
| 性能优化收口 | 16h | P3 | P3-1/2/3/4/5 |
| Doc finalization (RELEASE_NOTES.md, GA_GATE_REPORT.md) | 16h | 高 | — |
| 切 `v3.9.0-ga` tag | 0.5h | 高 | — |
| `release/v3.9.0` branch cut (maintenance) | 0.5h | 高 | — |
| Phase Gate 验证 (D1-D5 + D6 + D7+D8+D9 + ALL G REAL) | 5h | 高 | — |

**GA 总工时: ~374h (2-3 周 + 7 天 real soak)**

### 17.4 v3.10+ 规划 (远期)

- Window Function / CTE (推 v3.10+)
- Distributed Database (推 v3.10+)
- Replication (推 v3.10+)
- F-30 SEQUENCE / F-36 列级权限 / F-03 GIS (推 v3.10+)
- SIMD 集成 (推 v3.10+)
- MySQL 5.7 新函数 (NATURAL/FULL OUTER/GROUP_CONCAT/DATE_SUB) — 推 v3.10+
- Optimizer 重大增强 (Cost Model) — 推 v3.10+

---

## 18. 结论 (v1.0 RC2 Post-Audit)

### 18.1 RC2 后期综合评估

**v3.9.0 = Production Readiness Release (工程化版本) — RC2 ✅ form-only → RC3 ⏳ P0 cut 待启动**:

```text
✅ 分支 develop/v3.9.0 创建 (从 main@v3.8.0 fork, HEAD 0852e42e)
✅ 5 计划文档就绪 (V390_VERSION_PLAN + V390_DEVELOPMENT_PLAN + V390_TEST_PLAN + V390_TEST_PLAN_ROUND2_REVIEW + V390_TEST_PLAN_SUPPLEMENT_PERF)
✅ 阶段历程: Alpha1 ✅ + Beta ✅ + RC1 ✅ + RC2 ✅ (form-only)
✅ 16/16 子任务完成 (P0-P3)
✅ G1-G10 10/10 form-only PASS (G10 non-blocking warn)
✅ G11-G15 infrastructure ready
✅ G16 5/7 PASS
✅ TPC-H 22/22 in-process (PR #3213 Q11/Q14/Q22)
✅ INT-3 14/14 delegation (PR #3200)
✅ G1 TPC-H 22/22 baseline hash + CI gate (PR #3186)
✅ bulk_insert 优化 300000× faster (PR #3233)
✅ TX/WAL active_txs tracking Phase 1-4 (#3223, #3240, #3243, #3244)
✅ TPC-H Q4/Q15/Q22 fix (PR #3250, #3241, #3215)
✅ clippy G2 lib gate (PR #3254)
✅ 7 perf 报告 (1018 lines)
✅ 72h Soak compressed-time 10/10 PASS
⏳ 11 follow-up issues open (#3221-#3231)
⏳ 77 #[ignore] tests 待 unignore
⏳ 43 TBD perf placeholders 待 fill
⏳ 0/22 wire TPC-H 待实现
⏳ Z6G4 硬件 W14 之前可用性待确认
⏳ GA 目标 2026-09-23 (at risk)
```

### 18.2 核心战略 (ChatGPT 2026-06-05 评审采纳, RC3_PLAN 校准)

> **v3.9.0 不是"功能版本", 是"工程化版本". 核心问题是"数据库死了以后还能不能回来".**
> **校准 (RC3_PLAN 2026-06-06 揭示)**: form-only 验证 ≠ production-ready.
> **真实生产级 35%**, 需 RC3+RC4+GA 真实运行才能达到 Single-Node Production Candidate.

- 资源按 ChatGPT 建议重新分配 (40% 架构 / 35% 可靠性 / 15% GMP / 10% 性能 / 0% 新 SQL)
- 16 任务 / 451h / 12 周 / 6 Phase (原计划)
- 调整后: 12 周 + RC3 (2-3 周) + RC4 (3-4 周) + GA (2-3 周) = **22-26 周总周期**
- G1-G16 门禁 (16 维, 取代 L1-L6 8 维)
- Single-Node Production Candidate 主题 (form-only 完成, REAL pending)
- 真实生产级 35% (RC3_PLAN 揭示)
- 13 critical-path items (11 follow-up issues) 待关闭
- GA 2026-09-23 at risk (Z6G4 依赖)

### 18.3 关键风险与缓解 (更新)

**🔴 已发生 (RC3_PLAN 揭示)**:
- Form-only 验证不充分 → RC3 计划调整, G1/G7/G8/G10 真实化
- 77 #[ignore] + 43 TBD + 0/22 wire TPC-H → RC3+ 关闭

**🟠 高风险 (待解决)**:
- Z6G4 硬件 W14 之前不可用 → GA 延 1-2 周
- #3223 TX/WAL autocommit conflict → RC3 fix
- 168h real soak (7 days) → GA-final
- 13 critical-path items → RC3/RC4/GA 关闭

**🟡 中风险**:
- 性能 baseline 待 Z6G4 真实 run
- L3 acceptance 待 #3221
- Bulk_insert 已优化 (5min → 1ms)

### 18.4 关联文档 (全部 v3.9.0/* 真实存在)

- `docs/releases/v3.9.0/README.md` (v3.9.0 索引)
- `docs/releases/v3.9.0/ROADMAP.md` (6 Phase 路线图)
- `docs/releases/v3.9.0/CHANGELOG.md` (变更日志)
- `docs/releases/v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md` (本文件, v1.0 RC2 Post-Audit)
- `docs/releases/v3.9.0/plans/V390_VERSION_PLAN.md` (战略定位)
- `docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md` (P0/P1/P2/P3 任务)
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md` (G1-G10 门禁)
- `docs/releases/v3.9.0/beta/BETA_RELEASE_NOTES.md` (Beta 阶段)
- `docs/releases/v3.9.0/beta/SOAK_72H_REPORT.md` (72h Soak)
- `docs/releases/v3.9.0/rc/RC1_GATE_REPORT.md` (RC1 门禁)
- `docs/releases/v3.9.0/rc/RC1_RELEASE_NOTES.md` (RC1 公告)
- `docs/releases/v3.9.0/rc/RC2_GATE_REPORT.md` (RC2 门禁)
- `docs/releases/v3.9.0/rc/RC2_RELEASE_NOTES.md` (RC2 公告)
- `docs/releases/v3.9.0/rc/RC3_PLAN.md` (调整后 RC3/RC4/GA 计划, 关键)
- `docs/releases/v3.9.0/perf/*` (7 perf 报告)
- **`docs/audit/status/2026-06-07-v390-comprehensive-assessment.md`** (claude-macmini, PR #3255, 互补 gate-by-gate 评估, 35%→60% test authenticity)
- **`docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md`** (李哥 信任 concern, 关键发现)
- `docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md` (v3.8.0 评估基线)

### 18.5 维护信息

| 项目 | 值 |
|------|-----|
| 报告版本 | v3.9.0-ASSESSMENT-v1.0 |
| 创建日期 | 2026-06-07 |
| 维护人 | Hermes Agent |
| 阶段状态 | RC2 后期, 准备 RC3 |
| 下次审查 | RC3 cut 时 (W14 末) |
| 评估方法 | 仿照 V380 v3.2 + RC3_PLAN 真实校准 + RC2_GATE_REPORT 整合 |

---

**v3.9.0 RC2 Post-Audit Final: 一个明确战略反转 (Production Readiness Release) + 16 任务完成 + 5 阶段 (Alpha1/Beta/RC1/RC2) 全部 form-only 验证 + 真实生产级 35% (RC3_PLAN 揭示) + 11 follow-up issues 待关闭 + 13 critical-path items 待 RC3/RC4/GA 真实运行的工程化版本, 目标 22-26 周后达到 Single-Node Production Candidate 等级 (≥ 8.7/10 REAL), 适合中小规模 7×24h 生产场景, 但与 CockroachDB/TiDB/PostgreSQL/MySQL 等成熟分布式/单节点产品仍不在同一成熟度等级, 需要 v3.9.0 GA 真实运行验证 + v3.10+ 持续演进到 Production Grade.**

Reference: v3.8.0 v3.2 GA Decision Document (`docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md` 794 行)
Reference: v3.9.0 RC3_PLAN.md (Adjusted Release Plan, 关键发现)
Reference: v3.9.0 RC2_GATE_REPORT.md (G1-G16 形式门禁状态)
Reference: v3.9.0 BETA_RELEASE_NOTES.md (16/16 子任务 + 72h Soak)
Reference: claude-macmini audit (PR #3255, 互补 gate-by-gate 评估)
