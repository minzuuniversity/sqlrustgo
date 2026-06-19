# SQLRustGo v3.9.0 路线图 — 6 Phase / 12 周 + RC3-RC7 + Sprint 8

<!-- env:blocked:no-ci -->

> **配套文档**: `README.md` (入口) / `plans/V390_VERSION_PLAN.md` (战略) / `plans/V390_DEVELOPMENT_PLAN.md` (任务) / `plans/V390_TEST_PLAN.md` (测试) / `V390_COMPREHENSIVE_ASSESSMENT.md` (综合评估 v2.0)
> **创建日期**: 2026-06-05
> **最近更新**: **2026-06-17** (v2.0, RC7 + Sprint 8 后期视角)
> **GA 目标**: **2026-12-15** (per Hermes audit #3252, deferred from 2026-09-23)
> **当前阶段**: **RC7 ✅ + Sprint 8 ✅ → GA 启动待 Z6G4 真实 24h+ soak**
> **关键决策**:
>   - Hermes 审计 #3252 — 真实生产级覆盖率仅 ~35% (RC2), 推迟 GA 3 月
>   - **Sprint 8 (PR #3465, 2026-06-17)** — 关闭 3 critical-path items (Q8 hash join + ADR-006 V5/V6/V8/V2 + soak_runner)
>   - **RC7 后 真实校准** ~70% (vs RC2 35%)

**当前 Sprint 8 已关闭**:
- [x] **Q8 cartesian→hash join** (PR #3465 `1b200d33f`) — 33s → 0.18ms (165,000×)
- [x] **ADR-006 V5/V6/V8/V2 全部 4 项修复** (PR #3465) — 5 meta-gates (P11-P15) PASS
- [x] **`sqlrustgo-mysql-server soak` 子命令** (PR #3465 `de8b6b2fd`) — real wall-clock infra ready
- [x] **LONG_STABILITY_TESTS_ANALYSIS.md** (PR #3465 `fb2503061`) — 26 long tests 文档化

**仍待关闭 (10 of 13 critical-path items)**:
- [ ] **#3221** L3 acceptance + unignore 15 e2e + 1 L3
- [ ] **#3222** Server `LOAD DATA` perf
- [ ] **#3223** Storage tx tracking + unignore 9 TX/WAL
- [ ] **#3227** Replace corrupt SF=0.01 fixture
- [ ] **#3228** Unignore 10 long_run_stability (analysis ✅ via Sprint 8, unignore pending)
- [ ] **#3230** Unignore 8 wire_smoke_sf
- [ ] **#3231** TPC-H 22/22 SHA-256 baseline
- [ ] **#3225** Real 24h/72h wall-clock soak (**infra ✅ via Sprint 8**, run pending Z6G4)
- [ ] **#3224** Z6G4 真实 perf 测量
- [ ] **#3229** Real 168h wall-clock soak (infra ✅ via Sprint 8, run pending)

---

## 0. 总览 (时间轴, RC7 + Sprint 8 后期)

```
W0      W1    W2    W3    W4    W5    W6    W7    W8    W9    W10   W11   W12   W13   W14   W15   W16   W17   W18   W19+
|-------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|------|
Phase0  Phase1      Phase2      Phase3      Phase4      Phase5      Phase6      RC3  RC4  RC5  RC6  RC7  Sprint8 GA
 启动    ARCH-3      INT-2       Backup      Soak        Audit       性能+       ✅   ✅   ✅   ✅   ✅   ✅      ⏳
         INT-3       Savepoint   Crash       Upgrade     TimeTrav    GA
```

| Phase | 周 | 工作日 | 工时 | 主题 | 状态 |
|-------|----|----|----|------|------|
| 0 | W0 | 5 | 30h | 分支 + SPEC + 启动 commit | ✅ DONE |
| 1 | W1-2 | 10 | 80h | ARCH-3 VTU 主路径 + INT-3 expr 完整合并 | ✅ DONE form-only |
| 2 | W3-4 | 10 | 90h | INT-2 TransactionManager 集成 + Savepoint 完整 | ✅ DONE form-only |
| 3 | W5-6 | 10 | 95h | Backup/Restore + Crash Matrix (100+ 场景) | ✅ DONE form-only |
| 4 | W7-8 | 10 | 70h | 24h Soak + Upgrade (50+ 场景) | ✅ DONE form-only (real pending Z6G4) |
| 5 | W9-10 | 10 | 50h | Audit + Time Travel (40+ 测试) | ✅ DONE form-only |
| 6 | W11-12 | 10 | 36h | 性能优化 + GA 收口 | 🟡 partial + Sprint 8 Q8 hash join |
| **RC3** | W13 | — | P0 cut | 5 P0 关闭 + G1/G8/G10 real | ✅ DONE |
| **RC4** | W14 | — | Q9 fix | 6 Sprint 5 v2 残余 + Q9 hash join | ✅ DONE |
| **RC5** | W15 | — | G8/G9/G10 | Crash + Upgrade + Audit 全部 PASS | ✅ DONE |
| **RC6** | W16 | — | Meta | Meta-governance updates | ✅ DONE |
| **RC7** | W17 | — | Perf | Z6G4 perf reports + G1/G13 update | ✅ DONE |
| **Sprint 8** | W18 | ~10h | GA Gap Closure | **Q8 hash join + ADR-006 + soak_runner** | ✅ **DONE (PR #3465)** |
| **GA** | W19+ | — | GA cut | 168h real soak + final docs | ⏳ pending (Z6G4 依赖) |
| **总计** | **12+ 周** | **65+ 天** | **~461h** | — | — |

---

## 1. Phase 0 — 启动 (W0 / 5d / 30h)

**目标**: v3.9.0 分支 + 计划文档 + 启动 commit 落地

| 任务 | 时长 | 状态 |
|------|------|------|
| 创建 `develop/v3.9.0` 分支 | 0.5h | ✅ |
| 拉取 Gitea 分支 | 0.5h | ✅ |
| 移动 V390 plan 3 文件到 v3.9.0/plans/ | 0.5h | ✅ |
| 创建 `docs/releases/v3.9.0/` 目录 + README + CHANGELOG + ROADMAP | 2h | ✅ |
| 6 issues milestone → v3.9.0 | 0.5h | ✅ |
| 启动 commit "chore: v3.9.0 branch init" | 0.5h | ✅ |
| AGENTS.md main branch v3.8.0 → v3.9.0 | 0.5h | ✅ |
| 顶层 CHANGELOG.md 迁移 v3.8.0 段 | 0.5h | ✅ |
| Gitea milestone 公告 + 通知 | 1h | ✅ |
| Phase 0 Gate 验证 (D1-D5 + D6 + D9) | 2h | ✅ |

**门禁**: 无 (启动阶段, 不阻塞)

---

## 2. Phase 1 — ARCH-3 + INT-3 (W1-2 / 10d / 80h)

**目标**: VTU 主路径集成 + expr 完整合并

| 任务 | 工时 | 依赖 | 门禁 | 状态 |
|------|------|------|------|------|
| **ARCH-3**: VTU Guard 完整集成到 VTO/Selection 主路径 | 40h | #3109 (P1) | G4 | ✅ DONE form-only (8/8) |
| **INT-3**: Expr 完整合并 (EVAL chain) | 30h | #3146 follow-up | G3 | ✅ DONE form-only (17/17 delegation) |
| 文档同步 (FEATURE_MATRIX + DEBT_REGISTRY) | 5h | — | — | ✅ |
| Phase 1 Gate (D1-D5 + D6 + D7+D8+D9) | 5h | — | G3 G4 | ✅ |

**预期产物**:
- ARCH-3 state: IN_PROGRESS 60% → 100% (VERIFIED) ✅
- INT-3 state: OPEN → CLOSED ✅
- PR: 2-3 个 (VTU 主路径 / Expr chain / 文档同步) — #3189, #3200, #3254, #3357, #3362

---

## 3. Phase 2 — INT-2 + Savepoint (W3-4 / 10d / 90h)

**目标**: TransactionManager 集成 + Savepoint 完整

| 任务 | 工时 | 依赖 | 门禁 | 状态 |
|------|------|------|------|------|
| **INT-2**: TransactionManager 完整集成 (主路径) | 50h | #3108 (P0) | G2 | ✅ DONE form-only (13 tests) |
| **SEM-1**: Savepoint MVCC snapshot restore | 30h | #3146 + INT-2 | G5 | ✅ DONE form-only (8/8) |
| 文档同步 + 测试场景扩充 | 5h | — | — | ✅ |
| Phase 2 Gate | 5h | — | G2 G5 | ✅ |

**预期产物**:
- INT-2 state: OPEN → CLOSED ✅
- SEM-1 state: IN_PROGRESS 30% → 100% (VERIFIED) ✅
- PR: 2-3 个 (TX 主路径 / Savepoint / 测试) — #3357, #3362

---

## 4. Phase 3 — Backup/Restore + Crash Matrix (W5-6 / 10d / 95h)

**目标**: 备份恢复 + 100+ 崩溃场景覆盖

| 任务 | 工时 | 依赖 | 门禁 | 状态 |
|------|------|------|------|------|
| **Backup/Restore**: 100+ 场景 (full/incremental/point-in-time) | 50h | Phase 2 | G6 | ✅ DONE form-only (51 e2e tests) |
| **Crash Matrix**: 100+ 场景 (kill -9 / OOM / disk full) | 35h | Phase 2 | G8 | ✅ DONE (129 tests, 有独立验证) |
| 文档 + 测试报告 | 5h | — | — | ✅ |
| Phase 3 Gate | 5h | — | G6 G8 | ✅ |

**预期产物**:
- 100+ 备份恢复测试通过 ✅
- 100+ 崩溃场景覆盖 ✅ (129 tests)
- 报告: `docs/releases/v3.9.0/perf/CRASH_TEST_REPORT.md` + `perf/COMPATIBILITY_REPORT.md`

---

## 5. Phase 4 — Soak + Upgrade (W7-8 / 10d / 70h)

**目标**: 24h 浸泡 + 50+ 升级路径

| 任务 | 工时 | 依赖 | 门禁 | 状态 |
|------|------|------|------|------|
| **Soak Test**: 24h 连续运行 (1M txns) | 25h | Phase 3 | G7 | 🟡 **infra ✅** (Sprint 8 `soak_runner`), real run pending Z6G4 |
| **Upgrade Test**: 50+ 升级路径 (v3.6/3.7/3.8 → 3.9) | 35h | Phase 3 | G9 | ✅ DONE form-only (55 tests) |
| 文档 + 性能基线 | 5h | — | — | ✅ |
| Phase 4 Gate | 5h | — | G7 G9 | 🟡 partial |

**预期产物**:
- 24h Soak 报告 — **infra ready (Sprint 8)**, real run pending
- 50+ 升级测试通过 ✅
- 报告: `docs/releases/v3.9.0/beta/SOAK_72H_REPORT.md` (compressed) + `LONG_STABILITY_TESTS_ANALYSIS.md` (Sprint 8)

---

## 6. Phase 5 — Audit + Time Travel (W9-10 / 10d / 50h)

**目标**: 审计 + 时间旅行 40+ 测试

| 任务 | 工时 | 依赖 | 门禁 | 状态 |
|------|------|------|------|------|
| **Audit**: 40+ GMP 审计测试 | 25h | Phase 4 | G10 | ✅ DONE form-only |
| **Time Travel**: 历史快照查询 | 20h | Phase 2 | G10 | ✅ DONE form-only |
| 文档 | 5h | — | — | ✅ |

**预期产物**:
- 40+ 审计测试 ✅
- Time Travel 演示 ✅
- 报告: `docs/releases/v3.9.0/rc/AUDIT_TIME_TRAVEL.md`

---

## 7. Phase 6 — 性能 + GA 收口 (W11-12 / 10d / 36h) + Sprint 8

**目标**: 性能优化 + GA 报告

| 任务 | 工时 | 依赖 | 门禁 | 状态 |
|------|------|------|------|------|
| 性能优化 (TPC-H SF=1, latency/throughput) | 20h | Phase 5 | G1 保持 | ✅ DONE form-only |
| GA 报告 + 治理收口 | 10h | All | G1 | 🟡 in progress |
| 文档完整 + RC 标签 | 6h | — | — | ✅ DONE |
| **Sprint 8 Q8 hash join** (PR #3465) | ~3h | — | G1 ✅ | ✅ **DONE (0.18ms)** |
| **Sprint 8 ADR-006 V5/V6/V8/V2** (PR #3465) | ~5h | — | P11-P15 ✅ | ✅ **DONE (5/5 meta-gates)** |
| **Sprint 8 `soak_runner` binary** (PR #3465) | ~3h | — | G7 infra ✅ | ✅ **DONE (binary ready)** |

**预期产物**:
- 性能基线报告 ✅
- **Q8 cartesian→hash 优化 (Sprint 8)** — 33s → 0.18ms
- **ADR-006 V5/V6/V8/V2 治理 (Sprint 8)** — 5 meta-gates PASS
- **soak_runner binary (Sprint 8)** — real wall-clock infra
- GA 治理报告: `docs/releases/v3.9.0/ga/GA_GATE_REPORT.md` (current root: `GA_GATE_REPORT.md` RC7 in progress)
- Tag: `v3.9.0` (GA final, ⏳ pending)

---

## 8. 风险与缓解 (RC7 + Sprint 8 后期更新)

| 风险 | 概率 | 影响 | 缓解 | Sprint 8 状态 |
|------|------|------|------|--------------|
| Z6G4 硬件不可用 (W14 之前) | 中 | 高 | GA 顺延 1-2 周 | ⏳ 待确认 |
| **ADR-006 V1-V8 governance** | 中 | 高 | **Sprint 8 V5/V6/V8/V2 全部修复** | ✅ **CLOSED** (5 meta-gates PASS) |
| 168h real soak (7 days) | 中 | 高 | Sprint 8 `soak_runner` ready, 启动即可跑 | 🟡 **infra ✅** |
| 77 `#[ignore]` tests 关闭 | 中 | 中 | **Sprint 8 V2 fix: 93 → 42 + 1 marker** | 🟡 **35 left** |
| 43 TBD perf placeholders | 中 | 中 | RC4 real run on Z6G4 | ⏳ planned |
| 0/22 wire TPC-H | 中 | 高 | wire TPC-H 实现, RC4 验证 | ⏳ open |
| #3223 TX/WAL autocommit conflict | 中 | 高 | Sprint 3 autocommit 修复 | ⏳ fix in RC3+ |
| **TPC-H Q8 perf 33s** | 中 | 中 | **Sprint 8 hash join → 0.18ms** | ✅ **CLOSED** |
| Form-only 验证不充分 (RC3_PLAN 揭示) | 高 | 中 | **Sprint 8 校准到 ~70%** | 🟡 partial (real 24h+ pending) |

---

## 9. 关联文档

- `plans/V390_VERSION_PLAN.md` — 战略定位 + 16 任务总览
- `plans/V390_DEVELOPMENT_PLAN.md` — 详细 P0/P1/P2/P3 任务
- `plans/V390_TEST_PLAN.md` — G1-G10 门禁详细定义
- `V390_COMPREHENSIVE_ASSESSMENT.md` (v2.0) — 综合评估 RC7 + Sprint 8 视角
- `CHANGELOG.md` (1.1) — v3.9.0 变更日志 (Sprint 8 更新)
- `GA_GATE_REPORT.md` — 16/16 G1-G16 状态
- `LONG_STABILITY_TESTS_ANALYSIS.md` (Sprint 8) — 26 long tests 文档化
- `docs/governance/adr/ADR-006-meta-governance.md` (Sprint 8) — P11-P15 meta-gate framework
- `CONVERGENCE_TRACKER.md` (root) — Sprint 8 section
- `openspec/changes/2026-06-08-v390-sprint8-q8-exists/` — Q8 spec (Sprint 8)

---

## 维护信息

| 项目 | 值 |
|------|-----|
| 路线图版本 | v3.9.0-ROADMAP-2.0 |
| 创建日期 | 2026-06-05 |
| **最近更新** | **2026-06-17** (v2.0, RC7 + Sprint 8 后期视角) |
| 维护人 | Hermes Agent + claude-macmini (Sprint 8 实施) |
| 状态 | RC7 ✅ + Sprint 8 ✅ → GA 启动待 Z6G4 |
| 下次审查 | GA 收口后 v3.0 |
| 关联 PR | #3465 (Sprint 8), #3466 (G1/G13 update), #3467 (docs follow-up) |
