# SQLRustGo v3.9.0 综合索引

> **版本**: v3.9.0
> **状态**: RC7 (2026-06-12)
> **GA 目标**: 2026-12-15
> **分支**: `develop/v3.9.0`
> **最后更新**: 2026-06-17

---

## 一、版本概览

| 属性 | 值 |
|------|-----|
| 当前版本 | v3.9.0 |
| 阶段 | RC7 |
| 分支 | develop/v3.9.0 |
| 前置版本 | v3.8.0 GA (2026-06-08) |
| GA 目标 | 2026-12-15 |
| 主题 | Single-Node Production Candidate |

### 测试结果 — 诚实声明

| 测试集 | 结果 | 限制说明 |
|--------|------|----------|
| Lib Tests | 1670 PASS, 1 IGNORED | ✅ 已执行 |
| TPC-H | 22/22 PASS | ⚠️ 无 oracle 对比 |
| Corpus | 818/818 PASS | ⚠️ 无 oracle 对比 |
| D9 Gate | 8/8 PASS | ✅ 有独立验证 |

### G1-G16 门禁状态 — 诚实声明

| Gate | 主题 | 状态 | 限制说明 |
|------|------|------|----------|
| G1 | TPC-H 22/22 | ✅ PASS | ⚠️ 无 oracle 对比 |
| G2 | INT-2 | ✅ PASS | ⚠️ 无 oracle 对比 |
| G3 | INT-3 | ✅ PASS | ⚠️ 无 oracle 对比 |
| G4 | ARCH-3 | ✅ PASS | ✅ 有独立验证 |
| G5 | SEM-1 | ✅ PASS | ⚠️ 无 oracle 对比 |
| G6 | Backup/Restore | ✅ PASS | ✅ 有独立验证 |
| G7 | 24h Soak | ✅ PASS | ⚠️ **SIMULATED**，非真实 24h |
| G8 | Crash Matrix | ✅ PASS | ✅ 有独立验证 |
| G9 | Upgrade Test | ✅ PASS | ⚠️ 无 oracle 对比 |
| G10 | Audit + Time Travel | ✅ PASS | ✅ 有独立验证 |
| G11 | QPS/TPS | ✅ PASS | ⚠️ 无 oracle 对比 |
| G12 | Sysbench | ✅ PASS | ⚠️ 无 oracle 对比 |
| G13 | Stability | 🟡 进行中 | 真实 24h soak 未完成 |
| G14 | Real Crash | ✅ PASS | ⚠️ 部分模拟 |
| G15 | TPC-H SF0.01 | ✅ PASS | ⚠️ 无 oracle 对比 |
| G16 | Compatibility | ✅ PASS | ⚠️ 无 oracle 对比 |

**诚实声明**: 所有 gate 脚本已执行，但 11/16 缺乏独立 oracle 对比。真实 soak 测试尚未完成。

---

## 二、文档目录结构

