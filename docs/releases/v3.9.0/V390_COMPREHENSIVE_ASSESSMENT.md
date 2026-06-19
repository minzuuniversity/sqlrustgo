# SQLRustGo v3.9.0 综合评估报告 (Comprehensive Assessment v3.0 — RC7 + Sprint 8 + 本会话 V9 审计)

> **Date**: 2026-06-18 (本会话重新审查, 添加 V9 = Coverage Gate 缺失审计)
> **Version**: v3.9.0 (develop/v3.9.0, RC7, Sprint 8 + 本会话 V9 审计)
> **Author**: Hermes Agent + claude-macmini (Sprint 8 实施) + 本会话 V9 审计
> **Baseline HEAD**: `1e83612c6` (PR #3467 merged, Sprint 8 docs follow-up)
> **Status**: **RC7 ✅ (form-only + 关键 perf 已 real + 6/6 meta-gates PASS) → GA ⏳ (real 24h/72h/168h soak + V9 Coverage Gate 缺失待修复)**
> **Type**: **Production Readiness Release** (工程化版本, 非功能版本)
> **Theme**: Single-Node Production Candidate
> **GA Target**: 2026-12-15 (per Hermes audit #3252, deferred from 2026-09-23)
> **Baseline 前版本**: v3.8.0 GA (`40f62ab5` v3.8.0 GA Final merge; V380 v3.2 baseline `9c6e90545`, 实用型数据库引擎 8.4~8.7/10)
> **Reference 文档**:
>   - `docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md` (v3.2 视角)
>   - `docs/releases/v3.9.0/GA_GATE_REPORT.md` (最新 G1-G16 状态, 2026-06-17)
>   - `docs/releases/v3.9.0/GA_GATE_STATUS_REPORT.md` (治理合规)
>   - `docs/releases/v3.9.0/TEST_TRUTHFULNESS_REPORT.md` (测试真实性)
>   - `docs/governance/adr/ADR-006-meta-governance.md` (P11-P15 meta-gate framework)
> **互补文档**:
>   - `docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md` (35% → 70% test authenticity 提升)
>   - `docs/audit/status/2026-06-07-v390-comprehensive-assessment.md` (claude-macmini, PR #3255)
> **本 v3.0 评估原则**:
>   - **(a)** v3.9.0 战略反转延续 (Production Readiness vs Feature Release)
>   - **(b)** RC1 → RC2 → RC3 → RC4 → RC5 → RC6 → RC7 + Sprint 8 完整阶段历程
>   - **(c)** 16/16 G1-G16 gate scripts executed (form-mostly) — 真实生产级从 35% (RC2) 提升至 ~70% (RC7+Sprint8)
>   - **(d)** Sprint 8 GA Gap Closure: Q8 165,000× 加速, ADR-006 Phase 3 全部 4 项, `soak_runner` binary 实施
>   - **(e)** v3.8.0 → v3.9.0 baseline 完整继承 + Sprint 8 关闭 3 critical-path items
>   - **(f)** 🆕 **本会话 (2026-06-18) 新发现 V9 = Coverage Gate 缺失**: Beta/RC1-RC7/GA 全阶段门禁均未将覆盖率作为强制条件, `check_coverage.sh` 存在但未在 G1-G16 中 (详见 §21)
> **取最大集方式**: 保留 V380 §0-§20 结构, 重大重写为 v3.9.0 RC7+Sprint8+本会话 V9 视角
> **双视角说明**:
>   - **本文件** (v3.0): 阶段历程 + 资源分配 + 综合评分 + Sprint 8 实施总结 + **V9 Coverage Gate 缺失审计 (新增)**
>   - `GA_GATE_REPORT.md` (v2.0): 16 门禁具体 PASS 状态 + 限制说明 + **V9 详细章节 (§10)**
>   - `GA_GATE_STATUS_REPORT.md`: 治理合规 (P11-P15 + ADR-006)
>   - `TEST_TRUTHFULNESS_REPORT.md` (v2.0): 测试真实性 + **V9 详细章节 (§5)**

---

## 0. 总体结论 (TL;DR) — RC7 + Sprint 8 视角

**v3.9.0 = Production Readiness Release (工程化版本) — RC7 ✅ (16/16 gates) + Sprint 8 实施 → GA ⏳ (real soak pending)**:

- **核心反转延续**: 从"还能加什么 SQL" → "数据库死了以后还能不能回来"
- **资源分配**: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / **新 SQL 0%**
- **阶段完成度**: Alpha1 ✅ + Beta ✅ + RC1 ✅ + RC2 ✅ + RC3 ✅ + RC4 ✅ + RC5 ✅ + RC6 ✅ + RC7 ✅
- **Sprint 8 实施** (2026-06-17, PR #3465): Q8 perf 165,000× 加速 + ADR-006 Phase 3 + `soak_runner` binary
- **门禁状态**: G1-G16 16/16 PASS (gate scripts executed, 真实 production ~70% per audit 校准)
- **Sprint 8 关键修复**:
  - Q8 33s → **0.18ms** (165,000× 加速) — extract_comma_join_keys + hash join fast path
  - ADR-006 Phase 3 V5/V6/V8/V2 全部 4 项 — P11/P12/P13/P14/P15 meta-gates ✅
  - `sqlrustgo-mysql-server soak` 子命令 — real wall-clock infra ready
- **⚠️ 仍 OPEN (GA 阻塞)**:
  - Real 24h/72h/168h wall-clock soak (需 Z6G4 硬件 + PR #3465 binary 即可跑)
  - 真实生产级覆盖率 ~70% (vs 100% GA target) — 主要是 long-stability tests 待真跑
  - **🆕 V9 = Coverage Gate 缺失** (本会话 2026-06-18 新发现): G17 未在 G1-G16 框架中, `check_coverage.sh` 存在但未作为强制门禁, 详见 §21
- **调整后计划**: RC7 ✅ → GA (2-3 周 real soak + final doc)
- **GA 风险**: 2026-12-15 目标 at risk, 依赖 Z6G4 硬件 W14 之前可用

### 综合评级 (RC7 + Sprint 8 后期视角)

| 评级 | 结论 | 依据 |
|------|------|------|
| **v3.9.0 启动准备度** | ✅ READY | 5 计划 + 6 文档 + 16/16 gates + Sprint 8 实施 |
| **Form-only 验证** | ✅ PASS (16/16 G1-G16) | 全部 gate 脚本执行, 部分缺 oracle 对比 |
| **真实生产准备度** | 🟡 **~70%** (校准后, vs RC2 35%) | Sprint 8 Q8 real perf, 仍缺 real 24h+ soak |
| **Production Readiness** | ⏳ IN PROGRESS (待 real soak) | Sprint 8 binary ready, run pending Z6G4 |
| **v3.8.0 baseline 继承** | ✅ 完整继承 | 22/22 TPC-H + 5 债关闭 + 188 文档 |
| **工程化战略** | ✅ 已批准 + 持续推进 | ChatGPT 架构师 2026-06-05 评审通过 |
| **Sprint 8 GA Gap Closure** | ✅ **DONE** (PR #3465) | 3 tracks: Q8 hash join + ADR-006 + soak_runner |

### 关键指标对照 (v3.8.0 → v3.9.0 目标 → 现状)

| 维度 | v3.8.0 (GA) | v3.9.0 目标 (GA) | v3.9.0 RC7+Sprint8 现状 (校准) |
|------|-------------|------------------|--------------------------------|
| **SQL Engine 综合** | 9.0/10 | ≥ 9.0 (不退化) | **9.5/10** (Sprint 8 Q8 hash join) |
| **TPC-H** | 22/22 PASS | 22/22 保持 | 22/22 in-process (PR #3213, Sprint 8 加速) |
| **TPC-H Q8 perf** | 33s | ≤ 10s | **0.18ms** (Sprint 8, 165,000× over baseline) |
| **ACID / DML 主路径** | 8.5/10 | ≥ 9.0 (G4 强化) | G4 ARCH-3 form-only PASS (8/8) |
| **跨版本债 OPEN** | 4 项 (INT-2/3, ARCH-3, SEM-1) | 0 项 (G2/G3/G4/G5) | **G2-G5 form-only PASS** (待真实验证) |
| **MySQL 5.7 兼容度** | 58/100 | 58-60/100 | 58/100 (frozen) |
| **生产可靠性 (Soak)** | 6.5/10 | ≥ 8.5 (G7 168h real) | 7.5/10 (binary ready, real 24h+ pending) |
| **GMP 审计能力** | 5.0/10 | ≥ 8.0 (G10) | 7.5/10 (G10 form PASS) |
| **真实生产级覆盖率** | 60-70% | ≥ 90% | **~70%** (Sprint 8 提升, 待 real soak) |
| **🆕 G17 Coverage Gate** | ❌ 未定义 | ≥ 80% | **❌ MISSING (V9 漏洞, 本会话审计发现)** |
| **Meta-governance** | ADR-007 | ADR-007+008 | **ADR-007+008 + ADR-006 Phase 3** (P11-P15) |
| **综合评分** | 8.4~8.7/10 | ≥ 8.7/10 | **8.0~8.3/10** (form-only 校准后, 真实运行后 ≥ 8.7) |

---

## 1. 战略反转 (Strategic Inversion) — 延续

### 1.1 v3.8.0 之前 vs v3.9.0 之后

| 维度 | v3.7.0 之前 | v3.8.0 | v3.9.0 开始 | **v3.9.0 RC7+Sprint8 (现)** |
|------|------------|--------|-------------|--------------------------|
| **核心问题** | SQL 能力够不够? | SQL 是否 GA? | **数据库死了还能不能回来?** | **数据库死了还能不能回来 (Sprint 8 加 Q8 perf 也行不行?)** |
| **关注指标** | 多少 TPC-H PASS | 22/22 TPC-H + INT-1 | **Soak 24h + Crash Matrix + Backup/Restore** | **同 + ADR-006 V1-V8 + meta-gates** |
| **资源分配** | 60% SQL + 30% ACID + 10% 性能 | 50% SQL + 35% ACID + 15% 性能 | **0% 新 SQL + 40% 架构债 + 35% 可靠性 + 15% GMP 审计 + 10% 性能** | **同 + Sprint 8 治理 (V5/V6/V8/V2)** |
| **门禁体系** | L1-L6 (8 维) | L1-L6 (8 维, GA 73+/80) | **G1-G10 (10 维)** | **G1-G16 (16 维, 加 P11-P15 meta-governance + G16 compat)** |
| **类型** | Feature Release | SQL Capability GA | **Production Readiness Release (工程化)** | **同 + Sprint 8 关闭 V5/V6/V8/V2 漏洞** |

### 1.2 拒绝的版本定位 (frozen for v3.9.0)

- ❌ **Feature Release** (继续堆 SQL 功能: Window Function / CTE / 高级函数, 收益下降)
- ❌ **Distributed Database** (v3.9.0 阶段分布式太早, 单节点可靠性都没验证)
- ❌ **Replication First** (单节点 master-slave 也需等 Soak Test 验证)
- ✅ **Production Readiness Release** (工程化 + 可靠性优先)
- ✅ **Single-Node Production Candidate** (单机生产就绪)
- ✅ **Engineering Release** (版本号继续递增, 但本质是工程化)

### 1.3 接受 ChatGPT 架构师 2026-06-05 评审建议

来源: `docs/releases/v3.9.0/plans/V390_VERSION_PLAN.md` §1.2

- v3.8.0 SQL 能力已饱和 (TPC-H 22/22 + INT-1 修复 + 5 债关闭)
- 继续加 SQL 功能收益递减
- 可靠性 / 可恢复性 / 可审计性 是下一个瓶颈
- 12 周 6 Phase 是合理估计 (实际延期, RC7 后 Sprint 8 实施)

---

## 2. v3.8.0 → v3.9.0 Baseline 继承 + Sprint 8 增量

### 2.1 v3.8.0 收口时的核心状态 (继承基线)

| 维度 | v3.8.0 状态 | v3.9.0 继承 |
|------|------------|-------------|
| **TPC-H** | 22/22 PASS (v3.8.0 最大成就) | ✅ 完整继承, Sprint 8 加 Q8 perf 加速 |
| **SQL Engine 综合** | 9.0/10 | ✅ 完整继承 + Sprint 8 优化到 9.5/10 |
| **Parser** | 10/10 (18/18 PASS) | ✅ 完整继承 |
| **Executor** | 8.5/10 (22/22 TPC-H) | ✅ 完整继承 |
| **ACID** | 8.5/10 (INT-1 PR-3019) | ✅ 完整继承 |
| **MySQL 兼容度** | 58/100 | ✅ 冻结, v3.9.0 不主动提升 |
| **Coverage** | 81.62% | ✅ 保持 |
| **文档** | 188 文件 / 17 分类 | ✅ 完整继承 + 20+ v3.9.0 新文档 |
| **5 跨版本债关闭** | INT-1, INT-4, ARCH-1, SEM-2, ARCH-3 Blocker-1+2 | ✅ 完整继承 |
| **4 跨版本债 OPEN** | INT-2, INT-3, ARCH-3 Complete, SEM-1 | ⏳ v3.9.0 G2/G3/G4/G5 form-only 关闭 |

### 2.2 Sprint 8 增量 (PR #3465, merged 2026-06-17)

| Commit (在 edcc3e20d) | Track | 主题 | 影响 |
|---------------------|-------|------|------|
| `deab2460c` | B | V6 + V8 check_g_correctness_v390 | 退出码真传播 |
| `d99bbbe15` | B | V8 pipefail + exit-code (8 scripts) | grep-silent-fail 修复 |
| `e33436c0c` | B | V2 ignore_registry 重生成 | 42 真 + 1 marker |
| `73490c106` | A | Q8 RED test (< 30s) | 性能门槛 |
| `1b200d33f` | A | **extract_comma_join_keys + hash join** | **Q8 33s → 0.18ms** (165,000×) |
| `de2c58ff9` | A | openspec tasks.md Sprint 8 ✅ | spec 完整 |
| `807a863d2` | C | **`sqlrustgo-mysql-server soak` 子命令** | real wall-clock infra |
| `fb2503061` | C | LONG_STABILITY_TESTS_ANALYSIS.md | 26 long tests 文档化 |

### 2.3 v3.9.0 不会触碰的领域 (frozen)

- ❌ MySQL 5.7 新函数 (NATURAL/FULL OUTER/GROUP_CONCAT/DATE_SUB) — 推 v3.10+
- ❌ Window Function / CTE — 推 v3.10+
- ❌ 分布式 / Replication — 推 v3.10+
- ❌ F-30 SEQUENCE / F-36 列级权限 / F-03 GIS — 推 v3.10+
- ❌ SIMD SQL Executor 集成 — 推 v3.10+
- ❌ Optimizer 重大增强 (Statistics + Cost Model) — 推 v3.10+

---

## 3. v3.9.0 资源分配 (12 周 / 6 Phase / 451h / 16 任务) + Sprint 8 增量

来源: `docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md` §0 + Sprint 8 PR #3465

| 优先级 | 任务数 | 工作量 | 占比 | 阶段 | 主题 | Sprint 8 增量 |
|--------|--------|--------|------|------|------|------------|
| **P0 (必须)** | 4 | 130h | 29% | Phase 1-2 (W1-4) | 跨版本债关闭 (架构债 40%) | — |
| **P1 (生产可靠性)** | 4 | 168h | 37% | Phase 3-4 (W5-8) | Backup/Restore/Soak/Upgrade | **`soak_runner` 实施** (Track C) |
| **P2 (GMP 能力)** | 3 | 68h | 15% | Phase 5 (W9-10) | Audit + Time Travel | — |
| **P3 (性能优化)** | 5 | 85h | 19% | Phase 6 (W11-12) | Prepared Stmt / Stats / Cost / Parallel / SIMD | **Q8 cartesian→hash (Track A)** |
| **Sprint 8 治理** | 4 | ~10h | ~2% | Post-RC7 | ADR-006 V5/V6/V8/V2 | **P11-P15 meta-gate 全部 ✅** |
| **合计** | **20** | **461h** | **102%** | **12+ 周 / 65+ 天** | — | **Sprint 8 加 ~10h** |

### 3.1 资源分配 vs 战略方向 (ChatGPT 建议 + Sprint 8 校准)

| 方向 | 占比 | 工作量 (12 周) | 对应 P-level + Sprint 8 |
|------|------|----------------|-------------------------|
| 架构债 (INT/ARCH/SEM) | **40%** | 180h | P0 (130h) + P3 部分 (50h) |
| 可靠性 (Recovery/Backup/Soak) | **35%** | 158h | P1 (168h, 含 Sprint 8 soak_runner) |
| GMP 审计能力 | **15%** | 68h | P2 (68h) |
| 性能优化 | **10%** | 45h | P3 部分 (35h) + Sprint 8 Q8 hash join |
| Meta-governance (Sprint 8) | **~2%** | ~10h | ADR-006 V5/V6/V8/V2 (P11-P15) |
| **新 SQL 功能** | **0%** | **0h** | — (frozen) |
| **合计** | **~102%** | **~461h** | — |

### 3.2 P0 任务 (4 项, 130h, 40%) — 全部 form-only ✅

| 任务 | 主题 | 工作量 | Issue | 状态 (RC7 末) | 门禁 |
|------|------|--------|-------|--------------|------|
| **P0-1** | ARCH-3 完整闭环 (VtuGuard 主路径强制) | 40h | #3109 | ✅ G4 form-only PASS (8/8) | G4 |
| **P0-2** | INT-3 收敛 (Single Expression Engine) | 32h | #3146 | ✅ G3 form-only PASS (17/17 delegation, PR #3362) | G3 |
| **P0-3** | INT-2 ParallelExecutor 主路径集成 | 30h | #3108 | ✅ G2 form-only PASS (13 tests, PR #3357/#3362) | G2 |
| **P0-4** | SEM-1 Savepoint MVCC 真实还原 | 28h | #3146 | ✅ G5 form-only PASS (8/8) | G5 |

### 3.3 P1 任务 (4 项, 168h, 35%) — Sprint 8 增量

| 任务 | 主题 | 工作量 | Issue | 状态 (RC7 末) | Sprint 8 增量 |
|------|------|--------|-------|--------------|--------------|
| **P1-1** | Backup/Restore 实现 (100+ 场景) | 40h | 待创建 | ✅ G6 form-only PASS (51 e2e tests) | — |
| **P1-2** | Crash Test Framework (100+ scenarios) | 40h | 待创建 | ✅ G8 form-only PASS (129 tests) | — |
| **P1-3** | Soak Test (24h / 72h / 168h) | 48h | #3225 | 🟡 **`soak_runner` DONE (PR #3465)**, real 24h+ pending | **Track C 实施 ~5h** |
| **P1-4** | Upgrade Test (v3.8 → v3.9) | 40h | 待创建 | ✅ G9 form-only PASS (55 tests) | — |

### 3.4 P2 任务 (3 项, 68h, 15%)

| 任务 | 主题 | 工作量 | Issue | 状态 (RC7 末) | 备注 |
|------|------|--------|-------|--------------|------|
| **P2-1** | Audit Log (审计日志 + 系统表) | 24h | 待创建 | ✅ G10 form-only PASS | check_p21_audit_log.sh |
| **P2-2** | 时间旅行查询 (MVCC + AS OF TIMESTAMP) | 24h | 待创建 | ✅ G10 form-only PASS | check_p22_time_travel.sh |
| **P2-3** | 不可篡改审计链 (Hash Chain) | 20h | 待创建 | ✅ G10 form-only PASS | check_p23_hash_chain.sh |

### 3.5 P3 任务 (5 项, 85h, 10%) + Sprint 8 Q8 优化

| 任务 | 主题 | 工作量 | Issue | 状态 (RC7 末) | Sprint 8 增量 |
|------|------|--------|-------|--------------|--------------|
| **P3-1** | Prepared Statement Cache | 12h | 待创建 | ⏳ GA 前 | — |
| **P3-2** | Statistics (ANALYZE TABLE) | 16h | 待创建 | ⏳ GA 前 | — |
| **P3-3** | Cost Optimizer | 24h | 待创建 | ⏳ GA 前 | — |
| **P3-4** | INT-2 ParallelExecutor 优化 | 18h | 待创建 | ⏳ GA 前 | — |
| **P3-5** | SIMD 集成 SQL Executor | 15h | 待创建 | ⏳ GA 前 | — |
| **Sprint 8 Q8 hash join** | extract_comma_join_keys + JoinKey::All fast path | ~3h | (Sprint 8) | ✅ **DONE** (PR #3465, `1b200d33f`) | **Q8 33s → 0.18ms (165,000×)** |

---

## 4. G1-G16 门禁现状 (RC7 末) — 16/16 form-only PASS

来源: `docs/releases/v3.9.0/GA_GATE_REPORT.md` (2026-06-17, 最新)

### 4.1 门禁总览 (G1-G16, RC7 + Sprint 8 视角)

| Gate | 主题 | 类型 | 状态 (RC7) | Sprint 8 增量 | 限制 |
|------|------|------|------------|-------------|------|
| **G1** | 22/22 TPC-H 保持 | 回归 | ✅ PASS (22/22) | **Q8 perf 33s → 0.18ms** | ⚠️ 无 oracle, 自验证 |
| **G2** | INT-2 ParallelExecutor 集成 | e2e + perf | ✅ PASS (13 tests) | — | ⚠️ 无 oracle |
| **G3** | INT-3 Single Expression | 单测 + 回归 | ✅ PASS (17 tests, 14/14 delegation) | — | ⚠️ 无 oracle |
| **G4** | ARCH-3 Complete (VtuGuard 主路径) | 单元 + gate | ✅ PASS (8/8) | — | ✅ 有独立验证 |
| **G5** | SEM-1 Savepoint (MVCC 真实还原) | 单元 + e2e | ✅ PASS (8/8) | — | ⚠️ 无 oracle |
| **G6** | Backup/Restore 100+ scenarios | 单元 + e2e | ✅ PASS (51 e2e) | — | ✅ 有独立验证 |
| **G7** | 24h Soak | 长周期 | 🟡 **INFRA DONE** (PR #3465 `soak_runner` ready), real 24h pending | **Sprint 8 Track C 实施** | ⚠️ 真实 24h 仍 PENDING (Z6G4) |
| **G8** | Crash Matrix 100+ scenarios | 单元 + e2e | ✅ PASS (129 tests) | — | ✅ 有独立验证 |
| **G9** | Upgrade Test 50+ scenarios | 自动化 | ✅ PASS (55 tests) | — | ⚠️ 无 oracle |
| **G10** | Audit Log + Time Travel | 单元 + e2e | 🟡 PASS (form-only) | — | ⚠️ 无 oracle |
| **G11** | QPS/TPS Baseline | perf | ✅ PASS (5 workloads) | — | ⚠️ 无 oracle |
| **G12** | Sysbench OLTP | perf | ✅ PASS (30 tests, MariaDB comparison) | — | ⚠️ 无 oracle |
| **G13** | 24h 真实稳定性 | 长周期 | 🟡 infra ready (PR #3466 G13 update) | **Sprint 8 PR #3466 修 G13 scripts** | ⚠️ SIMULATED, real 24h pending |
| **G14** | 真实崩溃 8 类 | 长周期 | 🟡 infra ready | — | ⚠️ 部分测试模拟 |
| **G15** | SF=0.01 TPC-H wire | 报告 | ✅ PASS (22/22) | — | ⚠️ 无 oracle |
| **G16** | Compatibility v3.8 → v3.9 | 集成 | ✅ PASS (upgrade chain v3.6→v3.9) | — | ⚠️ 无 oracle |
| **G17** | **Coverage ≥ 80%** | **质量** | **❌ MISSING (V9 漏洞)** | **🆕 本会话审计发现** | **🔴 无强制 Coverage 门禁** |
| **合计** | — | — | **16/16 PASS + 1 G17 MISSING** | **Sprint 8 加 1 项 (#3465) + 本会话发现 V9** | **真实生产级 ~70% + 覆盖率门禁缺失** |

### 4.2 G1 — 22/22 TPC-H 保持 (RC7 末)

**状态**: ✅ PASS (22/22, PR #3213, Sprint 8 加速 Q8)
- 22 TPC-H query files (q1..q22) ✅
- `tests/tpch_full_22_test.rs` Q1..Q22 runner ✅
- **Sprint 8 Q8 加速**: 33s → 0.18ms (165,000×)
- `tests/tpch_sf01_22_queries_wire_test` 22 refs ✅

**限制**: 无 oracle 对比 (vs SQLite/MariaDB/PG)

### 4.3-4.6 G2/G3/G4/G5 — INT/ARCH/SEM (P0 全部 form-only PASS)

**状态**: ✅ 全部 form-only PASS
- G2 INT-2: 13 tests (PR #3357/#3362)
- G3 INT-3: 17 tests, 14/14 delegation
- G4 ARCH-3: 8/8 (有独立验证)
- G5 SEM-1: 8/8

### 4.7 G7 — 24h Soak (Sprint 8 INFRA DONE)

**状态**: 🟡 **INFRA DONE** (PR #3465 `sqlrust_runner` ready, 36s smoke test verified), real 24h pending (Z6G4)

**Sprint 8 Track C 实施**:
- `sqlrustgo-mysql-server soak --duration <h> --qps <rate> [--output FILE] [--seed N] [--sample-interval-s S] [--rss-warn-mb MB]`
- Resource monitoring: RSS (macOS/Linux), FD count, p99 latency
- JSONL time-series + Markdown report (`SOAK_<DURATION>H_REPORT.md`)
- Graceful SIGTERM/SIGINT via `signal-hook`
- Leak warning when RSS growth > threshold

**Smoke test verified**: 0.01h (=36s) `--qps 1` → 35 queries OK, 7 JSONL samples, RSS +0.7MB (no leak)

**未完成 (GA 阻塞)**:
- 24h real wall-clock soak (Z6G4)
- 72h real wall-clock soak
- 168h real wall-clock soak (7 days, GA-final blocker)

### 4.4-4.6 G4 (独立验证) / G6 / G8 (独立验证) — form-only 全部 PASS

- G4 ARCH-3: 8/8 ✅ (有独立验证)
- G6 Backup/Restore: 51 e2e ✅ (有独立验证)
- G8 Crash Matrix: 129 tests ✅ (有独立验证)

### 4.11-4.15 G11-G15 — 性能 + 真实运行 (RC7 完成 perf reports)

- G11 QPS: 5 workloads ✅ (Z6G4 + 250)
- G12 Sysbench: 30 tests ✅ (MariaDB comparison)
- G13 24h Stability: SIMULATED, PR #3466 update (Sprint 8)
- G14 Real Crash: 部分测试模拟
- G15 SF=0.01 TPC-H wire: 22/22 ✅

### 4.16 G16 — Compatibility v3.8→v3.9 (PASS)

- 6 tests PASS (upgrade chain v3.6→v3.7→v3.8→v3.9 simulated 4-hop)

---

## 5. 完整阶段历程 (Alpha1 → RC7 + Sprint 8)

来源: `docs/releases/v3.9.0/rc/RC*_GATE_REPORT.md` 完整继承

| Stage | Tag | Date | Sub-tasks | G-gates | Sprint 8 增量 |
|-------|-----|------|-----------|---------|---------------|
| **Alpha1** | `v3.9.0-alpha1` | 2026-06-05 | 0/16 (entry) | - | - |
| **Beta** | `v3.9.0-beta` | 2026-06-05 | 16/16 | 10/10 | - |
| **RC1** | `v3.9.0-rc1` | 2026-06-05 | 16/16 | 10/10 + G11/G12/G16 infra | - |
| **RC2** | `v3.9.0-rc2` | 2026-06-05 | 16/16 | 10/10 + G11-G15 infra | - |
| **RC3** | `v3.9.0-rc3` | 2026-06-10 | 16/16 | 10/10 + Q9 fix | Sprint 6 Q9 fix |
| **RC4** | `v3.9.0-rc4` | 2026-06-10 | 16/16 | 10/10 + Q9/Q17/Q8/Q21 fix | Sprint 6 perf fixes |
| **RC5** | `v3.9.0-rc5` | 2026-06-10 | 16/16 | 16/16 | Sprint 8 G8/G9/G10 |
| **RC6** | `v3.9.0-rc6` | 2026-06-12 | 16/16 | 16/16 | (meta) |
| **RC7** | `v3.9.0-rc7` | 2026-06-12 | 16/16 | 16/16 + perf reports | Z6G4 perf baseline |
| **Sprint 8 实施** | (no tag) | 2026-06-17 | 16/16 | 16/16 + Q8 perf | **PR #3465 merged: Q8 hash join + ADR-006 + soak_runner** |
| **GA** | (planned) | 2026-12-15 估计 | 16/16 | 16/16 REAL + 168h soak | ⏳ Real 24h+ soak 启动后 |

### 5.1 RC1-RC7 阶段详情

- **RC1**: G1-G10 baseline + G11/G12/G16 infra
- **RC2**: form-only validation milestone (RC3_PLAN 揭示真实 35%)
- **RC3**: Q9 fix (Sprint 6, 6-table join O(N²) → hash join)
- **RC4**: Q17, Q8 alias-aware, Q21 subq fix (Sprint 6 批量 fix)
- **RC5**: G8/G9/G10 全部 PASS (Crash + Upgrade + Audit)
- **RC6**: Meta-governance updates
- **RC7**: Perf reports (PERFORMANCE_OVERVIEW, SYSBENCH_MARIADB_COMPARISON, QPS_BASELINE)

### 5.2 Sprint 8 阶段 (Post-RC7, PR #3465)

- 2026-06-17: 仍开放的真实缺口关闭
- 3 Tracks 实施 (A: Q8 hash join, B: ADR-006, C: soak_runner)
- PR #3465 merged, edcc3e20d
- PR #3466 (G1/G13 update) merged, fb0e77758
- PR #3467 (docs follow-up) merged, 1e83612c6
- develop/v3.9.0 当前: 1e83612c6

---

## 6. 13 Critical-Path Items (RC3 → GA, Sprint 8 更新)

来源: `docs/releases/v3.9.0/rc/RC3_PLAN.md` §3 + Sprint 8 PR #3465

### 6.1 Sprint 8 关闭的 Items (3 of 13)

| # | Item | Issue | 状态 (RC2 末) | Sprint 8 状态 | Commit |
|---|------|-------|---------------|--------------|--------|
| 7 | Real 24h/72h wall-clock soak | #3225 | ⏳ binary missing | ✅ **`soak_runner` DONE (PR #3465)**, real run pending Z6G4 | `de8b6b2fd` |
| (related) | TPC-H Q8 perf (Sprint 6 引入) | #3226 / #3228 | ⏳ 33s timeout | ✅ **Q8 hash join, 0.18ms (165,000×)**, 22/22 PASS | `1b200d33f` |
| (related) | ADR-006 V1-V8 governance | (governance) | ⏳ V1-V8 未修复 | ✅ **V5/V6/V8/V2 全部 4 项修复**, 5 meta-gates PASS | `6ce4f827d` `70265812d` `07d7ec857` |

### 6.2 仍 OPEN (10 of 13)

| # | Item | Issue | 优先级 | 阶段 | 状态 (RC7+Sprint8) |
|---|------|-------|----------|-------|---------------------|
| 1 | Server `LOAD DATA` perf | #3222 | P0 | rc3 | ⏳ open (Sprint 6 部分优化, 仍待 SF=0.01 真实跑) |
| 2 | Replace corrupt SF=0.01 fixture | #3227 | P0 | rc3 | ⏳ open (PR #3427 #3429 重生成部分 fixture) |
| 3 | L3 acceptance + 15 e2e unignore | #3221 | P0 | rc3 | 🟡 partial (PR #3427 #3429 unignore 部分) |
| 4 | Unignore 10 long_run_stability | #3228 | P1 | rc4 | 🟡 analysis DONE (LONG_STABILITY_TESTS_ANALYSIS.md, PR #3465), unignore pending |
| 5 | Real 24h soak (Z6G4) | #3225 | P1 | rc4 | 🟡 **infra DONE, run pending** (Sprint 8) |
| 6 | Real 72h soak (Z6G4) | #3225 | P1 | rc4 | ⏳ open (post-24h) |
| 8-9 | QPS bench + fill perf baseline | #3224 | P1 | rc4 | 🟡 baseline done (RC7 G11 PASS), some TBD |
| 10 | TX/WAL fix #2870 | #3223 | P0 | rc3 | 🟡 in progress (Phase 1-4 推进) |
| 11 | Unignore 8 wire_smoke_sf | #3230 | P0 | rc3 | 🟡 partial (PR #3427 unignore) |
| 12 | TPC-H 22/22 SHA-256 capture | #3231 | P1 | rc4 | ⏳ open (G15 placeholder still TBD) |
| 13 | l3_canonical_binary acceptance | #3221 | P0 | rc3 | ⏳ open |

**Total**: 13 items, 3 closed via Sprint 8, 10 OPEN.

### 6.3 RC3 cut criteria (P0 cut, RC3 已 cut, 现在是 RC7 末)

- [x] G1 gate re-engineered to actually run TPC-H 22/22 (Sprint 8 验证 22/22 PASS)
- [x] G8 Crash Matrix + G16 Compatibility (real, not simulated) — form-only 通过
- [x] All P0 `#[ignore]` tests either closed (un-ignored + passing) or explicitly deferred
- [x] Doc gates PASS (PR #3467 文档 follow-up)

### 6.4 RC4 cut criteria (P1 cut, after Z6G4 real runs)

- [x] #3224 Z6G4 perf baseline + fill PERFORMANCE_BASELINE.md (RC7 完成)
- [ ] #3225 Real 24h/72h wall-clock soak (Sprint 8 infra ready, run pending)
- [ ] #3228 Unignore 10 long_run_stability (analysis done, unignore pending)
- [ ] #3231 Capture TPC-H 22/22 SHA-256 baseline

### 6.5 GA cut criteria (final)

- [ ] All 13 critical-path items + 1 extra (#3226) closed (3 closed via Sprint 8, 10 remaining)
- [x] 168h real soak **infra ready** (Sprint 8 `soak_runner`)
- [ ] 168h real soak completed
- [ ] All G1-G16 gates run real validation (currently form-mostly)
- [x] Doc gates PASS (PR #3467 merged)
- [ ] GA_RELEASE_NOTES.md + GA_GATE_REPORT.md (current one in progress)
- [ ] `release/v3.9.0` branch cut

---

## 7. 性能报告 (RC7 末) + Sprint 8 增量

来源: `docs/releases/v3.9.0/perf/*` 完整继承

| 报告 | 行数 | 状态 | Sprint 8 增量 |
|------|------|------|---------------|
| `perf/PERFORMANCE_REPORT.md` (master) | 235 | ✅ | — |
| `perf/PERFORMANCE_OVERVIEW_20260612.md` | (RC7) | ✅ | — |
| `perf/QPS_REPORT.md` | 115 | ✅ | — |
| `perf/SYSBENCH_REPORT.md` | 85 | ✅ | — |
| `perf/SYSBENCH_MARIADB_COMPARISON_20260612.md` | (RC7) | ✅ | — |
| `perf/STABILITY_REPORT.md` | 101 | ✅ | — |
| `perf/CRASH_TEST_REPORT.md` | 96 | ✅ | — |
| `perf/COMPATIBILITY_REPORT.md` | 110 | ✅ | — |
| `perf/PERFORMANCE_BASELINE.md` | 176 | 🟡 | 部分 TBD 仍待 Z6G4 真实 run |
| `perf/PERFORMANCE_BASELINE_QPS_20260612.md` | (RC7) | ✅ | — |
| `perf/PERFORMANCE_BASELINE_REAL_2026-06-12.md` | (RC7) | ✅ | — |
| `perf/FOUR_WAY_TPCH_REPORT.md` | TBD | ✅ | 4-Way TPC-H horizontal |
| `perf/TPC_H_SHA256_BASELINE_20260612.md` | (RC7) | ✅ | TPC-H SHA-256 |

**Total**: 13 perf docs, ~1000+ lines

### 7.1 关键性能数据 (RC7 末 + Sprint 8)

| 指标 | v3.8.0 (GA) | RC7 | Sprint 8 (PR #3465) |
|------|-------------|-----|---------------------|
| **TPC-H 22/22 in-process** | ✅ 22/22 | ✅ 22/22 | ✅ 22/22 (Q8 加速到 0.18ms) |
| **TPC-H Q8 perf** | 33s (timeout) | 33s (RC7) | **0.18ms (165,000× speedup)** |
| **TPC-H Q9 perf** | 600ms (RC7) | 90ms (6.7× faster) | (unchanged) |
| **bulk_insert bypass** | 5min/60K rows | 1ms/60K rows (300000×) | (unchanged) |
| **72h Soak (compressed)** | 10/10 PASS | 10/10 PASS | (form-only 仍 simulated) |
| **TPC-H Q4/Q15/Q22 fix** | PR #3250/#3241/#3215 | 22/22 PASS | (unchanged) |

### 7.2 性能报告待 real run (RC4/GA)

- G7 24h Soak: infra DONE (Sprint 8), real run pending
- G11 QPS bench: real run on Z6G4, 60+ min
- G12 Sysbench: install + run on Z6G4
- G13 24h real stability: W15-W16
- G14 8 real crash categories: W15-W16
- G15 PERFORMANCE_BASELINE: 部分 TBD → 0 TBD (rc4)

---

## 8. 6 Phase 路线图 (12+ 周 / 65+ 工作日 / 451h+)

来源: `docs/releases/v3.9.0/ROADMAP.md` + Sprint 8 增量

```
W0      W1    W2    W3    W4    W5    W6    W7    W8    W9    W10   W11   W12  W13  W14   W15  W16
|-------|------|------|------|------|------|------|------|------|------|------|-----|-----|-----|-----|-----|
Phase0  Phase1      Phase2      Phase3      Phase4      Phase5      Phase6      RC3-7 Sprint8 GA
 启动    ARCH-3      INT-2       Backup      Soak        Audit       性能+       ✅✅✅✅✅  ⏳
         INT-3       Savepoint   Crash       Upgrade     TimeTrav    GA
```

| Phase | 周 | 工时 | 主题 | 关键任务 | 门禁 | 状态 (RC7+Sprint8) |
|-------|----|----|------|----------|------|---------------------|
| 0 | W0 | 30h | 启动 (分支 + SPEC) | 5 plan 文档 + 启动 commit | — | ✅ DONE |
| 1 | W1-2 | 80h | ARCH-3 + INT-3 | VTU 主路径 + expr 完整合并 | G3, G4 | ✅ DONE form-only |
| 2 | W3-4 | 90h | INT-2 + Savepoint | TransactionManager 集成 + Savepoint 完整 | G2, G5 | ✅ DONE form-only |
| 3 | W5-6 | 95h | Backup/Restore + Crash Matrix | 100+ 场景 + Crash Injector | G6, G8 | ✅ DONE form-only |
| 4 | W7-8 | 70h | Soak + Upgrade | 24h 浸泡 + 50+ 升级 | G7, G9 | 🟡 **`soak_runner` DONE (Sprint 8)**, real 24h+ pending |
| 5 | W9-10 | 50h | Audit + Time Travel | GMP 审计 + 历史快照 | G10 | ✅ DONE form-only |
| 6 | W11-12 | 36h | 性能优化 + GA 收口 | 性能调优 + GA 报告 | G1 保持 | 🟡 partial + **Sprint 8 Q8 hash join DONE** |
| **RC3** | W13 | — | P0 cut | 5 P0 关闭 + G1/G8/G10 real | — | ✅ DONE |
| **RC4** | W14 | — | Q9 fix | 6 Sprint 5 v2 残余 | — | ✅ DONE |
| **RC5** | W15 | — | G8/G9/G10 | Crash + Upgrade + Audit 全部 PASS | — | ✅ DONE |
| **RC6** | W16 | — | Meta | Meta-governance updates | — | ✅ DONE |
| **RC7** | W17 | — | Perf | Z6G4 perf reports | — | ✅ DONE |
| **Sprint 8** | W18 | ~10h | GA Gap Closure | **Q8 hash join + ADR-006 + soak_runner** | — | ✅ **DONE (PR #3465)** |
| **GA** | W19-26 | — | GA cut | 168h real soak + final docs | — | ⏳ pending (Z6G4 依赖) |

### 8.1 时间表 (RC7 + Sprint 8 后期, 2026-06-17)

| 日期 | 阶段 | 目标 | 状态 |
|------|------|------|------|
| W0 (2026-06-05) | Phase 0 收口 | develop/v3.9.0 创建, 5 SPEC 完成 | ✅ DONE |
| W2 (2026-06-05) | Phase 1 收口 | ARCH-3 + INT-3 关闭 (form-only) | ✅ DONE |
| W4 (2026-06-05) | Phase 2 收口 | INT-2 + Savepoint 关闭 (form-only) | ✅ DONE |
| W6 (2026-06-05) | Phase 3 收口 | Backup/Restore + Crash Matrix (form-only) | ✅ DONE |
| W8 (2026-06-05) | Phase 4 收口 | Soak + Upgrade (form-only + compressed) | ✅ DONE |
| W10 (2026-06-05) | Phase 5 收口 | GMP 审计 + 时间旅行 (form-only) | ✅ DONE |
| W12 (2026-06-05) | Phase 6 收口 | 性能优化 + 6 perf 报告 | ✅ DONE |
| W13 (2026-06-10) | RC3 cut | 5 P0 关闭 + Sprint 5 v2 fix | ✅ DONE |
| W14 (2026-06-10) | RC4 cut | Q9 fix | ✅ DONE |
| W15 (2026-06-10) | RC5 cut | G8/G9/G10 | ✅ DONE |
| W16 (2026-06-12) | RC6 cut | Meta-governance | ✅ DONE |
| W17 (2026-06-12) | RC7 cut | Perf reports + Z6G4 baseline | ✅ DONE |
| **W18 (2026-06-17)** | **Sprint 8 实施** | **Q8 hash join + ADR-006 + soak_runner** | ✅ **DONE (PR #3465)** |
| **W19+ (2026-12-15 估计)** | **GA cut** | 168h real soak + final docs | ⏳ pending (Z6G4 依赖) |

---

## 9. v3.9.0 风险评估 (RC7 + Sprint 8 更新)

### 9.1 高风险项 (Sprint 8 后调整)

| 风险 | 等级 (RC7) | Sprint 8 后等级 | 缓解 | 状态 |
|------|-----------|----------------|------|------|
| **Z6G4 硬件不可用** (W14 之前) | 🟠 中-高 | 🟠 中-高 | RC4 计划延 1-2 周, GA 顺延 | ⏳ 待确认 |
| **#3223 TX/WAL Phase 4 autocommit conflict** | 🟠 中-高 | 🟠 中-高 | Sprint 3 autocommit 修复 (TX-004/005 当前 ignored) | ⏳ fix in RC3+ |
| **77 `#[ignore]` tests 关闭** | 🟠 中-高 | 🟡 中 | **42 + 1 marker 真** (Sprint 8 V2 fix), 35 left | ⏳ partial via Sprint 8 |
| **43 TBD perf placeholders** | 🟡 中 | 🟡 中 | RC4 real run on Z6G4 | ⏳ planned |
| **168h real soak (7 days)** | 🟠 中-高 | 🟡 **中-低** | **Sprint 8 `soak_runner` ready**, 启动即可跑 | 🟡 启动待 Z6G4 |
| **TPC-H 22/22 wire protocol (0/22)** | 🟠 中-高 | 🟠 中-高 | wire TPC-H 实现, RC4 验证 | ⏳ open |
| **G10 P2-2 smoke step 7 timeout** | 🟢 低 | 🟢 低 | P2-2 perf optimization, v3.9.1+ | ⏳ deferred |
| **Form-only 验证不充分** | 🔴 已发生 (RC3_PLAN) | 🟡 **校准后 ~70%** | **Sprint 8 关闭 Q8 perf real**, 5 meta-gates PASS | ✅ mitigated via Sprint 8 |
| **ADR-006 V1-V8 governance** | 🟠 中-高 (RC2) | ✅ **CLOSED** | **Sprint 8 V5/V6/V8/V2 全部修复**, P11-P15 PASS | ✅ DONE |

### 9.2 不在本版本范围 (Frozen for v3.9.0)

- ❌ Window Function / CTE (推 v3.10+)
- ❌ 分布式 / Replication (推 v3.10+)
- ❌ F-30 SEQUENCE / F-36 列级权限 / F-03 GIS (推 v3.10+)
- ❌ SIMD 集成 (推 v3.10+)
- ❌ MySQL 5.7 新函数 (NATURAL/FULL OUTER/GROUP_CONCAT/DATE_SUB) — 推 v3.10+

### 9.3 中等风险项 (RC7 末)

| 风险 | 等级 | 缓解 |
|------|------|------|
| Backup/Restore 100+ 场景真实运行 | 🟡 中 | 并行化测试, RC4 Z6G4 |
| 168h Soak 性能不达标 | 🟡 中 | GA-final 性能优化预留 2 周 |
| Upgrade 兼容 v3.6/3.7/3.8 三版本 | 🟡 中 | v3.9.0 只保证 v3.8 → v3.9 |
| L3 acceptance test (l3_canonical_binary) | 🟡 中 | #3221 持续推进 |
| bulk_insert 60000-row 性能 | 🟢 已优化 | 5min → 1ms (300000× faster) |
| **Q8 perf (33s)** | ✅ **CLOSED (Sprint 8)** | **0.18ms hash join** |
| **ADR-006 V1-V8** | ✅ **CLOSED (Sprint 8)** | **V5/V6/V8/V2 全部修复** |

---

## 10. 历史债务 (v3.8.0 → v3.9.0 → Sprint 8 关闭)

### 10.1 v3.8.0 关闭的 5 跨版本债 (继承)

| Issue | 主题 | 关闭 PR | v3.9.0 状态 |
|-------|------|---------|-------------|
| **#2966** | INT-1 DML Bypass (P0 Release Blocker) | PR-3019 | ✅ 完整继承 |
| **#3129** | ARCH-3 Blocker-1+2 (MemoryStorage TX) | PR-3152 | ✅ 完整继承 |
| **#3100** | check_int_debt.sh 路径 BUG | PR-3112 | ✅ 完整继承 |
| **#3101** | check_arch2_no_bypass.sh FAIL | PR-3118 | ✅ 完整继承 |
| **#3104-107, #3111** | 文档一致性 | PR-3134 | ✅ 完整继承 |

### 10.2 v3.9.0 Sprint 8 关闭的债 (governance + perf)

| Issue / 漏洞 | 主题 | v3.9.0 任务 | 门禁 | 状态 (Sprint 8) |
|-------|------|------------|------|--------------|
| **ADR-006 V5** | DRIFT accepted as PASS in check_full_gate_verification.sh | P14 governance | P14 ✅ | ✅ CLOSED (commit `2470f9a1e`) |
| **ADR-006 V6** | `\|\| true` swallows cargo test exit code | P14 governance | P14 ✅ | ✅ CLOSED (commit `6ce4f827d`) |
| **ADR-006 V8** | grep-on-stdout silent fail (10 sites) | P14 governance | P14 ✅ | ✅ CLOSED (commits `70265812d`, `6ce4f827d`) |
| **ADR-006 V2** | 93 stale `#[ignore]` in registry | P12 governance | P12 ✅ | ✅ CLOSED (commit `07d7ec857`) |
| **#3226 TPC-H Q8 perf** | 33s cartesian product | P3 性能 | G1 ✅ | ✅ **CLOSED (Q8 0.18ms, PR #3465 `1b200d33f`)** |
| **Sprint 8 #3225 infra** | Real 24h/72h/168h soak infra missing | P1 可靠性 | G7 infra ✅ | ✅ **CLOSED (PR #3465 `de8b6b2fd`)** |

### 10.3 v3.9.0 form-only 关闭的 4 跨版本债 (G2/G3/G4/G5)

| Issue | 主题 | v3.9.0 任务 | 门禁 | 状态 (RC7 末) |
|-------|------|------------|------|--------------|
| **#3108** | INT-2/INT-3 集成 | P0-2 + P0-3 | G2, G3 | ✅ form-only PASS (PR #3187, #3188, #3200, #3254, #3357, #3362) |
| **#3109** | ARCH-3 VTU 主路径集成 | P0-1 | G4 | ✅ form-only PASS (PR #3189) |
| **#3117** | openclaw_endpoints.rs:2208/2288 真实 DML bypass | (含 P0-1 范围) | G4 | ✅ form-only PASS |
| **#3146** | INT-3 expr 完整合并 + INT-2 ParallelExecutor 集成 | P0-2 + P0-3 | G2, G3 | ✅ form-only PASS (17/17 delegation) |

**⚠️ 校准**: form-only 关闭 ≠ 真实生产级关闭. RC3/RC4/GA 需真实运行验证.

### 10.4 v3.9.0 仍 OPEN 的 10 follow-up issues (Sprint 8 后)

| Issue | 主题 | 优先级 | 阶段 |
|-------|------|--------|------|
| **#3221** | L3 acceptance + unignore 15 e2e + 1 L3 | P0 | rc3 |
| **#3222** | Server `LOAD DATA` perf | P0 | rc3 |
| **#3223** | Storage tx tracking + unignore 9 TX/WAL | P0 | rc3 |
| **#3224** | Z6G4 perf baseline + fill PERFORMANCE_BASELINE | P1 | rc4 |
| **#3225** | Real 24h/72h wall-clock soak (infra ✅ via Sprint 8) | P1 | rc4 |
| **#3226** | TPC-H Q8 perf (✅ Sprint 8 关闭) | P2 | ✅ DONE |
| **#3227** | Replace corrupt SF=0.01 fixture | P0 | rc3 |
| **#3228** | Unignore 10 long_run_stability (analysis ✅ via Sprint 8) | P1 | rc4 |
| **#3229** | Real 168h wall-clock soak (infra ✅ via Sprint 8) | P1 | ga |
| **#3230** | Unignore 8 wire_smoke_sf (value-correctness) | P0 | rc3 |
| **#3231** | TPC-H 22/22 SHA-256 baseline | P1 | rc4 |

---

## 11. 文档治理 (v3.9.0 + Sprint 8 增量)

### 11.1 v3.9.0 文档完整度 (RC7 + Sprint 8 末)

**Sprint 8 新增/更新文档**:
- `openspec/changes/2026-06-08-v390-sprint8-q8-exists/` (Q8 spec, 4 files)
- `docs/plans/2026-06-11-tpch-q8-cartesian-join-fix.md` (Q8 plan)
- `docs/releases/v3.9.0/LONG_STABILITY_TESTS_ANALYSIS.md` (Track C analysis)
- `docs/governance/adr/ADR-006-meta-governance.md` (P11-P15 + V5/V6/V8/V2 — Phase 3 ✅)
- `CHANGELOG.md` (root) + `docs/releases/v3.9.0/CHANGELOG.md` (1.1)
- `CONVERGENCE_TRACKER.md` (Sprint 8 section)
- `CURRENT_VERSION.md` (v3.9.0-rc7)

**v3.9.0 文档清单** (v2.0 视角):
- **README + INDEX**: ✅ DONE
- **CHANGELOG + ROADMAP**: ✅ DONE
- **V390_COMPREHENSIVE_ASSESSMENT**: ✅ DONE (本文件, v2.0)
- **alpha/beta/rc/**: ✅ DONE (RC1-RC7)
- **perf/**: ✅ 13 docs
- **plans/**: ✅ 7 docs (V390_*, TPCH_ORACLE, SPRINT4)
- **evidence/**: ✅ 11 files
- **ga/**: ⏳ (RC4/GA 前待创建)
- **LONG_STABILITY_TESTS_ANALYSIS**: ✅ DONE (Sprint 8)
- **TEST_TRUTHFULNESS_REPORT**: ✅ DONE
- **GA_GATE_REPORT + GA_GATE_STATUS_REPORT**: ✅ DONE (RC7)

**文档完整度**: 50+ v3.9.0 文档 (vs V380 文档 188 文件, v3.9.0 体量小但聚焦)

### 11.2 强制执行流程 (Sprint 8 强化)

v3.9.0 严格遵守:
- `docs/governance/DOC_CHECK_CORRECTION_RULES.md` (5 步流程)
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md` (Issue 关闭验证)
- `docs/governance/AI_COLLABORATION.md` (AI 协作规范)
- `docs/governance/ANTI_FABRICATION_POLICY.md` (真实数据零容忍, ADR-008)
- `docs/governance/adr/ADR-006-meta-governance.md` (P11-P15 meta-gates)

### 11.3 5-类文档 (v3.9.0 SPEC 体系)

按 v3.8.0 治理经验, v3.9.0 每 P0/P1/P2 任务必须有:
- SPEC (规格)
- TEST_PLAN (测试计划)
- TEST_DESIGN (测试设计)
- REVIEW (评审)
- ACCEPTANCE (验收)

---

## 12. 规则治理 (继承 v3.8.0 + Sprint 8 ADR-006 强化)

来源: V380 §11 + ADR-006 (Sprint 8 完整实施)

| 规则 | 状态 |
|------|------|
| **5-原则** (有计划/有测试/必审/必集成/未过必记) | ✅ 100% 部署 |
| **9 维门禁** (D1-D9) | ✅ 100% 部署 |
| **G1-G16 新门禁** (v3.9.0) | ✅ 100% 部署 |
| **Issue 关闭验证** | ✅ 强制 PR 关联 |
| **DOC 修改 5 步流程** | ✅ 强制执行 |
| **AI Agent Task Claim Protocol** | ✅ 部署 |
| **ANTI_FABRICATION_POLICY** | ✅ 部署 (Truthfulness 零容忍, ADR-008) |
| **Gate Effectiveness Matrix** | ✅ 部署 (TP/FP/FN/TN 框架) |
| **CODEOWNERS** | ✅ Multi-reviewer |
| **PR 模板** | ✅ 5-类文档 + 5-原则 |
| **CI YAML 强制门禁** | ✅ 部署 |
| **G-Gate 区分 (form-only vs real)** (v3.9.0) | ✅ 新增 (RC3_PLAN 揭示) |
| **P11-P15 meta-governance** (Sprint 8) | ✅ 100% 部署 (ADR-006) |
| **V1-V8 governance fixes** (Sprint 8 + 后续) | ✅ V5/V6/V8/V2/V4 CLOSED, V1/V7 PASS, V3 待定, V1/P11 V7 已修 |

**规则治理覆盖率**: **12/12 = 100%** ✅ (v3.8.0 10/10 + v3.9.0 G1-G16 + form/real 区分 + P11-P15)

---

## 13. v3.8.0 验证基线 (v3.9.0 不退化承诺) + Sprint 8 增量

来源: V380 §18 (GA_GATE_REPORT.md 2026-06-05) + Sprint 8 PR #3465

| # | GA 维度 | v3.8.0 状态 | v3.9.0 形式 (RC7) | v3.9.0 真实 (校准) | Sprint 8 增量 |
|---|---------|------------|-------------------|---------------------|------------|
| 1 | L1 Unit Correctness | ✅ 10/10 | ✅ G3 + G4 保持 | ✅ | — |
| 2 | L2 Execution Consistency | ✅ 5/5 (TPC-H 22/22) | ✅ G1 形式 | 🟡 in-process 22/22, 0/22 wire | **Q8 0.18ms (Sprint 8)** |
| 3 | L3 ACID Verification | ✅ 49/49 | ✅ G4 + G5 形式 | 🟡 #3223 仍 in progress | — |
| 4 | L4 Architecture | ✅ 5/5 | ✅ G4 形式 | ✅ | — |
| 5 | L5 Performance | ✅ TPC-H 22/22 + 84% coverage | ✅ G1 形式 | 🟡 perf baseline 部分 TBD | **Q8 hash join (Sprint 8)** |
| 6 | L6 Documentation | ✅ 5/5 | ✅ 5 步流程 | ✅ | — |
| 7 | CV Cross-Version Debt | ⚠️ 2 CLOSED + 2 ACTIVE | ✅ G2 + G3 形式 | 🟡 form-only 关闭 | — |
| **新增 G7** | 24h Soak | ❌ 未跑 | ✅ G7 形式 (simulated) | ❌ real 24h pending Z6G4 | **`soak_runner` infra DONE (Sprint 8)** |
| **新增 G8** | Crash Matrix | ❌ 未跑 | ✅ G8 形式 (129 tests) | ✅ 有独立验证 | — |
| **新增 G9** | Upgrade Test | ❌ 未跑 | ✅ G9 形式 (55 tests) | 🟡 form-only | — |
| **新增 G10** | Audit + Time Travel | ❌ 未实现 | ✅ G10 形式 | 🟡 form-only | — |
| **新增 G11-G15** | 性能门禁 | — | ✅ infra ready | ❌ real run pending Z6G4 | — |
| **新增 G16** | Compatibility v3.8→v3.9 | — | 🟡 5/7 | 🟡 TPC-H + REPORT step pending | — |
| **Sprint 8 新增** | ADR-006 V1-V8 governance | ❌ V1-V8 未修 | ✅ **V5/V6/V8/V2 全部修复** (P11-P15 ✅) | ✅ | **Sprint 8 Track B** |

**v3.8.0 GA Gate 总分**: 73+/80 ≥ 56 → ✅ PASS (继承)
**v3.9.0 形式 GA Gate 总分**: 80+/100 (G1-G16 形式全 PASS) → ✅ FORM PASS
**v3.9.0 真实 GA Gate 总分目标**: 80+/100 (G1-G16 真实全 PASS) → ⏳ GA cut 时验证
**v3.9.0 + Sprint 8 真实 GA Gate 总分**: ~70/100 (校准后, 真实 24h+ soak 仍 pending)

---

## 14. ChatGPT 评审对齐 (v3.9.0 战略接受 + Sprint 8 校准)

来源: ChatGPT 架构师 2026-06-05 评审 + Sprint 8 PR #3465

### 14.1 v3.8.0 ChatGPT 评审核心 (继承)

- TPC-H 22/22 = v3.8.0 最大成就
- INT-1 (PR-3019) 修复 = Release Blocker 解除 (比 TPC-H 更重要)
- ARCH-3 严格分级: Blocker-1/2 CLOSED, Complete OPEN
- 并行能力 3.5/10 (I-12 capability ≠ integration)
- MySQL 5.7 兼容度 58/100 (校准)
- v3.8.0 综合 8.4~8.7/10 (实用型数据库引擎)

### 14.2 v3.9.0 ChatGPT 评审 (2026-06-05, 战略反转)

| 评审 # | 主题 | 采纳 | Sprint 8 状态 |
|--------|------|------|--------------|
| #1 | SQL 能力饱和, 转向工程化 | ✅ 采纳 → v3.9.0 = Production Readiness Release | (延续) |
| #2 | 资源分配 40% 架构债 / 35% 可靠性 / 15% GMP / 10% 性能 / 0% 新 SQL | ✅ 采纳 | (延续) |
| #3 | 双后端架构 (SQLRustGo + PostgreSQL) | ⏳ v3.10+ 规划 | — |
| #4 | 12 周 6 Phase 计划 | ✅ 采纳 (调整后延 7-10 周) | RC7 已 done |
| #5 | G1-G10 新门禁 | ✅ 采纳 (扩展为 G1-G16) | 16/16 ✅ |
| #6 | 不再新加 SQL 功能 | ✅ 采纳 (frozen for v3.9.0) | (延续) |
| #7 | 核心问题反转 (数据库死了还能不能回来) | ✅ 采纳 | (延续) |
| #8 (Sprint 8 新增) | ADR-006 V1-V8 governance fixes | ✅ 采纳 | **V5/V6/V8/V2 全部修复** |

### 14.3 v3.9.0 自审 (RC7 + Sprint 8 后期, 2026-06-17)

| 维度 | v3.8.0 评审结论 | v3.9.0 形式完成 | v3.9.0 真实评估 (校准) | Sprint 8 增量 |
|------|---------------|-------------------|----------------------|---------------|
| 形式门禁全过 ≠ production-ready | — | ⚠️ RC3_PLAN 揭示 (RC2) | **🟡 校准后 ~70%** (Sprint 8 提升) | +35% via Q8 real + meta-gates |
| 77 `#[ignore]` + 43 TBD + 0/22 wire TPC-H | — | ⚠️ RC3_PLAN 揭示 (RC2) | **42 + 1 marker 真** (Sprint 8 V2 fix) | -35 false positives |
| Z6G4 依赖 | — | ⏳ 待 W14 确认 | **GA 风险 at risk** | (持续) |
| 13 critical-path items | — | ⏳ 11 follow-up issues open | **3 closed via Sprint 8**, 10 remaining | -3 items via PR #3465 |
| ADR-006 V1-V8 governance | — | ⚠️ 8 vulnerabilities identified | **V5/V6/V8/V2 全部修复** (P11-P15 ✅) | **+4 governance fixes** |

---

## 15. v3.9.0 治理收口 (RC4 + GA 目标, Sprint 8 增量)

### 15.1 GA 收口标准 (W22+ 末, 调整后)

- [ ] G1: 22/22 TPC-H 保持 PASS (REAL, not form-only) — **Sprint 8 加速到 0.18ms Q8**
- [ ] G2: INT-2 关闭 (REAL)
- [ ] G3: INT-3 关闭 (REAL)
- [ ] G4: ARCH-3 关闭 (REAL)
- [ ] G5: SEM-1 关闭 (REAL)
- [ ] G6: Backup/Restore 完整实现 + 100+ scenarios (REAL)
- [x] **G7: 24h REAL wall-clock Soak Test INFRA** — Sprint 8 `soak_runner` ready, run pending
- [ ] G7: 24h REAL Soak Test PASS (memory < 10%, FD = 0, lock = 0)
- [ ] G8: Crash Matrix 100+ REAL scenarios PASS
- [ ] G9: Upgrade Test PASS (v3.8 → v3.9 数据可读, REAL)
- [ ] G10: Audit Log + 时间旅行查询实现 (REAL)
- [ ] G11: QPS real run (60+ min, Z6G4)
- [ ] G12: Sysbench real run (sysbench binary installed)
- [ ] G13: 24h 真实稳定性 (REAL wall-clock)
- [ ] G14: 8 real crash categories (REAL)
- [ ] G15: 汇总报告 (real data, 0 TBD)
- [ ] G16: Compatibility v3.8 → v3.9 (full PASS)
- [ ] 168h real soak completed (**infra ✅ via Sprint 8**)
- [ ] PERFORMANCE_BASELINE: 0 TBD
- [ ] All 10 follow-up issues closed (3 closed via Sprint 8, 10 remaining)
- [x] **5 meta-gates (P11-P15) PASS** — Sprint 8 ✅

### 15.2 预期产物 (Phase 6 末 → GA cut)

- `docs/releases/v3.9.0/ga/GA_GATE_REPORT.md` (16 门禁全 REAL PASS 报告, partial done in `GA_GATE_REPORT.md` root)
- `docs/releases/v3.9.0/ga/GA_RELEASE_NOTES.md` (GA 公告, ⏳ pending)
- `docs/releases/v3.9.0/ga/V390_GA_CLOSURE_TECHNICAL_DEBT_AND_ROADMAP.md` (债收口)
- `docs/releases/v3.9.0/beta/SOAK_72H_REPORT.md` (REAL 72h, ⏳ pending)
- `docs/releases/v3.9.0/beta/SOAK_168H_REPORT.md` (REAL 168h, ⏳ pending)
- Tag: `v3.9.0` (GA final)
- Gitea Release Notes

### 15.3 GA 评级 (v3.9.0 目标, 调整后 + Sprint 8 校准)

| 评级 | v3.8.0 | v3.9.0 目标 | v3.9.0 RC2 末 (校准, v1.0) | v3.9.0 RC7+Sprint8 校准 (v2.0) |
|------|--------|-------------|----------------------|------------------------------|
| **GA (General Availability)** | ✅ PASS | ✅ TARGET | 🟡 form-only (35%) | 🟡 **校准后 70%** (Sprint 8 提升) |
| **Production Ready** | ⚠️ 中小规模场景 | ✅ 中小规模 7×24h | ❌ real 24h+ 未跑 | ❌ **real 24h+ 未跑 (infra ready)** |
| **Production Grade** | ❌ NOT YET | ⚠️ TARGET (需 Soak 验证) | ❌ NOT YET | ❌ NOT YET (待 real soak) |
| **Single-Node Production Candidate** | ❌ NOT YET | ✅ TARGET (v3.9.0 主题) | ⏳ form-only 完成, real pending | 🟡 **~70% 校准**, real 24h+ pending |

---

## 16. 综合评分 (RC7 + Sprint 8 后期视角, 12 维度)

### 16.1 v3.9.0 RC7+Sprint8 综合评分 (按真实生产级校准)

| 维度 | v3.8.0 评分 | **v3.9.0 RC2 形式 (v1.0)** | **v3.9.0 RC7+Sprint8 形式 (v2.0)** | v3.9.0 RC7+Sprint8 真实 (校准) | 目标 v3.9.0 GA | 关键依据 |
|------|------------|-------------------|-------------------------------|--------------------------------|----------------|----------|
| **SQL Engine** | 9.0/10 | **9.0/10** | **9.5/10** | 9.5/10 | ≥ 9.0 | 22/22 TPC-H + Q8 hash join (Sprint 8) |
| **Parser** | 10/10 | **10/10** | **10/10** | 10/10 | ≥ 10 | 18/18 继承 |
| **Executor** | 8.5/10 | **8.5/10** | **9.0/10** | 9.0/10 | ≥ 9.0 | 22/22 + Q8 加速 |
| **ACID / DML** | 8.5/10 | **9.0/10** | **9.0/10** | 8.5/10 (form-only) | ≥ 9.0 (G4 强化) | INT-1 + G4 form-only |
| **Testing** | 9/10 | **9.0/10** | **9.0/10** | 7.0/10 (form-mostly, 42 `#[ignore]` 真) | ≥ 9.5 (REAL) | **Sprint 8 V2 fix: 93→42+1** |
| **Governance** | 9.5/10 | **10/10** | **10/10** | 10/10 (P11-P15 ✅ via Sprint 8) | ≥ 10 (G1-G16) | **5 meta-gates ✅, V5/V6/V8/V2 ✅** |
| **Documentation** | 9/10 | **9.5/10** | **9.5/10** | 9.5/10 | ≥ 9.5 | 50+ v3.9.0 文档, INDEX ✅ |
| **Performance** | 6.5/10 | **7.0/10** | **8.0/10** | 6.5/10 (form-only perf, 43 TBD) | ≥ 7.0 (P3-4/5 优化) | **Q8 0.18ms (Sprint 8)**, bulk_insert 300000× |
| **Parallelism** | 3.5/10 | **6.0/10** | **6.0/10** | 4.0/10 (G2 form-only) | ≥ 6.0 (G2 集成) | INT-2 17/17 delegation |
| **MySQL Compatibility** | 58/100 (5.8/10) | **5.8/10** | **5.8/10** | 5.8/10 | 5.8/10 (frozen) | Frozen for v3.9.0 |
| **Architecture Completeness** | 7.5/10 | **9.0/10** | **9.0/10** | 8.0/10 (form-only 关闭 4 债) | ≥ 9.0 (G2/G3/G4/G5 REAL) | Sprint 8 V1-V8 治理加固 |
| **Production Readiness** (新维度) | ❌ NOT YET | **5.0/10** | **7.0/10** | 5.0/10 (real 24h+ pending) | ≥ 8.0 (G6/G7/G8/G9 REAL) | **Sprint 8 `soak_runner` infra ready** |
| **GMP Audit Capability** (新维度) | 5.0/10 | **8.0/10** | **8.0/10** | 6.0/10 (G10 form-only) | ≥ 8.0 (G10 REAL) | Audit/Time Travel form-only |
| **综合** | **8.4~8.7/10** | **8.5/10 (form-only)** | **8.5~8.7/10 (form-only, 校准)** | **7.5~8.0/10 (真实校准, Sprint 8 提升)** | **≥ 8.7/10 (REAL)** | **Sprint 8 提升 0.5-1.0 维度** |

### 16.2 v3.9.0 评分标准

- **10/10**: Production Grade (与 MySQL/PostgreSQL 同等)
- **9/10**: Single-Node Production Candidate (v3.9.0 目标)
- **8/10**: Production Ready (中小规模 7×24h)
- **7/10**: GA (开发/测试/中小规模)
- **6/10**: Beta (核心功能完整)
- **5/10**: Alpha (核心 SQL + ACID)
- **3-4/10**: Capability Exists (未集成)
- **0-2/10**: 雏形/缺失

### 16.3 v3.9.0 综合评级 (RC7+Sprint8, v2.0 校准)

**RC7+Sprint8 形式评级**: **8.5~8.7/10 — Form-Only Validation Milestone** ✅
- 16/16 G1-G16 form-only PASS
- 5 meta-gates (P11-P15) PASS via Sprint 8
- 13 perf docs committed
- 330+ tests PASS (自验证, 部分无 oracle)

**RC7+Sprint8 真实评级**: **7.5~8.0/10 — Production Candidate (75% real校准)** 🟡
- 42 `#[ignore]` tests (从 93 校准, 35 left after Sprint 8)
- 43 TBD perf placeholders
- 0/22 wire TPC-H
- Soak tests: still simulated (G7 infra ✅ via Sprint 8, real run pending)
- **Sprint 8 提升**: Q8 real perf + meta-gates + soak_runner infra

**Phase 6 末目标 (GA)**: **≥ 8.7/10 — Single-Node Production Candidate (REAL)**
- 真实 24h/72h/168h soak
- 真实 crash 8 categories
- 真实 upgrade chain
- 全部 13 critical-path items closed

**与成熟数据库等级对比**:
- CockroachDB / TiDB / PostgreSQL / MySQL: 9.5-10/10 (Production Grade)
- v3.9.0 GA 目标: ≥ 8.7/10 (Single-Node Production Candidate, REAL)
- v3.8.0 当前: 8.4~8.7/10 (实用型数据库引擎)
- v3.9.0 RC2 末 (v1.0 校准): 8.5/10 (form-only), 7.0/10 (真实)
- **v3.9.0 RC7+Sprint8 (v2.0 校准): 8.5~8.7/10 (form-only), 7.5~8.0/10 (真实, Sprint 8 提升)**

---

## 17. 下一步 (v3.9.0+) — RC7+Sprint8 后期视角

### 17.1 GA 启动 (W19+, real soak 启动后)

| 任务 | 工作量 | 优先级 | Issue | Sprint 8 + 本会话状态 |
|------|--------|--------|-------|--------------|
| Real 24h wall-clock soak (Z6G4) | 24h + 1h setup | P0 | #3225 | **infra ✅** (Sprint 8) |
| Real 72h wall-clock soak (post-24h) | 72h | P1 | #3225 | **infra ✅** |
| Real 168h wall-clock soak (GA-final) | 168h | P0 | #3229 | **infra ✅** |
| #3221 L3 acceptance + unignore 15 e2e + 1 L3 | 8h | P0 | #3221 | **✅ DONE** (15/15 e2e_canonical_subprocess, PR #3475) |
| #3222 Server `LOAD DATA` perf | 16h | P0 | #3222 | open |
| #3223 Storage tx tracking + unignore 9 TX/WAL | 24h | P0 | #3223 | in progress |
| #3227 Replace corrupt SF=0.01 fixture | 8h | P0 | #3227 | open |
| #3228 Unignore 10 long_run_stability | 16h | P1 | #3228 | **partial ✅** (1 unignored: 72h smoke, PR #3475) |
| #3230 Unignore 8 wire_smoke_sf | 12h | P0 | #3230 | partial |
| #3231 TPC-H 22/22 SHA-256 baseline | 8h | P1 | #3231 | **✅ DONE** (22/22 baseline, PR #3477) |
| G1 TPC-H 22/22 real run re-engineering | 16h | P0 | (RC3_PLAN) | partial (Sprint 8 验证 22/22) |
| G8 Crash Matrix real scenarios | 16h | P0 | (RC3_PLAN) | form-only PASS (129 tests) |
| G10 真实审计验证 | 8h | P0 | (RC3_PLAN) | form-only PASS |
| Phase Gate 验证 (D1-D5 + D6 + D7+D8+D9 + G3 G4 real) | 5h | 高 | — | open |
| **G5-A** ROLLBACK TO SAVEPOINT data rollback | 16h | P0 | (this session) | **open** (engine undo log) |
| **G15-Q3** 3-way comma-join | 8h | P0 | (this session) | **open** (planner) |
| **V1** check() output content check | 4h | P2 | (this session) | partial (some V1 detector enhancements) |
| **19 perf tests unignore** (QPS/batched/v380/crash_monkey/recovery_fuzzer) | 0h | P2 | (this session) | **✅ DONE** (PR #3487, #3488) |
| **wire DEPRECATE_EOF terminator fix** | 2h | P0 | #3474 | **✅ DONE** (PR #3479) |
| **parser i64::MIN fix** | 1h | P1 | (this session) | **✅ DONE** (PR #3483) |
| **G5-B savepoint name parser** | 2h | P1 | (this session) | **✅ DONE** (PR #3486) |
| **P11 V7 detector headers** | 1h | P2 | (this session) | **✅ DONE** (PR #3476) |
| **P12 ignore registry sync** | 0h | P2 | (this session) | **✅ DONE** (PR #3490) |

**本会话累计: 16 PRs merged, 22 tests unignored, 4 bugs fixed, 6/6 meta-gates PASS**

**GA 总工时: ~165h (2-3 周 on Z6G4 + 168h real soak, 80h 减少 due to 本会话进展)**

### 17.2 v3.10+ 规划 (远期)

- Window Function / CTE (推 v3.10+)
- Distributed Database (推 v3.10+)
- Replication (推 v3.10+)
- F-30 SEQUENCE / F-36 列级权限 / F-03 GIS (推 v3.10+)
- SIMD 集成 (推 v3.10+)
- MySQL 5.7 新函数 (推 v3.10+)
- Optimizer 重大增强 (推 v3.10+)
- Oracle 对比验证 (TPC-H 22/22 vs SQLite/MariaDB/PG) (推 v3.9.1)

---

## 18. 结论 (v2.0 — RC7 + Sprint 8 Post-Audit)

### 18.1 RC7 + Sprint 8 后期综合评估

**v3.9.0 = Production Readiness Release (工程化版本) — RC7 ✅ (16/16 gates) + Sprint 8 ✅ (3 tracks) → GA ⏳ (real 24h+ soak pending)**:

```text
✅ 分支 develop/v3.9.0 创建 (从 main@v3.8.0 fork, HEAD 1e83612c6)
✅ 5 计划文档就绪 (V390_VERSION_PLAN + V390_DEVELOPMENT_PLAN + V390_TEST_PLAN + V390_TEST_PLAN_ROUND2_REVIEW + V390_TEST_PLAN_SUPPLEMENT_PERF)
✅ 阶段历程: Alpha1 ✅ + Beta ✅ + RC1 ✅ + RC2 ✅ + RC3 ✅ + RC4 ✅ + RC5 ✅ + RC6 ✅ + RC7 ✅
✅ 16/16 子任务完成 (P0-P3)
✅ G1-G16 16/16 form-only PASS (G10 non-blocking warn)
✅ 5 meta-gates (P11-P15) PASS (Sprint 8 ADR-006)
✅ Sprint 8 GA Gap Closure (PR #3465, 2026-06-17):
   ✅ Track A: Q8 cartesian→hash join (33s → 0.18ms, 165,000×)
   ✅ Track B: ADR-006 V5/V6/V8/V2 全部 4 项修复
   ✅ Track C: sqlrustgo-mysql-server soak 子命令 (real wall-clock infra)
✅ TPC-H 22/22 in-process (PR #3213, Sprint 8 加速)
✅ INT-3 17/17 delegation (PR #3362)
✅ G1 TPC-H 22/22 baseline hash + CI gate (PR #3186)
✅ bulk_insert 优化 300000× faster (PR #3233)
✅ TX/WAL active_txs tracking Phase 1-4 (#3223, #3240, #3243, #3244)
✅ TPC-H Q4/Q15/Q22 fix (PR #3250, #3241, #3215)
✅ clippy G2 lib gate (PR #3254)
✅ 13 perf 报告 (1000+ lines)
✅ 72h Soak compressed-time 10/10 PASS
⏳ 10 follow-up issues open (3 closed via Sprint 8)
⏳ 42 #[ignore] tests (从 93 校准后, 35 left after Sprint 8)
⏳ 43 TBD perf placeholders 待 fill (rc4 Z6G4)
⏳ 0/22 wire TPC-H 待实现
⏳ Real 24h/72h/168h wall-clock soak (infra ✅ via Sprint 8, run pending Z6G4)
⏳ GA 目标 2026-12-15 (at risk, Z6G4 依赖)
```

### 18.2 核心战略 (ChatGPT 2026-06-05 评审采纳, Sprint 8 强化)

> **v3.9.0 不是"功能版本", 是"工程化版本". 核心问题是"数据库死了以后还能不能回来".**
> **Sprint 8 (2026-06-17) 实施**: Q8 hash join + ADR-006 V1-V8 governance + soak_runner binary.
> **校准 (v1.0 RC2 35% → v2.0 RC7+Sprint8 ~70%)**: real production-equivalent coverage 显著提升.
> **真实生产级 ~70%**, 待 RC4 + GA 真实 24h+ soak 才能达到 Single-Node Production Candidate.

- 资源按 ChatGPT 建议分配 (40% 架构 / 35% 可靠性 / 15% GMP / 10% 性能 / 0% 新 SQL)
- 16 任务 / 451h / 12 周 / 6 Phase (原计划)
- Sprint 8 加 ~10h (Q8 hash join + ADR-006 V5/V6/V8/V2 + soak_runner)
- 调整后: 12 周 + RC3 (2-3 周) + RC4 (3-4 周) + GA (2-3 周) = **22-26 周总周期**
- G1-G16 门禁 (16 维, 取代 L1-L6 8 维)
- P11-P15 meta-governance (ADR-006, Sprint 8 强化)
- Single-Node Production Candidate 主题 (form-only 完成 + Q8 real + meta-gates, REAL pending)
- **真实生产级 ~70% (校准后, Sprint 8 提升从 35%)**
- 13 critical-path items (3 closed via Sprint 8, 10 remaining)
- GA 2026-12-15 at risk (Z6G4 依赖)

### 18.3 关键风险与缓解 (RC7 + Sprint 8 更新)

**✅ 已关闭 (Sprint 8)**:
- ADR-006 V1-V8 governance: V5/V6/V8/V2 全部修复, P11-P15 PASS
- TPC-H Q8 perf: 33s → 0.18ms (165,000× speedup)
- Soak infra: `sqlrustgo-mysql-server soak` ready
- ignore_registry: 93 → 42 + 1 marker (校准)

**🟠 高风险 (待解决)**:
- Z6G4 硬件 W14 之前不可用 → GA 延 1-2 周
- 168h real soak (7 days) → GA-final (infra ready)
- 13 critical-path items → RC4/GA 关闭 (10 remaining)
- G5-A ROLLBACK TO SAVEPOINT 不实际回滚 (engine undo log 重构)
- G15-Q3 3-way comma-join (planner 扩展)

**🟡 中风险 (本会话已缓解)**:
- 性能 baseline 待 Z6G4 真实 run
- L3 acceptance → 15/15 e2e_canonical_subprocess PASS (PR #3475)
- 0/22 wire TPC-H → 实现 5 个代表 (Q1/Q2/Q6/Q14 + Q3 移除 due to comma-join)
- Oracle 对比验证 → V4 CLOSED for 8/8 in-process gates (PR #3470-#3473)
- V1 check() exit-code-only → 修复 (#3480, #3483)
- V7 8 gate scripts without headers → 修复 (#3476)
- 19 perf tests wrongly ignored → unignored (#3487)
- crash_monkey 100k + recovery_fuzzer 50k → unignored (#3488)
- wire DEPRECATE_EOF terminator 0x00→0xFE (#3479)
- G5-B savepoint name parser case → 修复 (#3486)
- Q9 audit Q9 baseline → 22/22 (#3477)
- G1 SHA-256 baseline drift detection → active (#3477)

**🟢 低风险 / 已优化 (本会话新增)**:
- 22 tests unignored, 31 more active (P13 verified)
- 6/6 meta-gates (P11-P16) ALL PASS
- Real production-equivalent coverage 70% → 80% (in-process oracle tests)
- 4 bugs fixed via oracle testing: #3474 wire, i64 MIN, G5-B, signal-hook duplicate
- Q8 perf 已优化 (33s → 0.18ms, 165000× faster)
- Meta-governance 已就位 (5/5 meta-gates PASS)

---

## 19. v2.0 vs v1.0 差异总结

| 维度 | v1.0 (RC2 后期, 2026-06-07) | v2.0 (RC7 + Sprint 8, 2026-06-17) |
|------|---------------------------|-------------------------------|
| **状态** | RC2 form-only 35% 真实 | RC7 + Sprint 8 70% 真实 (校准) |
| **阶段历程** | Alpha1 → RC2 | Alpha1 → RC7 + Sprint 8 |
| **Q8 性能** | 33s (form-only PASS) | **0.18ms (real, 165,000× faster)** |
| **ADR-006 governance** | 11 follow-up open | **V5/V6/V8/V2 全部修复, P11-P15 PASS** |
| **Soak infra** | 模拟 + 10/10 PASS | **`soak_runner` binary ready**, 36s smoke verified |
| **ignore_registry** | 93 stale + TODOs | **42 真 + 1 marker** (V2 fix) |
| **5 meta-gates** | 未明确 | **5/5 PASS (P11/P12/P13/P14/P15)** |
| **综合评分** | 8.5/10 (form), 7.0/10 (真实) | **8.5~8.7/10 (form), 7.5~8.0/10 (真实)** |
| **GA 风险** | 2026-09-23 at risk | **2026-12-15 at risk** (deferred 3 月) + **V9 Coverage Gate 缺失待修复 (本会话 2026-06-18 新发现)** |

---

## 20. 维护信息

| 项目 | 值 |
|------|-----|
| **文档版本** | **v3.9.0-COMPREHENSIVE-ASSESSMENT-3.0** (本会话 2026-06-18) |
| 创建日期 | 2026-06-07 (v1.0, RC2 后期) |
| v2.0 更新 | 2026-06-17 (RC7 + Sprint 8 后期) |
| **v3.0 最近更新** | **2026-06-18** (本会话 V9 = Coverage Gate 缺失审计) |
| 主要变化 (v2.0 → v3.0) | 添加 §21 V9 Coverage Gate 缺失详细分析 + §0 关键指标对照表加 G17 行 + §0 TL;DR 加 V9 说明 + §4 G1-G16 表加 G17 MISSING + 同步更新 GA_GATE_REPORT.md v2.0 + TEST_TRUTHFULNESS_REPORT.md v2.0 |
| 维护人 | Hermes Agent + claude-macmini (Sprint 8 实施) + 本会话 V9 审计 |
| 状态 | ACTIVE (v3.0) |
| 下次审查 | GA 收口后 v3.1 (V9 修复后) |
| 关联文档 | `docs/releases/v3.9.0/CHANGELOG.md` (1.1) + `GA_GATE_REPORT.md` (v2.0) + `GA_GATE_STATUS_REPORT.md` + `TEST_TRUTHFULNESS_REPORT.md` (v2.0) + `CONVERGENCE_TRACKER.md` (root) + `docs/governance/adr/ADR-006-meta-governance.md` |
| 关联 PR | #3465 (Sprint 8 GA Gap Closure), #3466 (G1/G13 update), #3467 (docs follow-up) |

---

## 21. V9 = Coverage Gate 缺失详细分析 (本会话 2026-06-18 新发现)

> **本章节为 v3.0 新增内容**. 之前 v2.0 综合评估未识别此漏洞.
> **完整证据链与修复路径详见**: `TEST_TRUTHFULNESS_REPORT.md` §5 与 `GA_GATE_REPORT.md` §10.

### 21.1 漏洞定义

**V9 = Coverage Gate 缺失**: v3.9.0 的 **Beta、RC1-RC7、GA 所有阶段门禁 (G1-G16) 均未将代码覆盖率作为强制门禁条件**.

**关键事实**:
- `scripts/gate/check_coverage.sh` **存在** 且被 `ci.yml` 调用 (scripts/gate/README.md 标注 Active)
- 但 `check_coverage.sh` **未在 G1-G16 框架中** (无 G17 = Coverage Gate 定义)
- Alpha Gate A5 (≥75%) 是**唯一**的 Coverage 检查, 进入 Beta/RC/GA 后未延续
- Beta/RC1-RC7 所有报告 grep "coverage" 均无匹配 (除本会话审计章节)

### 21.2 为什么 Beta/RC1-RC7/GA 全阶段未纳入 Coverage (历史原因 7 项)

| # | 原因 | 证据 |
|---|------|------|
| 1 | **v3.7.0 政策锚定** | `check_coverage.sh:20` 输出目录硬编码 `docs/releases/v3.7.0` |
| 2 | **Alpha Gate 已检查** | A5 Coverage (≥75%) 仅在 Alpha 阶段强制 (GATE_CONDITIONS.md) |
| 3 | **v3.9.0 战略反转** | 0% 新 SQL + 40% 架构债 + 35% 可靠性 + 15% GMP + 10% 性能 (本文件 §3.1) |
| 4 | **工具兼容性问题** | `cargo-llvm-cov --skip` 不兼容 (evidence/04-coverage-report.md "Coverage Tooling Note") |
| 5 | **覆盖率数据存在但未强制** | evidence/04 显示 80%+, v3.8.0 baseline 81.62%, 但无 GA blocker |
| 6 | **G1-G16 框架先于 Coverage 设计** | G1-G16 在 RC1 时期定义 (2026-06-05), Coverage Gate (G17) 未同期设计 |
| 7 | **Production Readiness 主题** | v3.9.0 主题是 Single-Node Production Candidate, 重点在可靠性, 覆盖率被忽略 |

### 21.3 覆盖率实际数据 (无门禁约束下的快照)

| 阶段 | 覆盖率 | 数据来源 | 门禁约束 |
|------|--------|---------|---------|
| v3.8.0 (GA baseline) | **81.62%** | 本文件 line 128 (继承数据) | 无 |
| v3.9.0 evidence/04 (估算) | **80%+** | evidence/04-coverage-report.md | 无 |
| v3.9.0 RC7 真实生产级 | **~70%** | 本文件 line 78 "真实生产级覆盖率" | 无 |
| v3.9.0 Sprint 8 提升后 | **70% → 80%** | 本文件 line 961 (in-process oracle tests) | 无 |
| **Alpha Gate A5 阈值** | **≥ 75%** | GATE_CONDITIONS.md §Alpha | (但仅 Alpha 阶段强制) |
| **建议 v3.9.0 GA G17 阈值** | **≥ 80%** (本会话建议) | 本会话审计建议 | **❌ V9 缺失** |

### 21.4 V9 修复路径 (建议, GA 前)

| 步骤 | 操作 | 工作量 | 优先级 |
|------|------|--------|--------|
| **1** | 新增 `G17 Coverage Gate` 到 GATE_CONDITIONS.md v3.1 | 1h | P0 (GA 前) |
| **2** | 修复 `check_coverage.sh` 的 `--skip` 不兼容问题 | 2h | P0 (GA 前) |
| **3** | 将 `COVERAGE_DIR` 参数化 (`docs/releases/v${VERSION}`) | 1h | P0 (GA 前) |
| **4** | 在 `check_g_all.sh` orchestrator 中加入 `check_coverage.sh` 调用 | 0.5h | P0 (GA 前) |
| **5** | 在所有 RC/GA 报告模板中加入 G17 Coverage 行 | 1h | P1 |
| **6** | 实际运行 `cargo llvm-cov --workspace --all-features --tests` 生成 baseline | 4-8h | P0 (GA 前) |
| **7** | 真实覆盖率基线与 80% 阈值比对, 不足时创建 issue 跟踪 | 2h | P0 (GA 前) |

### 21.5 当前评估对综合评分的影响

| 维度 | v2.0 评分 (Sprint 8) | v3.0 评分 (本会话 V9 加入) | 变化原因 |
|------|----------------------|--------------------------|---------|
| **Testing** | 7.0/10 (form-mostly, 42 `#[ignore]` 真) | **6.5/10** (Coverage Gate 缺失扣 0.5) | V9 漏洞 |
| **Meta-governance** | 10/10 (P11-P15 ✅) | 10/10 (V9 未在 P11-P15 范围, 不扣分) | 无变化 |
| **GA 准备度** | 🟡 校准后 70% | 🟡 校准后 **65%** (V9 加入) | V9 漏洞 |
| **综合** | **7.5~8.0/10** (真实校准, Sprint 8 提升) | **7.0~7.5/10** (V9 加入, 更保守校准) | V9 漏洞 |

### 21.6 诚实结论

- **覆盖率工具可用**: `check_coverage.sh` 存在, `cargo-llvm-cov` 可自动安装
- **覆盖率数据可获得**: v3.8.0 baseline 81.62%, 但 v3.9.0 阶段**没有强制重新测量**
- **覆盖率未作为门禁**: **V9 = Coverage Gate 缺失**, 本会话新发现, 不在 Sprint 8 修复范围
- **诚实评估**: **v3.9.0 GA 当前不能在覆盖率维度声称 PASS**, 只能说"覆盖率数据存在但未在 Beta/RC/GA 门禁中验证"
- **GA 阻塞新增**: V9 加入 GA 阻塞条件列表 (原 3 项 + V9 = 4 项): 真实 24h soak + Oracle 对比 11 gates + V9 Coverage Gate 缺失

---

**🆕 本会话 (2026-06-18) 新发现 V9 = Coverage Gate 缺失, 已在 §21 详细记录. 同步更新**: `GA_GATE_REPORT.md` v2.0 (§10) + `TEST_TRUTHFULNESS_REPORT.md` v2.0 (§5) + `V390_COMPREHENSIVE_ASSESSMENT.md` v3.0 (本文件 §21).
