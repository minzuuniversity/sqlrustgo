# v3.9.0 文档索引

<!-- env:blocked:no-ci -->

> **版本**: v3.9.0
> **类型**: **Production Readiness Release** (工程化版本, 非功能版本)
> **主题**: Single-Node Production Candidate
> **分支**: `develop/v3.9.0` (从 `main@v3.8.0` fork)
> **创建日期**: 2026-06-05
> **当前状态**: **RC7** — 门禁全 PASS，Soak 进行中，GA 待定

---

## 0. 当前状态（2026-06-25）

### 门禁状态

| Gate | 主题 | 状态 | 备注 |
|------|------|------|------|
| G1 | TPC-H 22/22 | ✅ PASS | RC7 |
| G2 | INT-2 ParallelExecutor | ✅ PASS | RC7 |
| G3 | INT-3 Expression | ✅ PASS | RC7 |
| G4 | ARCH-3 VtuGuard | ✅ PASS | RC7 |
| G5 | SEM-1 Savepoint | ✅ PASS | RC7 |
| G6 | Backup/Restore | ✅ PASS | RC7 |
| G7 | 24h Soak (simulated) | ✅ PASS | RC7 |
| G8 | Crash Matrix | ✅ PASS | RC7 |
| G9 | Upgrade v3.8→v3.9 | ✅ PASS | RC7 |
| G10 | Audit + Time Travel | ✅ PASS | RC7 |
| G11 | QPS/TPS Benchmark | ✅ PASS | RC7 |
| G13 | 24h Stability | ✅ PASS | RC7 |
| G15 | TPC-H SF=0.01 wire | ✅ PASS | RC7 |
| G16 | Compatibility v3.8→v3.9 | ✅ PASS | RC7 |

### Soak 状态（GA 最终阻断项）

| Soak | 目标 | 主机 | 状态 |
|------|------|------|------|
| 24h simulated | GA blocker | — | ✅ PASS (RC7) |
| 24h real | GA blocker | Z6G4 (250 backup) | 🟡 运行中（843 samples, 0 errors） |
| 72h real | GA-final | Z6G4 | ⏳ 阻塞（Z6G4 网络不可达） |
| 168h real | GA-final | Z6G4 | ⏳ 阻塞（Z6G4 网络不可达） |

### 本次会话完成的修复（2026-06-25）

| 修复 | Commit | 测试 |
|------|--------|------|
| PredicateCompiler Column bug | `d87801e43` | 4 tests un-ignore → PASS |
| Boundary INT64_MIN parsing | `9e2806278` | 2 tests un-ignore → PASS |
| Parser Token::Star warning | `ff77472bf` | 0 tests（warning fix） |

### ignore 测试修复状态

| 修复 | 数量 | 总 ignore |
|------|------|----------|
| ✅ PredicateCompiler (4 bugs) | 4 | 50 → 46 |
| ✅ Boundary test (INT64_MIN, div-zero) | 2 | 46 → 44 |
| 剩余 KNOWN_BUG | 3 | — |
| 剩余 KNOWN_GAP | 18 | — |
| 剩余 PERF_BENCHMARK | 17 | — |
| 剩余 SOAK | 5 | — |
| 剩余 MANUAL_ORACLE | 1 | — |

---

## 1. 核心战略

> **核心问题反转**: 不是"支持多少 SQL", 而是"数据库死了以后还能不能回来"。

- 资源分配: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / **新 SQL 0%**
- 16 任务 / 451h / 6 Phase
- 新门禁 G1-G16 (取代 L1-L6)
- 详细计划: `plans/V390_VERSION_PLAN.md`

---

## 2. 目录结构

```
v3.9.0/
├── README.md                          # 本文件 - 文档索引（已更新 2026-06-25）
├── CHANGELOG.md                       # v3.9.0 变更日志
├── ROADMAP.md                         # 6 Phase 路线图
├── GA_GATE_REPORT.md                  # RC7 GA 门禁报告
├── GA_GATE_STATUS_REPORT.md          # GA 门禁状态
├── IGNORE_REGISTRY_2026-06-25.md     # ignore 测试清单（已更新 44个）
│
├── alpha/                             # Alpha 阶段文档
├── beta/                              # Beta 阶段文档
├── rc/                                 # RC 阶段文档
├── evidence/                           # GA 阶段证据
├── perf/                              # 性能基线
│
└── plans/                             # 计划文档
    ├── V390_VERSION_PLAN.md           # 战略定位
    ├── V390_DEVELOPMENT_PLAN.md     # P0/P1/P2/P3 详细任务
    └── V390_TEST_PLAN.md            # G1-G10 门禁 + 测试场景
```

