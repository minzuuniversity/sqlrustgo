# v3.9.0 文档索引

<!-- env:blocked:no-ci -->

> **版本**: v3.9.0
> **类型**: **Production Readiness Release** (工程化版本, 非功能版本)
> **主题**: Single-Node Production Candidate
> **分支**: `develop/v3.9.0` (从 `main@v3.8.0` fork)
> **创建日期**: 2026-06-05
> **GA 目标**: 2026-09-23 (12 周 / 6 Phase)

---

## 核心战略

> **核心问题反转**: 不是"支持多少 SQL", 而是"数据库死了以后还能不能回来"。

- 资源分配: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / **新 SQL 0%**
- 16 任务 / 451h / 6 Phase
- 新门禁 G1-G10 (取代 L1-L6)
- 详细计划: `plans/V390_VERSION_PLAN.md`

---

## 目录结构

```
v3.9.0/
├── README.md                          # 本文件 - 文档索引
├── CHANGELOG.md                       # v3.9.0 变更日志
├── ROADMAP.md                         # 6 Phase 路线图
│
├── alpha/                             # Alpha 阶段文档 (Phase 1-2)
│   └── (待创建)
├── beta/                              # Beta 阶段文档 (Phase 3-4)
│   └── (待创建)
├── rc/                                # RC 阶段文档 (Phase 5)
│   └── (待创建)
├── ga/                                # GA 阶段文档 (Phase 6)
│   └── (待创建)
│
├── debt/                              # 债务跟踪
│   └── (从 v3.8.0 迁移)
│
└── plans/                             # 计划文档
    ├── V390_VERSION_PLAN.md           # 战略定位
    ├── V390_DEVELOPMENT_PLAN.md       # P0/P1/P2/P3 详细任务
    └── V390_TEST_PLAN.md              # G1-G10 门禁 + 测试场景
```

---

## 6 Phase 路线图 (12 周)

| Phase | 周 | 主题 | 关键任务 | 门禁 |
|-------|----|----|----------|------|
| 0 | W0 | 分支 + SPEC | V390 plan 落地 + 启动 commit | — |
| 1 | W1-2 | ARCH-3 + INT-3 | VTU 主路径 + expr 完整合并 | G3 INT-3, G4 ARCH-3 |
| 2 | W3-4 | INT-2 + Savepoint | TransactionManager 集成 + Savepoint 完整 | G2 INT-2, G5 SEM-1 |
| 3 | W5-6 | Backup/Restore + Crash Matrix | 备份/恢复 100+ 场景 | G6 Backup/Restore |
| 4 | W7-8 | Soak + Upgrade | 24h 浸泡 + 50+ 升级 | G7 24h Soak, G8 Crash Matrix |
| 5 | W9-10 | Audit + Time Travel | 40+ 审计测试 | G9 Upgrade, G10 Audit+Time Travel |
| 6 | W11-12 | 性能优化 + GA 收口 | 性能调优 + GA 报告 | G1 22/22 TPC-H 保持 |

---

## G1-G10 新门禁

| # | 门禁 | 替换 |
|---|------|------|
| G1 | 22/22 TPC-H 保持 | L3 |
| G2 | INT-2 关闭 | L2 |
| G3 | INT-3 关闭 | L2 |
| G4 | ARCH-3 关闭 | L1 |
| G5 | SEM-1 关闭 | L5 |
| G6 | Backup/Restore 100+ 场景 | L4 |
| G7 | 24h Soak Test | L4 |
| G8 | Crash Matrix 100+ 场景 | L4 |
| G9 | Upgrade Test 50+ 场景 | L4 |
| G10 | Audit + Time Travel 40+ tests | L4 |

---

## 关联资源

- **Milestone**: http://192.168.0.252:3000/openclaw/sqlrustgo/milestones/32 (v3.9.0, 0/0 issues)
- **前置版本**: v3.8.0 (`docs/releases/v3.8.0/`)
- **GA 治理报告**: `docs/governance/GA_GOVERNANCE_DEMO_v3.8.0.md`
- **优化报告**: `docs/governance/GA_OPTIMIZATION_REPORT_2026-06-05.md`

---

## 维护信息

| 项目 | 值 |
|------|-----|
| 文档版本 | v3.9.0-INDEX-1.0 |
| 创建日期 | 2026-06-05 |
| 维护人 | Hermes Agent |
| 状态 | ACTIVE (Phase 0) |
| 下次审查 | Phase 1 启动前 (W1) |

---

*本索引遵循 `docs/governance/DOC_CHECK_CORRECTION_RULES.md` 5 步流程创建。*
