# SQLRustGo v3.9.0 路线图 — 6 Phase / 12 周

<!-- env:blocked:no-ci -->

> **配套文档**: `README.md` (入口) / `plans/V390_VERSION_PLAN.md` (战略) / `plans/V390_DEVELOPMENT_PLAN.md` (任务) / `plans/V390_TEST_PLAN.md` (测试)
> **创建日期**: 2026-06-05
> **GA 目标**: 2026-09-23 (at risk, 调整后 22-26 周, 参考 RC3_PLAN.md)
> **当前阶段**: Phase 6 收口 (form-only) → RC3 待启动

---

## 0. 总览 (时间轴)

```
W0      W1    W2    W3    W4    W5    W6    W7    W8    W9    W10   W11   W12
|-------|------|------|------|------|------|------|------|------|------|------|------|
Phase0  Phase1      Phase2      Phase3      Phase4      Phase5      Phase6
 启动    ARCH-3      INT-2       Backup      Soak        Audit       性能+
        INT-3       Savepoint   Crash       Upgrade     TimeTrav    GA
```

| Phase | 周 | 工作日 | 工时 | 主题 |
|-------|----|----|----|------|
| 0 | W0 | 5 | 30h | 分支 + SPEC + 启动 commit |
| 1 | W1-2 | 10 | 80h | ARCH-3 VTU 主路径 + INT-3 expr 完整合并 |
| 2 | W3-4 | 10 | 90h | INT-2 TransactionManager 集成 + Savepoint 完整 |
| 3 | W5-6 | 10 | 95h | Backup/Restore + Crash Matrix (100+ 场景) |
| 4 | W7-8 | 10 | 70h | 24h Soak + Upgrade (50+ 场景) |
| 5 | W9-10 | 10 | 50h | Audit + Time Travel (40+ 测试) |
| 6 | W11-12 | 10 | 36h | 性能优化 + GA 收口 |
| **总计** | **12 周** | **65 天** | **451h** | — |

---

## 1. Phase 0 — 启动 (W0 / 5d / 30h)

**目标**: v3.9.0 分支 + 计划文档 + 启动 commit 落地

| 任务 | 时长 | 状态 |
|------|------|------|
| 创建 `develop/v3.9.0` 分支 | 0.5h | ✅ |
| 拉取 Gitea 分支 | 0.5h | ✅ |
| 移动 V390 plan 3 文件到 v3.9.0/plans/ | 0.5h | ✅ |
| 创建 `docs/releases/v3.9.0/` 目录 + README + CHANGELOG + ROADMAP | 2h | ✅ |
| 6 issues milestone → v3.9.0 | 0.5h | 进行中 |
| 启动 commit "chore: v3.9.0 branch init" | 0.5h | 待执行 |
| AGENTS.md main branch v3.8.0 → v3.9.0 | 0.5h | 进行中 |
| 顶层 CHANGELOG.md 迁移 v3.8.0 段 | 0.5h | 进行中 |
| Gitea milestone 公告 + 通知 | 1h | 待执行 |
| Phase 0 Gate 验证 (D1-D5 + D6 + D9) | 2h | 待执行 |

**门禁**: 无 (启动阶段, 不阻塞)

---

## 2. Phase 1 — ARCH-3 + INT-3 (W1-2 / 10d / 80h)

**目标**: VTU 主路径集成 + expr 完整合并

| 任务 | 工时 | 依赖 | 门禁 |
|------|------|------|------|
| **ARCH-3**: VTU Guard 完整集成到 VTO/Selection 主路径 | 40h | #3109 (P1) | G4 |
| **INT-3**: Expr 完整合并 (EVAL chain) | 30h | #3146 follow-up | G3 |
| 文档同步 (FEATURE_MATRIX + DEBT_REGISTRY) | 5h | — | — |
| Phase 1 Gate (D1-D5 + D6 + D7+D8+D9) | 5h | — | G3 G4 |

**预期产物**:
- ARCH-3 state: IN_PROGRESS 60% → 100% (VERIFIED)
- INT-3 state: OPEN → CLOSED
- PR: 2-3 个 (VTU 主路径 / Expr chain / 文档同步)

---

## 3. Phase 2 — INT-2 + Savepoint (W3-4 / 10d / 90h)

**目标**: TransactionManager 集成 + Savepoint 完整

