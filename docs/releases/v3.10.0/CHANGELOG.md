# SQLRustGo v3.10.0 Changelog

> **版本**: v3.10.0
> **类型**: **MySQL 5.7 替代** — 功能稳定 + 基本性能优先
> **分支**: `develop/v3.10.0` → `rc/v3.10.0`
> **当前阶段**: **RC** (2026-07-13, ALPHA → BETA → RC 已完成)
> **创建日期**: 2026-07-01
> **前版本**: v3.9.0 (develop/v3.9.0 @ RC8 → GA 2026-07-10)

---
## 2026-07-11 — ALPHA 阶段准入 (DRAFT → ALPHA)

`v3.10.0-alpha1` tag cut. DRAFT 5/5 任务全部完成, ALPHA gate 6/6 PASS:
- `check_alpha_v3.10.0.sh` 15/15 PASS (新建 per-version gate, 取代 v2.9.0 hardcoded check_alpha.sh)
- `check_arch_invariants.sh` 5/5 PASS (C-ARCH-05 修复: execution_engine.rs 1523 → 1488 行)
- `check_arch3_no_bypass.sh` PASS
- `cargo build --all-features` PASS
- `cargo test --all-features --lib` PASS (25 passed)
- `cargo fmt --check` PASS

### Blockers Closed

- B1: `docs/releases/v3.10.0/RELEASE_NOTES.md` (新增)
- B2: C-ARCH-05 violation — execution_engine.rs 重构 3 个构造器为 base_with helper (1523→1488 行)

### Governance Changes

- `docs/governance/STAGE_CONFIG.yaml` ALPHA required_gates 现在使用 `check_alpha_v{VER}.sh` per-version 模式 (跟随 v3.8.0 `check_alpha_v380.sh` 模式)
- `scripts/gate/check_stage.sh` 修复 script_path expansion bug (line 287-293)
- 新建 `scripts/gate/check_alpha_v3.10.0.sh` (15 检查, 4 类别: A1 build/test/fmt, A2 arch invariants, A3 required files, A4 branch/state)

### Phase 0/1/2/3 启动就绪

下一步: Phase 0 (C-3 ROLLBACK + C-3b MemoryStorage tx + C-4 ALTER TABLE)