---

## 3. 6 Phase 路线图 (12 周)

| Phase | 周 | 主题 | 关键任务 | 门禁 | 状态 |
|-------|----|------|----------|------|------|
| 0 | W0 | 分支 + SPEC | V390 plan 落地 | — | ✅ |
| 1 | W1-2 | ARCH-3 + INT-3 | VTU 主路径 + expr 合并 | G3 G4 | ✅ |
| 2 | W3-4 | INT-2 + Savepoint | TransactionManager 集成 | G2 G5 | ✅ |
| 3 | W5-6 | Backup/Restore + Crash Matrix | 备份/恢复 100+ 场景 | G6 G8 | ✅ |
| 4 | W7-8 | Soak + Upgrade | 24h 浸泡 + 升级 | G7 G9 | ✅ |
| 5 | W9-10 | Audit + Time Travel | 40+ 审计测试 | G10 | ✅ |
| 6 | W11-12 | 性能优化 + GA 收口 | 性能调优 | G1 | ✅ |

---

## 4. G1-G16 门禁

| # | 门禁 | 替换 | 状态 |
|---|------|------|------|
| G1 | 22/22 TPC-H 保持 | L3 | ✅ PASS |
| G2 | INT-2 ParallelExecutor 集成 | L2 | ✅ PASS |
| G3 | INT-3 Expression 合并 | L2 | ✅ PASS |
| G4 | ARCH-3 VtuGuard 完整 | L1 | ✅ PASS |
| G5 | SEM-1 Savepoint MVCC | L5 | ✅ PASS |
| G6 | Backup/Restore 100+ 场景 | L4 | ✅ PASS |
| G7 | 24h Soak (simulated) | L4 | ✅ PASS |
| G8 | Crash Matrix 100+ 场景 | L4 | ✅ PASS |
| G9 | Upgrade v3.8→v3.9 | L4 | ✅ PASS |
| G10 | Audit + Time Travel | L4 | ✅ PASS |
| G11 | QPS/TPS Benchmark | — | ✅ PASS |
| G13 | 24h Stability | — | ✅ PASS |
| G15 | TPC-H SF=0.01 wire | — | ✅ PASS |
| G16 | Compatibility v3.8→v3.9 | — | ✅ PASS |

---

## 5. v3.10.0 后继计划

v3.10.0 开发计划已移至 `docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md`。

v3.10.0 定位: **MySQL 5.7 替代** — 功能稳定 + 基本性能优先

| 类别 | 内容 |
|------|------|
| C-1~C-5 核心MySQL兼容 | DML完整性 / UNION集合 / ACID正确性 / ALTER TABLE / 崩溃恢复 |
| H-1~H-5 历史债务 | ARCH-2双路径 / ARCH-3 VTU剩余5% / GIS+SEQUENCE+列权限 |
| M/L 推迟项 | CBO代价模型 / 覆盖率标准化 / Cypher图查询扩展 |

---

## 6. 关联资源

- **Milestone**: http://192.168.0.250:3000/openclaw/sqlrustgo/milestones/32 (v3.9.0)
- **前置版本**: v3.8.0 (`docs/releases/v3.8.0/`)
- **v3.10.0**: `docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md`
- **GA 治理报告**: `docs/governance/GA_GOVERNANCE_DEMO_v3.8.0.md`
- **优化报告**: `docs/governance/GA_OPTIMIZATION_REPORT_2026-06-05.md`

---

## 7. 维护信息

| 项目 | 值 |
|------|-----|
| 文档版本 | v3.9.0-INDEX-2.0 |
| 创建日期 | 2026-06-05 |
| 最后更新 | 2026-06-25 |
| 维护人 | Claude Code |
| 状态 | **RC7** — GA 待 Z6G4 恢复 |

---

*本索引遵循 `docs/governance/DOC_CHECK_CORRECTION_RULES.md` 5 步流程创建。*