| 任务 | 工时 | 依赖 | 门禁 |
|------|------|------|------|
| **INT-2**: TransactionManager 完整集成 (主路径) | 50h | #3108 (P0) | G2 |
| **SEM-1**: Savepoint MVCC snapshot restore | 30h | #3146 + INT-2 | G5 |
| 文档同步 + 测试场景扩充 | 5h | — | — |
| Phase 2 Gate | 5h | — | G2 G5 |

**预期产物**:
- INT-2 state: OPEN → CLOSED
- SEM-1 state: IN_PROGRESS 30% → 100% (VERIFIED)
- PR: 2-3 个 (TX 主路径 / Savepoint / 测试)

---

## 4. Phase 3 — Backup/Restore + Crash Matrix (W5-6 / 10d / 95h)

**目标**: 备份恢复 + 100+ 崩溃场景覆盖

| 任务 | 工时 | 依赖 | 门禁 |
|------|------|------|------|
| **Backup/Restore**: 100+ 场景 (full/incremental/point-in-time) | 50h | Phase 2 | G6 |
| **Crash Matrix**: 100+ 场景 (kill -9 / OOM / disk full) | 35h | Phase 2 | G8 |
| 文档 + 测试报告 | 5h | — | — |
| Phase 3 Gate | 5h | — | G6 G8 |

**预期产物**:
- 100+ 备份恢复测试通过
- 100+ 崩溃场景覆盖
- 报告: `docs/releases/v3.9.0/beta/BACKUP_CRASH_REPORT.md`

---

## 5. Phase 4 — Soak + Upgrade (W7-8 / 10d / 70h)

**目标**: 24h 浸泡 + 50+ 升级路径

| 任务 | 工时 | 依赖 | 门禁 |
|------|------|------|------|
| **Soak Test**: 24h 连续运行 (1M txns) | 25h | Phase 3 | G7 |
| **Upgrade Test**: 50+ 升级路径 (v3.6/3.7/3.8 → 3.9) | 35h | Phase 3 | G9 |
| 文档 + 性能基线 | 5h | — | — |
| Phase 4 Gate | 5h | — | G7 G9 |

**预期产物**:
- 24h Soak 报告
- 50+ 升级测试通过
- 报告: `docs/releases/v3.9.0/beta/SOAK_UPGRADE_REPORT.md`

---

## 6. Phase 5 — Audit + Time Travel (W9-10 / 10d / 50h)

**目标**: 审计 + 时间旅行 40+ 测试

| 任务 | 工时 | 依赖 | 门禁 |
|------|------|------|------|
| **Audit**: 40+ GMP 审计测试 | 25h | Phase 4 | G10 |
| **Time Travel**: 历史快照查询 | 20h | Phase 2 | G10 |
| 文档 | 5h | — | — |

**预期产物**:
- 40+ 审计测试
- Time Travel 演示
- 报告: `docs/releases/v3.9.0/rc/AUDIT_TIME_TRAVEL.md`

---

## 7. Phase 6 — 性能 + GA 收口 (W11-12 / 10d / 36h)

**目标**: 性能优化 + GA 报告

| 任务 | 工时 | 依赖 | 门禁 |
|------|------|------|------|
| 性能优化 (TPC-H SF=1, latency/throughput) | 20h | Phase 5 | G1 保持 |
| GA 报告 + 治理收口 | 10h | All | G1 |
| 文档完整 + RC 标签 | 6h | — | — |

**预期产物**:
- 性能基线报告
- GA 治理报告: `docs/releases/v3.9.0/ga/GA_GATE_REPORT.md`
- Tag: v3.9.0

---

## 8. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| INT-2 主路径集成复杂度超预期 | 中 | 高 | Phase 2 末尾 Gate 严格, 必要时延 1 周 |
| 备份恢复 100+ 场景耗时 | 中 | 中 | 并行化测试, 优先核心 50 场景 |
| 24h Soak 性能不达标 | 中 | 高 | Phase 6 性能优化预留 2 周 |

---

## 9. 关联文档

- `plans/V390_VERSION_PLAN.md` — 战略定位 + 16 任务总览
- `plans/V390_DEVELOPMENT_PLAN.md` — 详细 P0/P1/P2/P3 任务
- `plans/V390_TEST_PLAN.md` — G1-G10 门禁详细定义

---

## 维护信息

| 项目 | 值 |
|------|-----|
| 路线图版本 | v3.9.0-ROADMAP-1.0 |
| 创建日期 | 2026-06-05 |
| 维护人 | Hermes Agent |
| 状态 | Phase 0 (启动) |
| 下次审查 | Phase 1 启动前 (W1) |
