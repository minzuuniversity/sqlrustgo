# v3.11.0 GA 发布时间线

> **版本**: v3.11.0
> **当前状态**: GA，2026-08-09 评估更新
> **负责人**: @openclaw
> **说明**: 原英文时间线包含 2026-07-19 RC -> GA 过渡期的旧 PENDING 口径，已作为历史原文移入附录。当前状态以中文主文和综合评估报告为准。

## 1. 当前时间线

| 日期 | 里程碑 | 当前解释 |
|---|---|---|
| 2026-07-15 | 从 v3.10.0 创建 v3.11.0 分支 | 已完成 |
| 2026-07-15 | DRAFT -> ALPHA | 已完成 |
| 2026-07-18 | ALPHA -> RC | 已完成 |
| 2026-07-19 | 曾出现 RC/GA 混合与后续回退/整改 | 历史阶段，不能单独作为 GA 证据 |
| 2026-08-09 | v3.11.0 GA 综合评估更新 | 当前正式 GA 判断入口 |
| 2026-08-09 | 3.12.0 规划承接 v3.11 弱项 | 作为后续生产硬化路线 |

## 2. GA 证据边界

| 要求 | 当前依据 | 判断 |
|---|---|---|
| RC/GA 阶段状态 | `STAGE.yaml`、`GA_GATE_REPORT.md`、`COMPREHENSIVE_ASSESSMENT_REPORT.md` | 以最新综合评估为准 |
| Full test / coverage | coverage 报告和 GA gate 报告 | 注意 `--lib` 与 `--lib --tests` 口径差异 |
| TPC-H SF=1 | `TPCH_SF1_22_22_PASS_REPORT.md` | 证明 22/22 可运行性；correctness 仍需跨引擎 hash/row-count |
| Documentation | release docs 和 governance self-audit | 可作为文档完整性证据，不替代测试执行 |
| Security audit | `SECURITY_AUDIT.md` 和审计输出 | 依赖风险需按最新 `cargo audit` 复跑 |

## 3. 回滚原则

如果 GA 后发现 hard gate 证据不成立，应：

1. 明确受影响 claim 和证据缺口。
2. 在 `STAGE.yaml` 或相关 gate 报告中降级或标注限制。
3. 修复后重新执行对应 gate。
4. 使用 PR/commit 记录整改过程。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# v3.11.0 GA Release Timeline

> **Version**: v3.11.0
> **Status**: RC → GA (2026-07-19)
> **Owner**: @openclaw

---

## Release Schedule

| Date | Milestone | Status |
|------|-----------|--------|
| 2026-07-15 | Branch created from v3.10.0 | ✅ Complete |
| 2026-07-15 | DRAFT → ALPHA | ✅ Complete |
| 2026-07-18 | ALPHA → RC | ✅ Complete |
| 2026-07-19 | RC → GA | 🔄 **IN PROGRESS** |
| 2026-07-20 | GA tag cut | Pending |
| 2026-07-21 | Release binaries published | Pending |
| 2026-07-21 | crates.io publish | Pending |
| 2026-07-22 | 168h SOAK starts | Pending |

---

## GA Gate Evidence

| Requirement | Evidence | Status |
|-------------|----------|--------|
| RC Gate PASS | RC_GATE_REPORT.md | ✅ |
| Full test suite | 300+ tests | ✅ |
| Coverage ≥ 75% | L1_8 avg 80.60% | ✅ |
| TPC-H SF=1 ~10/22 (honest status, see SF1_TRUTH_AUDIT.md) | scripts/tpch/run_sf1.sh | ⚠️ PENDING (fixture missing) |
| Documentation | CHANGELOG, UPGRADE_GUIDE, ARCHITECTURE | ✅ |
| Security audit | Code review | ⚠️ Pending |

---

## Rollback Plan

If GA release fails:
1. Revert `develop/v3.11.0` to previous known-good commit
2. Investigate failure root cause
3. Apply fix and re-promote through RC gate

---

## Contacts

| Role | Contact |
|------|---------|
| Release Manager | @openclaw |
| Build Engineer | @ci |
| QA Lead | @qa |
