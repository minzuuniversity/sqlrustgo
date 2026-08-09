# v3.11.0 证据状态

> **说明**: 本文件用于追踪 v3.11.0 证据链状态。英文原文保留在附录；若有旧状态，以最新综合评估和对应 gate 输出为准。

## 1. 证据分层

| 类型 | 可用于什么 | 限制 |
|---|---|---|
| 命令输出 / CI log | gate PASS/FAIL 判断 | 必须带时间、commit、输出位置 |
| Git commit / tag | 代码状态追溯 | 不等同测试已通过 |
| 报告/计划 | 解释背景和后续行动 | 不能替代实跑证据 |
| 历史英文原文 | 追溯旧状态 | 若与中文主文冲突，以中文主文为准 |

## 2. 当前重点证据

当前应优先查看 `COMPREHENSIVE_ASSESSMENT_REPORT.md`、`GA_GATE_REPORT.md`、`TPCH_SF1_22_22_PASS_REPORT.md`、`SOAK_168H_REPORT.md`、coverage 报告和 security audit 输出。

## 3. 风险

任何没有 command output、timestamp、source_agent、source_run、evidence_hash 和 output location 的 PASS claim，都应视为未充分证实。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# v3.11.0 Evidence Status

> **Status**: RC (2026-07-20 整改后) — **GA 未通过**（G3/G4 FAIL）

> **Owner**: @openclaw

Evidence binding for each GA gate requirement.

---

## G1: R1-R4 (RC Indicators)

| Evidence | File | Status |
|----------|------|--------|
| RC_GATE_REPORT.md | `docs/releases/v3.11.0/RC_GATE_REPORT.md` | ✅ |
| BETA_GATE_REPORT.md | `docs/releases/v3.11.0/BETA_GATE_REPORT.md` | ✅ |
| ALPHA_GATE_REPORT.md | `docs/releases/v3.11.0/ALPHA_GATE_REPORT.md` | ✅ |

---

## G2: Full Test Suite

| Evidence | File | Status |
|----------|------|--------|
| Test execution | `cargo test --workspace` | ✅ 300+ tests pass |

---

## G3: Coverage

| Evidence | File | Status |
|----------|------|--------|
| COVERAGE_REPORT.md | `docs/releases/v3.11.0/COVERAGE_REPORT.md` | ✅ |
| COVERAGE_TESTING_METHODOLOGY.md | `docs/releases/v3.11.0/COVERAGE_TESTING_METHODOLOGY.md` | ✅ |
| L1_8 Average | 80.60% | ✅ (≥ 75% Alpha threshold) |

---

## G4: TPC-H SF=1

| Evidence | File | Status |
|----------|------|--------|
| PERFORMANCE_BASELINE.md | `docs/releases/v3.11.0/perf/PERFORMANCE_BASELINE.md` | ✅ |
| TPC-H runner | `scripts/tpch/run_sf1.sh` | ⚠️ PENDING (script created; fixture at /tmp/tpch-sf1 missing; real 22/22 not executed) |

---

## G5: Security Audit

| Evidence | File | Status |
|----------|------|--------|
| SECURITY_AUDIT.md | `docs/releases/v3.11.0/SECURITY_AUDIT.md` | ⚠️ Pending |

Note: Security audit required before final GA. Current assessment based on code review.

---

## G6: Documentation

| Evidence | File | Status |
|----------|------|--------|
| CHANGELOG.md | `CHANGELOG.md` | ✅ Updated |
| UPGRADE_GUIDE.md | `docs/releases/v3.11.0/UPGRADE_GUIDE.md` | ✅ |
| ARCHITECTURE.md | `docs/releases/v3.11.0/ARCHITECTURE.md` | ✅ |

---

## Evidence Summary

| Requirement | Evidence | Status |
|-------------|----------|--------|
| G1 | RC/BETA/ALPHA Gate Reports | ✅ Complete |
| G2 | Full test suite | ✅ Complete (2,060 lib tests / 0 fail) |
| G3 | Coverage reports | **❌ FAIL**（实测 9-crate 平均 63.25%，8/9 < 80%；AUDIT_V311_REALITY_CHECK.md）|
| G4 | TPC-H SF=1 results | **❌ FAIL**（fixture 缺失；22/22 未跑；TPCH_SF1_VERIFICATION_REPORT.md）|
| G5 | Security audit | ⚠️ Pending |
| G6 | Documentation | ✅ PARTIAL（虚假声明已下架;audit/verify 报告已补;SOAK 报告新增）|