```
v3.9.0/
├── INDEX.md                              # 本文件 - 综合索引
├── README.md                             # 文档入口
├── CHANGELOG.md                          # 变更日志
├── ROADMAP.md                            # 6 Phase 路线图
├── RELEASE_NOTES.md                       # 发布说明
├── GA_GATE_REPORT.md                      # GA 门禁报告 (最新)
├── GA_GATE_STATUS_REPORT.md              # GA 门禁状态
├── FEATURE_MATRIX.md                     # 功能矩阵
├── QUICK_START.md                        # 快速开始
├── INSTALL.md                            # 安装指南
├── DEPLOYMENT_GUIDE.md                   # 部署指南
├── MIGRATION_GUIDE.md                    # 迁移指南
│
├── alpha/                                # Alpha 阶段 (Phase 0-1)
│   └── ALPHA1_RELEASE_NOTES.md
├── beta/                                 # Beta 阶段 (Phase 2-3)
│   ├── BETA_RELEASE_NOTES.md
│   └── SOAK_72H_REPORT.md
├── rc/                                   # RC 阶段 (Phase 4-5)
│   ├── RC1_RELEASE_NOTES.md
│   ├── RC1_GATE_REPORT.md
│   ├── RC2_RELEASE_NOTES.md
│   ├── RC2_GATE_REPORT.md
│   ├── RC3_PLAN.md
│   ├── RC3_RELEASE_NOTES.md
│   ├── RC3_GATE_REPORT.md
│   ├── RC4_RELEASE_NOTES.md
│   ├── RC4_GATE_REPORT.md
│   ├── RC5_RELEASE_NOTES.md
│   ├── RC5_GATE_REPORT.md
│   ├── RC6_RELEASE_NOTES.md
│   ├── RC6_GATE_REPORT.md
│   ├── RC7_RELEASE_NOTES.md
│   └── RC7_GATE_REPORT.md
├── evidence/                              # GA 证据 (Phase 6)
│   ├── 00-release-summary.md
│   ├── 01-release-notes.md
│   ├── 02-scope-definition.md
│   ├── 03-test-report.md
│   ├── 04-coverage-report.md
│   ├── 05-security-scan-report.md
│   ├── 06-performance-report.md
│   ├── 07-dependency-audit.md
│   ├── 08-license-compliance.md
│   ├── 09-ci-build-log.md
│   └── 10-approval-record.md
├── perf/                                 # 性能文档
│   ├── PERFORMANCE_BASELINE.md
│   ├── PERFORMANCE_BASELINE_REAL_2026-06-12.md
│   ├── PERFORMANCE_BASELINE_QPS_20260612.md
│   ├── PERFORMANCE_OVERVIEW_20260612.md
│   ├── TPC_H_SHA256_BASELINE_20260612.md
│   ├── SYSBENCH_MARIADB_COMPARISON_20260612.md
│   ├── COMPATIBILITY_REPORT.md
│   ├── CRASH_TEST_REPORT.md
│   ├── PERFORMANCE_REPORT.md
│   ├── QPS_REPORT.md
│   ├── STABILITY_REPORT.md
│   ├── SYSBENCH_REPORT.md
│   └── FOUR_WAY_TPCH_REPORT.md
├── plans/                                # 计划文档
│   ├── V390_VERSION_PLAN.md              # 战略定位
│   ├── V390_DEVELOPMENT_PLAN.md          # P0/P1/P2/P3 详细任务
│   ├── V390_TEST_PLAN.md                 # G1-G10 门禁 + 测试场景
│   ├── V390_TEST_PLAN_SUPPLEMENT_PERF.md
│   ├── V390_TEST_PLAN_ROUND2_REVIEW.md
│   ├── TPCH_ORACLE_PLAN.md
│   └── SPRINT4_MASTER_PLAN.md
└── incidents/                            # 事故报告
    └── GITEA_252_OUTAGE_20260607.md
```

---

## 三、快速导航

### 3.1 核心文档

| 文档 | 说明 |
|------|------|
| [README.md](README.md) | 文档入口 + 6 Phase 路线图 |
| [CHANGELOG.md](CHANGELOG.md) | 完整变更日志 |
| [ROADMAP.md](ROADMAP.md) | 详细路线图 (W0-W12) |
| [GA_GATE_REPORT.md](GA_GATE_REPORT.md) | **GA 门禁报告** (最新状态) |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | 发布说明 |

### 3.2 用户文档

| 文档 | 说明 |
|------|------|
| [QUICK_START.md](QUICK_START.md) | 快速开始指南 |
| [INSTALL.md](INSTALL.md) | 安装指南 |
| [DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) | 部署指南 |
| [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) | 从旧版本迁移指南 |

### 3.3 阶段文档

#### Alpha 阶段
| 文档 | 说明 |
|------|------|
| [alpha/ALPHA1_RELEASE_NOTES.md](alpha/ALPHA1_RELEASE_NOTES.md) | Alpha1 发布笔记 |

#### Beta 阶段
| 文档 | 说明 |
|------|------|
| [beta/BETA_RELEASE_NOTES.md](beta/BETA_RELEASE_NOTES.md) | Beta 发布笔记 |
| [beta/SOAK_72H_REPORT.md](beta/SOAK_72H_REPORT.md) | 72h 浸泡报告 |

