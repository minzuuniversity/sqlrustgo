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

*Last updated: 2026-07-18*
