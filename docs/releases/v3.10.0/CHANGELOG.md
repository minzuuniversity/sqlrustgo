# SQLRustGo v3.10.0 Changelog

> **版本**: v3.10.0
> **类型**: **MySQL 5.7 替代** — 功能稳定 + 基本性能优先
> **分支**: `develop/v3.10.0` (待从 `develop/v3.9.0` 创建)
> **当前阶段**: **DRAFT** (2026-07-01, 设计与规划阶段)
> **创建日期**: 2026-07-01
> **前版本**: v3.9.0 (develop/v3.9.0 @ RC8, 待 GA)
> **Maintainer**: claude-macmini (initial DRAFT setup)

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
