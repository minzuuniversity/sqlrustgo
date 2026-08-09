# v3.11.0 文档索引

> **状态**: GA Release (2026-08-09)
> **SSOT**: 本文件是 v3.11.0 文档结构入口；版本状态和可信度边界以 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 为准。
> **说明**: 原英文索引保留在附录，当前中文主文用于日常查阅。

## 1. 活跃文档

### 1.1 版本管理

| 文档 | 路径 | 状态 | 说明 |
|---|---|---|---|
| VERSION_PLAN.md | `VERSION_PLAN.md` | Current | 版本战略摘要 |
| STAGE.yaml | `STAGE.yaml` | Current | 阶段状态、gate、threshold |
| CHANGELOG.md | `CHANGELOG.md` | Current | 发布变更记录 |
| COMPREHENSIVE_ASSESSMENT_REPORT.md | `COMPREHENSIVE_ASSESSMENT_REPORT.md` | Current | v3.11.0 GA 综合评估和证据边界 |

### 1.2 plans/ 目录

| 文档 | 路径 | 状态 | 说明 |
|---|---|---|---|
| V311_VERSION_PLAN.md | `plans/V311_VERSION_PLAN.md` | Current | 详细战略计划 |
| V311_DEVELOPMENT_PLAN.md | `plans/V311_DEVELOPMENT_PLAN.md` | Current | V311 任务拆解 |
| V311_DEBT_CLOSURE_PLAN.md | `plans/V311_DEBT_CLOSURE_PLAN.md` | Current | 债务清理计划 |
| V311_DOCS_RESTRUCTURE_PLAN.md | `plans/V311_DOCS_RESTRUCTURE_PLAN.md` | Current | 文档重构计划 |
| V311_ISSUE_CROSSREF.md | `plans/V311_ISSUE_CROSSREF.md` | Current | 任务与 Gitea issue 映射 |
| INDEX.md | `plans/INDEX.md` | Current | plans 目录索引 |

### 1.3 报告与分析

| 文档 | 路径 | 状态 | 说明 |
|---|---|---|---|
| PROGRESS.md | `PROGRESS.md` | Current | 任务进度追踪 |
| LEGACY_DEBT_TRACKING_TABLE.md | `LEGACY_DEBT_TRACKING_TABLE.md` | Current | 债务状态快照 |
| LEGACY_DEBT_AUDIT_REPORT.md | `LEGACY_DEBT_AUDIT_REPORT.md` | Archived | 历史审计，需结合综合评估阅读 |
| FEATURE_CHECKLIST.md | `FEATURE_CHECKLIST.md` | Current/历史混合 | 功能清单，部分历史统计需交叉核对 |
| GOVERNANCE_TRUTH_AUDIT.md | `GOVERNANCE_TRUTH_AUDIT.md` | Current | 治理真实性审计 |
| AUDIT_V311_REALITY_CHECK.md | `AUDIT_V311_REALITY_CHECK.md` | Current | v3.11 现实状态核查 |

### 1.4 性能、覆盖率和测试文档

| 文档 | 路径 | 状态 | 说明 |
|---|---|---|---|
| TPCH_SF1_22_22_PASS_REPORT.md | `TPCH_SF1_22_22_PASS_REPORT.md` | Current | TPC-H SF=1 22/22 可运行性报告 |
| TPCH_SF1_VERIFICATION_REPORT.md | `TPCH_SF1_VERIFICATION_REPORT.md` | Current | TPC-H 验证和后续 correctness 跟踪 |
| G4_WIRE_TEST_CLOSE_OUT_PLAN.md | `G4_WIRE_TEST_CLOSE_OUT_PLAN.md` | Current | Wire protocol close-out 计划 |
| COVERAGE_TESTING_METHODOLOGY.md | `COVERAGE_TESTING_METHODOLOGY.md` | Current | 覆盖率测量方法 |
| G3_COVERAGE_REMEDIATION_PLAN.md | `G3_COVERAGE_REMEDIATION_PLAN.md` | Current | G3 覆盖率整改计划 |
| E2E_TESTING_GUIDE.md | `E2E_TESTING_GUIDE.md` | Current | E2E 测试开发指南 |
| DISABLED_TESTS_ANALYSIS.md | `DISABLED_TESTS_ANALYSIS.md` | Current | disabled tests 分析 |
| REGRESSION_TEST_SUITE.md | `REGRESSION_TEST_SUITE.md` | Current | 回归测试套件说明 |

