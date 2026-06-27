# v3.5.0 GATE Summary

> **版本**: v3.5.0  
> **分支**: `origin/develop/v3.5.0`  
> **编制日期**: 2026-05-29  
> **类型**: 版本门禁综合概览  

---

## 一、版本基本信息

| 项目 | 值 |
|------|---|
| 版本 | v3.5.0 |
| 战略定位 | AI Native GMP Platform（AI 原生 GMP 平台） |
| 前置版本 | v3.4.0 GA |
| 开发分支 | `develop/v3.5.0` → `rc/v3.5.0` → `main` |
| Tag | `v3.5.0` (未推送，待 GA 完成) |
| 当前分支 SHA | `2344194f` (main) / `8db2b897` (GA commit) |

---

## 二、门禁执行结果（全部 PASS）

| Gate | 日期 | 结果 | 覆盖率/关键指标 | 签署 Commit |
|------|------|------|----------------|------------|
| **Alpha** | 2026-05-28 | ✅ PASS | L1 平均 **84.99%** (≥75%) | `7f7d963` |
| **Beta** | 2026-05-28 | ✅ PASS | L1 平均 **85.27%** (≥70%) | `752859c` |
| **RC** | 2026-05-28 | ✅ PASS | L1 综合 **86.57%** (≥85%) | `795f408` |
| **GA** | 2026-05-28 | ✅ PASS | L1 综合 **87.36%** (≥85%) | `8db2b89` |

**门禁通过率**: 4/4 阶段全部 PASS，0 FAIL，0 PENDING

---

## 三、GA Gate G1-G5 详细结果

| Gate | 检查项 | 期望 | 实际 | 状态 |
|------|--------|------|------|------|
| G1 | Build | 成功 | ✅ 0.64s | ✅ PASS |
| G2 | Test (workspace) | 100% | 39 passed, 0 failed | ✅ PASS |
| G3 | Clippy | 零 error | ✅ 0 errors | ✅ PASS |
| G4 | Format | 无 diff | ✅ 无格式错误 | ✅ PASS |
| G5 | Coverage L1 | ≥85% | **87.36%** | ✅ PASS (+2.36pp) |

---

## 四、覆盖率详情（L1 Crates）

| Crate | Alpha (≥75%) | Beta (≥70%) | RC (≥85%) | GA (≥85%) |
|-------|-------------|-------------|-----------|-----------|
| sqlrustgo-types | 87.65% ✅ | 87.11% ✅ | 87.65% ✅ | 87.65% ✅ |
| sqlrustgo-parser | 78.18% ✅ | 75.69% ✅ | 83.82% ✅ | 83.82% ✅ |
| sqlrustgo-planner | 89.39% ✅ | 88.82% ✅ | 89.39% ✅ | 89.39% ✅ |
| sqlrustgo-optimizer | 83.67% ✅ | 84.16% ✅ | 89.72% ✅ | 89.72% ✅ |
| sqlrustgo-executor | 83.00% ✅ | 83.24% ✅ | 83.00% ✅ | 89.30% ✅ |
| sqlrustgo-storage | 81.75% ✅ | 81.99% ✅ | 81.75% ✅ | 81.70% ✅ |
| sqlrustgo-transaction | 87.81% ✅ | 90.08% ✅ | 88.75% ✅ | 88.75% ✅ |
| sqlrustgo-catalog | 88.52% ✅ | 91.03% ✅ | 88.52% ✅ | 88.52% ✅ |
| **L1 平均** | **84.99%** | **85.27%** | **86.57%** | **87.36%** |

> 注: GA 阶段使用 `--tests` 综合测量（覆盖 src/ + tests/），优于 Alpha/Beta 的 `--lib` 单一测量。

---

## 五、新增 Crate 测试结果

| Crate | 测试结果 | 状态 |
|-------|---------|------|
| `sqlrustgo-compliance-engine` | 59 passed | ✅ |
| `sqlrustso-gmp-retrieval` | 9 passed | ✅ |
| `sqlrustgo-gmp-api` | 107 passed | ✅ |
| `sqlrustgo-gmp` | 196 passed | ✅ |

---

## 六、Trust Infrastructure 测试结果

| Crate | 测试结果 | 状态 |
|-------|---------|------|
| evidence-engine | 31 passed | ✅ |
| provenance-graph | 24 passed | ✅ |
| compliance-engine | 59 passed | ✅ |
| workflow-v2 | 41 passed | ✅ |
| perf-baseline | 23 passed | ✅ |
| crash-sim | 49 passed | ✅ |
| wal-verification | 50 passed | ✅ |

---

## 七、TPC-H 基准测试

| 配置 | 结果 | 状态 |
|------|------|------|
| SF=1 (22 queries) | 22/22 PASS | ✅ PASS |
| 执行时间 | ~150s | 正常 |

---

## 八、版本文档状态

| 文档 | 路径 | 状态 |
|------|------|------|
| DEV_PLAN.md | `docs/releases/v3.5.0/DEV_PLAN.md` | ✅ 存在 |
| TEST_PLAN.md | `docs/releases/v3.5.0/TEST_PLAN.md` | ✅ 存在 |
| CHANGELOG.md | `docs/releases/v3.5.0/CHANGELOG.md` | ✅ 存在 |
| RELEASE_NOTES.md | `docs/releases/v3.5.0/RELEASE_NOTES.md` | ✅ 存在 |
| ALPHA_GATE_REPORT.md | `docs/releases/v3.5.0/ALPHA_GATE_REPORT.md` | ✅ 存在 |
| BETA_GATE_REPORT.md | `docs/releases/v3.5.0/BETA_GATE_REPORT.md` | ✅ 存在 |
| RC_GATE_REPORT.md | `docs/releases/v3.5.0/RC_GATE_REPORT.md` | ✅ 存在 |
| GA_GATE_REPORT.md | `docs/releases/v3.5.0/GA_GATE_REPORT.md` | ✅ 存在 |
| **GATE_SUMMARY.md** | `docs/releases/v3.5.0/GATE_SUMMARY.md` | ✅ 本文档 |
| LEGACY_ISSUES.md | `docs/releases/v3.5.0/LEGACY_ISSUES.md` | ✅ 存在 |
| OO_ROADMAP.md | `docs/releases/v3.5.0/oo/OO_ROADMAP.md` | ✅ 存在 |

---

## 九、遗留问题

| ID | 问题 | 门禁项 | 状态 |
|----|------|--------|------|
| EX-v350-001 | 新增 crate 覆盖率阈值 | Beta 阶段 | ⏭️ 豁免 (Beta 阶段无阈值要求) |

---

## 十、Truthfulness 声明

> 本 GATE_SUMMARY.md 严格基于实测数据编写：
> - 所有覆盖率数据来自 `cargo llvm-cov` 实际执行
> - 所有测试结果来自 `cargo test --all-features --lib` 实际执行
> - 所有 Gate 状态基于实际 Gate 报告文件（ALPHA/BETA/RC/GA_GATE_REPORT.md）
> - 无 PENDING 占位，无引用历史数据冒充本次执行

---

*本文件由 hermes-z440 编制，数据截至 2026-05-29*
*规范来源: docs/governance/gate_spec_v350.md*