#### RC 阶段
| 文档 | 说明 |
|------|------|
| [rc/RC1_RELEASE_NOTES.md](rc/RC1_RELEASE_NOTES.md) | RC1 发布笔记 |
| [rc/RC1_GATE_REPORT.md](rc/RC1_GATE_REPORT.md) | RC1 门禁报告 |
| [rc/RC2_RELEASE_NOTES.md](rc/RC2_RELEASE_NOTES.md) | RC2 发布笔记 |
| [rc/RC2_GATE_REPORT.md](rc/RC2_GATE_REPORT.md) | RC2 门禁报告 |
| [rc/RC3_PLAN.md](rc/RC3_PLAN.md) | RC3 计划 |
| [rc/RC3_RELEASE_NOTES.md](rc/RC3_RELEASE_NOTES.md) | RC3 发布笔记 |
| [rc/RC3_GATE_REPORT.md](rc/RC3_GATE_REPORT.md) | RC3 门禁报告 |
| [rc/RC4_RELEASE_NOTES.md](rc/RC4_RELEASE_NOTES.md) | RC4 发布笔记 |
| [rc/RC4_GATE_REPORT.md](rc/RC4_GATE_REPORT.md) | RC4 门禁报告 |
| [rc/RC5_RELEASE_NOTES.md](rc/RC5_RELEASE_NOTES.md) | RC5 发布笔记 |
| [rc/RC5_GATE_REPORT.md](rc/RC5_GATE_REPORT.md) | RC5 门禁报告 |
| [rc/RC6_RELEASE_NOTES.md](rc/RC6_RELEASE_NOTES.md) | RC6 发布笔记 |
| [rc/RC6_GATE_REPORT.md](rc/RC6_GATE_REPORT.md) | RC6 门禁报告 |
| [rc/RC7_RELEASE_NOTES.md](rc/RC7_RELEASE_NOTES.md) | RC7 发布笔记 |
| [rc/RC7_GATE_REPORT.md](rc/RC7_GATE_REPORT.md) | RC7 门禁报告 |

### 3.4 证据文档 (GA)

| 文档 | 说明 |
|------|------|
| [evidence/00-release-summary.md](evidence/00-release-summary.md) | 发布摘要 |
| [evidence/01-release-notes.md](evidence/01-release-notes.md) | 发布笔记 |
| [evidence/02-scope-definition.md](evidence/02-scope-definition.md) | 范围定义 |
| [evidence/03-test-report.md](evidence/03-test-report.md) | 测试报告 |
| [evidence/04-coverage-report.md](evidence/04-coverage-report.md) | 覆盖率报告 |
| [evidence/05-security-scan-report.md](evidence/05-security-scan-report.md) | 安全扫描报告 |
| [evidence/06-performance-report.md](evidence/06-performance-report.md) | 性能报告 |
| [evidence/07-dependency-audit.md](evidence/07-dependency-audit.md) | 依赖审计 |
| [evidence/08-license-compliance.md](evidence/08-license-compliance.md) | 许可证合规 |
| [evidence/09-ci-build-log.md](evidence/09-ci-build-log.md) | CI 构建日志 |
| [evidence/10-approval-record.md](evidence/10-approval-record.md) | 审批记录 |

### 3.5 性能文档