### 1.5 perf/ 目录

| 文档 | 路径 | 状态 | 说明 |
|---|---|---|---|
| SF1_BASELINE_REPORT.md | `perf/SF1_BASELINE_REPORT.md` | Current | SF=1 baseline |
| SF01_CROSS_ENGINE_BASELINE.md | `perf/SF01_CROSS_ENGINE_BASELINE.md` | Current | 跨引擎对比 baseline |
| TPCH_QExecution_Analysis.md | `perf/TPCH_QExecution_Analysis.md` | Current | query execution 分析 |
| Q5_MEMORY_ANALYSIS.md | `perf/Q5_MEMORY_ANALYSIS.md` | Current | Q5 memory usage 分析 |
| HASH_ANTI_JOIN_PERF.md | `perf/HASH_ANTI_JOIN_PERF.md` | Current | Hash Anti Join 性能分析 |

## 2. 使用规则

- 当前 GA 判断优先读取 `COMPREHENSIVE_ASSESSMENT_REPORT.md`。
- 单项 gate 结论必须读取对应报告和命令输出，不得只凭索引状态判断 PASS。
- 英文附录仅用于历史追溯，不替代中文主文。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# v3.11.0 Documentation Index

> **Status**: **GA Release (2026-08-09)** — commit `83c623835`, tag `v3.11.0-ga` on 5 remotes
> **SSOT**: This is the **single source of truth** for v3.11.0 documentation structure.
>
> **v3.11.0 GA — 6/6 gates PASS** — see [`TPCH_SF1_22_22_PASS_REPORT.md`](TPCH_SF1_22_22_PASS_REPORT.md)

---

## 1. Active Documents

### 1.1 Version Management

| Document | Path | Status | Description |
|----------|------|--------|-------------|
| VERSION_PLAN.md | `VERSION_PLAN.md` | Current | Strategic summary (1-2 pages) |
| STAGE.yaml | `STAGE.yaml` | Current | Stage state, gates, thresholds |
| CHANGELOG.md | `CHANGELOG.md` | Current | Release notes (auto-generated) |

### 1.2 Plans (in `plans/`)

| Document | Path | Status | Description |
|----------|------|--------|-------------|
| V311_VERSION_PLAN.md | `plans/V311_VERSION_PLAN.md` | Current | Detailed strategic plan |
| V311_DEVELOPMENT_PLAN.md | `plans/V311_DEVELOPMENT_PLAN.md` | Current | 23 task breakdown (SSOT) |
| V311_DEBT_CLOSURE_PLAN.md | `plans/V311_DEBT_CLOSURE_PLAN.md` | Current | Debt clearance plan |
| V311_DOCS_RESTRUCTURE_PLAN.md | `plans/V311_DOCS_RESTRUCTURE_PLAN.md` | Current | Documentation restructuring |
| V311_ISSUE_CROSSREF.md | `plans/V311_ISSUE_CROSSREF.md` | Current | Task ↔ Gitea issue mapping |
| INDEX.md | `plans/INDEX.md` | Current | Plan directory index |

### 1.3 Reports & Analysis

