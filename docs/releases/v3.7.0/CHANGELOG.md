# Changelog v3.7.0

> **版本**: v3.7.0
> **分支**: develop/v3.7.0
> **日期**: 2026-05-30

---

## 变更日志

## v3.7.0 GA (2026-05-30)

#### P0 修复

- `01db4fdf` — fix(mysql-server): session-level engine cache for transaction state persistence
- `2607d788` — fix(mysql-server): SKIP_AUTH=false to restore authentication gate

#### VTU IR Validation

- `a17f13f3` — Merge PR #2618: feat(vtu): Phase 1.6 IR validation - PredicateIR + AstAdapter for UPDATE
- `dafe8680` — feat(vtu): Phase 1.6 IR validation - PredicateIR + AstAdapter for UPDATE

#### 文档

- `5e11bd04` — docs(v3.7.0): GA re-evaluation after P0 fixes — score 41→65
- `bf10eb8d` — docs(v3.7.0): add INTEGRATION_DEBT_REPORT — freeze report with legacy debt tracking
- `b925f438` — docs: add v3.7.0 RELEASE_SUMMARY + v3.8.0 full doc set (LEGACY_ISSUES, etc.)

#### 版本信息

| 指标 | 值 |
|------|-----|
| GA Score | 65/100 (81%) |
| P0 Blockers | 2/2 Fixed |
| Unit Tests | 93/93 PASS |
| E2E | 28/28 PASS |
| TPC-H SF=1 | 22/22 PASS |

### v3.7.0-alpha (2026-05-30)

#### 合并

- `af886c46d` — Merge PR #2608: 将 v3.6.0 最新变更（executor 模块重构 + clippy 修复 + 门禁脚本）合并到 v3.7.0
- `f048ef029` — Merge PR #2594: v3.6.0 治理体系改进 + Build Fixes
- `34ac54e6c` — Merge PR #2595: docs: sync v3.4.0+v3.5.0 docs to develop/v3.6.0

#### 文档

- `88c500525` — docs(phase-3): Add WAL DML integration technical report for v3.7.0
- `15173eb72` — docs: update VERSION_HISTORY.md with v3.5.0~v3.7.0 entries
- `db61d8908` — docs(v3.6.0): add INTEGRATION_DEBT_REPORT + v3.7.0 roadmap

#### 修复

- `5f2821bf7` — fix: v3.6.0 build fixes for Beta entry
- `9e035cdc1` — fix: verify_beta_entry.sh syntax fix

#### 执行器

- `d95407183` — fix(executor): complete missing modules + clippy fixes
- `4a5a7f193` — fix(window): add missing PercentRank/CumeDist and fix test assertion
- `045d4f3c` — test: parser coverage integration tests (32 tests, 0 failed)

---

## 版本历史

| 版本 | 日期 | 状态 |
|------|------|------|
| v3.7.0 | 2026-05-30 | GA |
| v3.6.0 | 2026-05-30 | GA |
| v3.5.0 | 2026-05-17 | GA |
| v3.4.0 | 2026-05-10 | GA |
| v3.3.0 | 2026-04-25 | GA |