| 文档 | 说明 |
|------|------|
| [perf/PERFORMANCE_BASELINE.md](perf/PERFORMANCE_BASELINE.md) | 性能基线 |
| [perf/PERFORMANCE_BASELINE_REAL_2026-06-12.md](perf/PERFORMANCE_BASELINE_REAL_2026-06-12.md) | 真实性能基线 (2026-06-12) |
| [perf/PERFORMANCE_BASELINE_QPS_20260612.md](perf/PERFORMANCE_BASELINE_QPS_20260612.md) | QPS 基线 |
| [perf/PERFORMANCE_OVERVIEW_20260612.md](perf/PERFORMANCE_OVERVIEW_20260612.md) | 性能概览 |
| [perf/TPC_H_SHA256_BASELINE_20260612.md](perf/TPC_H_SHA256_BASELINE_20260612.md) | TPC-H SHA256 基线 |
| [perf/SYSBENCH_MARIADB_COMPARISON_20260612.md](perf/SYSBENCH_MARIADB_COMPARISON_20260612.md) | 与 MariaDB 对比 |
| [perf/COMPATIBILITY_REPORT.md](perf/COMPATIBILITY_REPORT.md) | 兼容性报告 |
| [perf/CRASH_TEST_REPORT.md](perf/CRASH_TEST_REPORT.md) | 崩溃测试报告 |
| [perf/QPS_REPORT.md](perf/QPS_REPORT.md) | QPS 报告 |
| [perf/STABILITY_REPORT.md](perf/STABILITY_REPORT.md) | 稳定性报告 |
| [perf/FOUR_WAY_TPCH_REPORT.md](perf/FOUR_WAY_TPCH_REPORT.md) | 四路 TPC-H 报告 |

### 3.6 计划文档

| 文档 | 说明 |
|------|------|
| [plans/V390_VERSION_PLAN.md](plans/V390_VERSION_PLAN.md) | 版本计划 (战略) |
| [plans/V390_DEVELOPMENT_PLAN.md](plans/V390_DEVELOPMENT_PLAN.md) | 开发计划 (任务) |
| [plans/V390_TEST_PLAN.md](plans/V390_TEST_PLAN.md) | 测试计划 |
| [plans/V390_TEST_PLAN_SUPPLEMENT_PERF.md](plans/V390_TEST_PLAN_SUPPLEMENT_PERF.md) | 测试计划补充 (性能) |
| [plans/V390_TEST_PLAN_ROUND2_REVIEW.md](plans/V390_TEST_PLAN_ROUND2_REVIEW.md) | 测试计划第二轮评审 |
| [plans/TPCH_ORACLE_PLAN.md](plans/TPCH_ORACLE_PLAN.md) | TPC-H Oracle 计划 |
| [plans/SPRINT4_MASTER_PLAN.md](plans/SPRINT4_MASTER_PLAN.md) | Sprint 4 主计划 |

---

## 四、版本历史追溯

| 版本 | 阶段 | 日期 | 关键产物 |
|------|------|------|----------|
| v3.9.0 | RC7 | 2026-06-12 | 当前版本，性能文档 + MariaDB 对比 |
| v3.9.0 | RC6 | 2026-06-12 | INT-2/INT-3 实质性测试 (30/30 PASS) |
| v3.9.0 | RC5 | 2026-06-12 | 跨版本升级链 + IS NULL pushdown |
| v3.9.0 | RC4 | 2026-06-12 | G1/G7/G8/G9/G13 PASS, QPS 基线 |
| v3.9.0 | RC3 | 2026-06-12 | G1-G16 全部 PASS |
| v3.9.0 | RC1/RC2 | 2026-06-05 | 表单验证里程碑 |
| v3.9.0 | Beta | 2026-06-05 | 表单验证里程碑 |
| v3.9.0 | Alpha1 | 2026-06-05 | 入口基线 |
| v3.8.0 | GA | 2026-06-08 | 架构统一 |

---

## 五、关键链接

| 资源 | 链接 |
|------|------|
| Gitea 仓库 | http://192.168.0.252:3000/openclaw/sqlrustgo |
| Milestone v3.9.0 | http://192.168.0.252:3000/openclaw/sqlrustgo/milestones/32 |
| 分支 | `develop/v3.9.0` |
| 前置版本 | [v3.8.0](../v3.8.0/README.md) |
| 顶层文档 | [docs/README.md](../../README.md) |

---

## 六、维护信息

| 项目 | 值 |
|------|-----|
| 索引版本 | v3.9.0-INDEX-3.0 |
| 创建日期 | 2026-06-05 |
| 最后更新 | 2026-06-17 |
| 更新内容 | 添加 RC4-RC7 文档 |
| 维护人 | Hermes Agent |
| 状态 | ACTIVE |

---

*本索引由 Hermes Agent 维护*
*更新频率: 每个 RC 版本发布后更新*