| Document | Path | Status | Description |
|----------|------|--------|-------------|
| PROGRESS.md | `reports/v3.11.0/PROGRESS.md` | **Current (SSOT)** | Unified task progress tracker |
| LEGACY_DEBT_TRACKING_TABLE.md | `LEGACY_DEBT_TRACKING_TABLE.md` | Current | Debt status snapshot |
| LEGACY_DEBT_AUDIT_REPORT.md | `LEGACY_DEBT_AUDIT_REPORT.md` | **Archived** | Historical audit (see PROGRESS.md) |
| FEATURE_CHECKLIST.md | `FEATURE_CHECKLIST.md` | Deprecated | Merged into DEVELOPMENT_PLAN |

### 1.4 Performance Reports

| Document | Path | Status | Description |
|----------|------|--------|-------------|
| TPCH_PERFORMANCE_REPORT.md | `reports/v3.11.0/perf/TPCH_PERFORMANCE_REPORT.md` | Current | Consolidated TPC-H results |
| SOAK_TEST_REPORT.md | `SOAK_TEST_REPORT.md` | Current | SOAK test methodology |
| SOAK_PERFORMANCE_ANALYSIS.md | `SOAK_PERFORMANCE_ANALYSIS.md` | Current | Batch mode analysis |
| V311-09-soak-report.md | `reports/v3.11.0/V311-09-soak-report.md` | Current | V311-09 specific |
| COVERAGE_HISTORICAL_ANALYSIS.md | `COVERAGE_HISTORICAL_ANALYSIS.md` | Current | Coverage threshold history |
| COVERAGE_TESTING_METHODOLOGY.md | `COVERAGE_TESTING_METHODOLOGY.md` | Current | Coverage measurement |
| COVERAGE_E2E_AUG09.md | `COVERAGE_E2E_AUG09.md` | **Current (2026-08-09)** | E2E test coverage expansion |
| E2E_TESTING_GUIDE.md | `E2E_TESTING_GUIDE.md` | **Current (2026-08-09)** | E2E test development guide |
| DISABLED_TESTS_ANALYSIS.md | `DISABLED_TESTS_ANALYSIS.md` | Current | Disabled test analysis |
| EXTENSION_CRATES_ARCHIVE.md | `EXTENSION_CRATES_ARCHIVE.md` | Current | Extension crate decisions |

### 1.5 Performance Details (in `perf/`)

| Document | Path | Status | Description |
|----------|------|--------|-------------|
| SF1_BASELINE_REPORT.md | `perf/SF1_BASELINE_REPORT.md` | Current | SF=1 baseline |
| SF01_CROSS_ENGINE_BASELINE.md | `perf/SF01_CROSS_ENGINE_BASELINE.md` | Current | Cross-engine comparison |
| TPCH_QExecution_Analysis.md | `perf/TPCH_QExecution_Analysis.md` | Current | Query execution analysis |
| Q5_MEMORY_ANALYSIS.md | `perf/Q5_MEMORY_ANALYSIS.md` | Current | Q5 memory usage |
| HASH_ANTI_JOIN_PERF.md | `perf/HASH_ANTI_JOIN_PERF.md` | Current | Anti join performance |

---

## 2. Archived Documents

| Document | Reason | Archived Date |
|----------|--------|---------------|
| LEGACY_DEBT_AUDIT_REPORT.md | Superseded by PROGRESS.md | 2026-07-18 |
| FEATURE_CHECKLIST.md | Merged into DEVELOPMENT_PLAN | 2026-07-18 |

---

## 3. Gitea Issue References

| Issue | Title | Task |
|-------|-------|------|
| #3433 | v3.11.0 Release Preparation | V311-MASTER |
| #3431 | TPC-H SF=1 full performance baseline | V311-20 |
| #3434 | High-concurrency INSERT bug | V311-23 |

---

## 4. Quick Links

- **Task Progress**: `reports/v3.11.0/PROGRESS.md`
- **Stage Gates**: `STAGE.yaml`
- **Development Plan**: `plans/V311_DEVELOPMENT_PLAN.md`
- **Performance**: `reports/v3.11.0/perf/TPCH_PERFORMANCE_REPORT.md`

---

*Last updated: 2026-08-09 (E2E Coverage Expansion)*