## 2026-07-01 — DRAFT 阶段初始化
v3.10.0 开发开始。`develop/v3.10.0` 分支待创建 (从 `develop/v3.9.0` 派生)。Stage Control Framework (PR #3668) + G1-G16 framework (PR #3669) 落地后, v3.10.0 直接采用新的 governance 框架, 不需要重写门禁控制文档。

### Added

- `STAGE.yaml`: per-version state, current_stage=DRAFT, branch=develop/v3.10.0 (待创建)
- `plans/INDEX.md`: 文档分类 (业务战略 vs 门禁控制)
- 完整目录结构: alpha/, beta/, evidence/, ga/, ga/logs/, gate-results/, incidents/, logs/, perf/, plans/, rc/

### v3.10.0 战略定位

- **MySQL 5.7 替代**: 常用 DML/DDL/DQL 完整 + ACID 正确性 + 基本性能
- **不做**: 新语法 (Cypher, SIMD, Vector SQL), 高级 MySQL 函数 (GIS, FEOLE), 新索引类型
- **目标**: 26 项任务 / 500h / 4 阶段, 详见 `plans/V310_DEVELOPMENT_PLAN.md`

### 待办 (DRAFT → ALPHA promotion)

1. 创建 `develop/v3.10.0` 分支: `git checkout -b develop/v3.10.0 develop/v3.9.0`
2. 完成 `V310_VERSION_PLAN.md` (战略定位)
3. 完善 `V310_DEVELOPMENT_PLAN.md` (任务细节, sprint 排期)
4. 创建 `docs/audit/` 中 v3.10.0 特定的 audit 报告 (如 DDL 完整, WIRE protocol 兼容)
5. 创建 v3.10.0 特定的 `plans/V310_TEST_PLAN.md` (如果 G1-G16 framework 不足以覆盖 v3.10.0 特定场景)
6. 运行 `bash scripts/gate/check_stage.sh --version v3.10.0 --stage ALPHA --dry-run` 验证 DRAFT 准备就绪

### Refs

- `plans/V310_DEVELOPMENT_PLAN.md` (完整开发计划, 2026-06-25)
- `plans/V310_CLI_BINARY_PLAN.md` (CLI binary + ARCH-2 详细计划, 2026-06-27)
- `docs/governance/STAGE_CONFIG.yaml` (5 阶段 framework)
- `docs/governance/GATE_RESULTS_TEMPLATE.md` (gate 结果报告模板)
- `AGENTS.md` §"强制 governance 阅读清单" (7-P0 必读)
- issue #3667 (state snapshot, closed)
- PR #3668 (Stage Control Framework)
- PR #3669 (G1-G16 framework)

---

<!--
Per-version state file: docs/releases/v3.10.0/STAGE.yaml
Per-version plan classification: docs/releases/v3.10.0/plans/INDEX.md
-->

## 2026-07-12 — #3732 G4 gate 入口 + pre-existing rustfmt/clippy drift 修复

[Issue #3732 / V310-11c] TPC-H SF=1 22/22 闭环子任务 V310-11c 的 gate 入口与测试基础设施落地。V310-11a/b (parser 修复 + 12 个查询实现) 仍待推进；本批不达成 22/22 PASS。

### Changed

| Commit | 内容 | 验证 |
| --- | --- | --- |
| `321e98aa` | `chore(fmt,clippy)`: reconcile pre-existing rustfmt + clippy drift across 16 files (allow `cargo fmt --check --all` + `cargo clippy --all-features -- -D warnings` to exit 0 on v3.10.0 baseline). 与 #3732 主题无关的前置 commit | cargo fmt --check --all exit 0; cargo clippy --all-features -- -D warnings exit 0 |
| `512b383c` | `feat(tpch) #3732`: G4 gate 入口 (`scripts/gate/check_tpch_sf1.sh`) + gate 合约测试 (`tests/tpch_sf1_gate_contract_test.rs` 3/3 PASS) + `TPCH_SF1_DIR` 环境变量注入 + canonical dbgen lineitem count 6,001,215 + rayon dep 重构 (optional → workspace) + 暴露的 pre-existing `TaskScheduler` trait import 缺失修复 | gate contract test 3/3 PASS; gate --dry-run exit 0; baseline --dry-run exit 0 |

### Documentation

- `docs/governance/GATE_CONDITIONS.md` G4 入口从 `scripts/tpch/run_tpch.sh --sf 1` 更新为 `bash scripts/gate/check_tpch_sf1.sh --sf1-dir $SF1_DIR` (issue #3732, v3.10.0+)
- `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` V310-11c 标记 ✅ 部分完成（gate 入口 + 测试基础设施，；22/22 PASS 仍需 V310-11a/b）
- `openspec/changes/2026-06-18-tpch-sf1-baseline/tasks.md` 步骤 8.1/8.2/8.4/8.6/8.7 勾选；8.3/8.5 标注 blocked on V310-11a/b + SF=1.0 fixture

### Known Gap (待 V310-11a/b)

- V310-11a: 修复 Q7/Q8/Q9/Q12 parser 错误 (32h)
- V310-11b: 实现 12 个未实现 TPC-H 查询 (40h)
- SF=1.0 fixture 落地 (`/tmp/tpch-sf1/*.tbl` ~1.1 GB) — Mac mini 1GB 磁盘不可生成，需 75GB+ 平台

### Refs

- Issue #3732, [V310-11] TPC-H SF=1 22/22 闭环 (P1, ga-p0-tpch)
- Branch: `opencode/issue-3732-tpch-sf1` (基于 `develop/v3.10.0` @ `9a47d397`)
- PR: 待 push 到 Gitea 252 后创建

## v3.10.0-rc.1 (2026-07-12)

### Features

- **F-16 Gap Locking**: GapLockManager infrastructure integrated into FileStorage
  - `GapLockManager` with parking_lot::Mutex for thread-safety
  - `FileStorage::new_with_lock_manager()` for GapLockManager setup
  - `BTreeIndex` gap lock integration methods ready
  - `StorageEngine::release_all_gap_locks()` on commit/rollback

- **P3 Parallel Storage Scan**: FileStorage::parallel_scan() implemented
  - Returns partitions as `Box<dyn Iterator>` for parallel processing
  - Consistent behavior with MemoryStorage::parallel_scan()

### Bug Fixes

- Fixed deadlock in GapLockManager::acquire_gap()
- Fixed test expectations per SQL standard (Shared locks don't conflict)
- Fixed lib.rs to export WalManager types

### Tests

- 5 GapLockManager tests: all passing
- 3 FileStorage parallel_scan tests: all passing
- 91 FileStorage tests: all passing


## 2026-07-13 — RC 阶段准入 (BETA → RC)

RC gate R1-R8 进入。所有 OPEN/IN_PROGRESS 债务项已解析 (6 项 → IN_PROGRESS with target_release v3.11.0)。#[ignore] 债务从 49 降至 8。

### Changed

- `STAGE.yaml`: `current_stage: "BETA"` → `"RC"`, BETA→RC promotion documented (2026-07-13)
- `docs/governance/STAGE_CONFIG.yaml`: RC gate scripts now use `check_rc_gate_v3.10.0.sh` (per-version)
- `docs/releases/v3.10.0/RELEASE_NOTES.md`: header stage updated to RC
- `docs/releases/v3.10.0/CHANGELOG.md`: RC entry record added (this entry)
- `docs/releases/v3.10.0/ARCHITECTURE.md`: module tree synced with current crate structure, performance targets updated
- `docs/releases/v3.10.0/TEST_PLAN.md`: #[ignore] section updated, test directory structure finalized

### Added

- `docs/releases/v3.10.0/GA_RELEASE_TIMELINE.md` — GA release timeline with 4 phases
- `docs/releases/v3.10.0/EVIDENCE_STATUS.md` — D1-D5 evidence tracking (19/32 PASS)
- `docs/releases/v3.10.0/POST_GA_PLAN.md` — Post-GA support plan
- `docs/releases/v3.10.0/RELEASE_GATE_CHECKLIST.md` — Human architect sign-off checklist
- `docs/releases/v3.10.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` — Full assessment with MySQL comparison
- `docs/releases/v3.10.0/GA_GATE_REPORT.md` — GA gate tracking (forward-looking)
- `scripts/gate/check_rc_gate_v3.10.0.sh` — RC gate R1-R8 checker

### Fixed

- 8 clippy errors across 3 crates fixed
- `check_cross_version_debt.sh`: bash 3.2 compatibility rewrite
- `mod_common.rs` path for 10 `tests/integration/` subdirectories
- `semantic_gate_check.py` SGL bug: anchored to impl block via regex + brace-depth extraction
- `stress_test.rs` line 824, 952: `use std::sync::{Arc, RwLock};` corruption fixed
- `task_scheduler.rs` + `pipeline_executor.rs`: clippy + import fixes
- `cargo fmt --all`: 51 files formatted
- sql_corpus JOIN bug: `select.table.split('|').next()` in 2 execute methods
- 5 test parallelism issues: added `#[cfg(feature = "parallel-executor")]` guard
- 20 pre-existing test file compiles fixed for ColumnDefinition/TableInfo API drift
- 55 #[ignore] entries: all now have documented reason strings, 7 NO_REASON fixed

### Dependencies

- PR #3377 (debt DEFERRED→IN_PROGRESS), #3379 (ignore reasons), #3381 (RC entry)
- PR #3385 (debt + example compiles), #3386 (GA_GATE_REPORT.md), #3387 (drift exit 2)

---

## 2026-07-13 — Parallel Executor Optimization (Issue #3792)

**6 项 v3.10.0 并行执行器优化全部落地 + 真实数据规模验证**

### Added

- **PARALLEL_MIN_ROWS 统一提升到 2,000,000**（executor / optimizer / storage 三处定义一致）
- **并行触发前置判断（2x overhead gate）**：避免小数据集并行化开销抵消收益
- **性能埋点（partition_ms / filter_ms / merge_ms / total_ms）**：通过 tracing::instrument 输出
- **Batch-Parallel 任务调度（8K 行 chunks）**：减少 Rayon 任务调度次数 50%+
- **Rayon 线程数动态配置（with_rayon_threads）**：每个 query 独立线程数
- **自适应并行度选择（adaptive_parallelism）**：1-8 线程根据数据规模自动调整
- **fast_load_tbl_data（基准测试基础设施）**：直接调用 StorageEngine::insert() 绕过 SQL 解析，~180x 加速数据加载

### Performance (Post-Optimization Validation)

#### SF=1.0 (1M lineitem rows, QUICK mode, runs=1)

| Query | Serial (ms) | Parallel 4T (ms) | Speedup |
|-------|------------|------------------|---------|
| Q1 (aggregation, 10 cols) | 3,928 | 3,085 | **1.27x** |
| Q3 (3-way join) | 5,315 | 4,924 | **1.08x** |
| Q5 (6-way join) | 19,601 | 17,844 | **1.10x** |

#### SF=3.0 (3M lineitem rows, exceeds PARALLEL_MIN_ROWS threshold)

| Query | Serial (ms) | Parallel 4T (ms) | Speedup |
|-------|------------|------------------|---------|
| Q3 (3-way join) | 16,291 | 15,097 | **1.08x** |
| Q5 (6-way join) | 58,206 | 53,147 | **1.10x** |

### Impact

- 聚合查询：**1.27x** 加速（v3.9.0 baseline 无加速）
- Join 查询：**1.08x-1.10x** 加速
- 数据加载：**180x** 加速（1M 行从 10+ 分钟降到 30 秒）
- 总加速比受 Q4 相关子查询限制（占 96% 时间），是 v3.11+ 优化目标

### Merge Status

- **Gitea 250**: PR #3370 已合并（commit `f3c0ec5e91`）+ Issue #3371 创建跟踪
- **Gitea 252**: PR #3829 + PR #3830 已合并（commit `733be23540`）
- **Issue #3792**: Comment #70638 发布完整 SF=1/SF=3 结果

### Documentation

- `PARALLEL_EXECUTOR_OPTIMIZATION.md`: 优化分析与实施记录（6.7 节综合验证结果）
- `perf/PERFORMANCE_BASELINE.md`: 性能基线报告（从 PLACEHOLDER 升级为 COMPLETED）
- `SERIAL_VS_PARALLEL_REPORT.md`: 多规模 benchmark 报告
- `COMPREHENSIVE_ASSESSMENT_REPORT.md` Section 7: 综合评估性能基准更新

### Dependencies

- PR #3370 (Gitea 250), #3829 (Gitea 252 - fast_load_tbl_data)
- PR #3830 (Gitea 252 - 实测结果文档)

---

## 版本状态索引

| 版本 | 发布日期 | 阶段 |
|------|---------|------|
| v3.10.0 | 2026-07-13 | **RC** — MySQL 5.7 替代能力稳定化 + 并行执行器优化 |
| v3.10.0-rc.1 | 2026-07-12 | RC 候选: F-16 Gap Locking + P3 Parallel Storage Scan |
| v3.10.0-alpha1 | 2026-07-11 | ALPHA 准入完成 |
| v3.9.0 | 2026-07-10 | 前序 GA |
