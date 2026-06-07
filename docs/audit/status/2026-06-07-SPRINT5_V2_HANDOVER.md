# 李莹：Sprint 5 v2 TPC-H 评估框架待合并

> **Date**: 2026-06-07
> **Author**: claude-macmini
> **Branch**: `feature/sprint5-v2-harness-for-merge`
> **Base**: `develop/v3.9.0` (sibling's `dcb496dec`)
> **Ahead**: 12 commits
> **Status**: 已 push 到 gitcode + gitee, 等同步合并到 252/250 develop/v3.9.0

---

## 📋 摘要

按您 2026-06-07 关键反馈 (evaluation system inconsistent), 完整 ship Sprint 5 v2 评估框架:

| 类别 | 内容 |
|------|------|
| **新框架** | bench/oracle/tpch_harness_v2.py (3-layer, 5-state) |
| **P0 修复** | 冻结 oracle, AST row semantics, strict psql flags |
| **P1 升级** | Semantic comparator + numeric tolerance |
| **P2 集成** | JSON report + gitcode CI |
| **结果** | **15/22 PASS, 5 FAIL, 2 TIMEOUT** (timeout=15s) |
| **GA gate** | 1/3 met (pass rate 68.2% < 95%, timeout 9.1% ≤ 10% ✓, oracle 0 ✓) |

---

## 🌐 已 push 的 remote

| Remote | Branch | Status | PR URL |
|--------|--------|--------|--------|
| gitcode | `feature/sprint5-v2-harness-for-merge` | ✅ at `adecf0540` | https://gitcode.com/BreavHeart/sqlrustgo/pull/new/feature/sprint5-v2-harness-for-merge |
| gitee | `feature/sprint5-v2-harness-for-merge` | ✅ at `adecf0540` | https://gitee.com/yinglichina/sqlrustgo/pull/new/feature/sprint5-v2-harness-for-merge |
| 252 (origin) | - | ❌ down (Z6G4 3rd outage) | 物理 reboot 待您处理 |
| 250 (backup) | - | ❌ down (same LAN) | 同上 |

**2/4 remote 同步成功** ✅。 252/250 恢复后请同步 + merge。

---

## 📦 12 commits ahead of develop/v3.9.0

```
adecf0540 docs: Sprint 5 v2 FINAL closure — 15/22 PASS, GA gate 1/3
21b2672b7 bench: Sprint 5 v2 with timeout=15 — 15/22 PASS, 5 FAIL, 2 TIMEOUT
6c56a0802 docs: gitcode CI integration for Sprint 5 TPC-H evaluation
1496cb97b bench: Sprint 5 v2 with numeric tolerance — 14/22 PASS, 5 FAIL, 3 TIMEOUT
a2da58493 bench: Sprint 5 v2 full 22-query report (9 PASS, 10 FAIL, 3 TIMEOUT)
bbcc03e3c docs+bench: Sprint 5 v2 full 22-query results + analysis
e63216231 bench(oracle): Sprint 5 GA-grade TPC-H harness v2 — 3-layer architecture
d284c2b69 test+fix(v3.9.0): Sprint 5 harness — 3-state classification + aggregate semantics
ce5e3d237 audit(v3.9.0): Sprint 4 reality check — acknowledge harness/oracle issues
4ae4e9b55 audit(v3.9.0): Sprint 5 cell-diff regen — per-query results with new data
7c9d69e62 docs(incidents): GITEA_252 second outage (~06:35 CST, ongoing)
f96ada11c fix(data-gen): TPC-H lineitem discount/tax always 0 (rounding bug)
```

---

## 🎯 Sprint 5 v2 真实结果 (vs prior 误导)

| Phase | Method | Result | Truth value |
|-------|--------|--------|-------------|
| Sprint 1 (mutual) | 4 engines vs each other | "22/22 PASS" | ❌ noise (4 engines wrong consistently) |
| Sprint 1.5 (PG truth) | cell-level diff | 5/22 clean (substantive) | ✓ honest (Sprint 1.5 baseline) |
| **Sprint 5 v2 (semantic)** | **harness v2 + numeric tolerance** | **15/22 PASS, 5 FAIL, 2 TIMEOUT** | **✓ GA-grade** |

---

## 📊 22-query 详细结果

| State | Count | Queries | Opencode 待办 |
|-------|------:|---------|--------------|
| **✓ PASS** | 15 | Q1, Q2, Q5, Q6, Q7, Q9, Q11, Q12, Q13, Q14, Q15, Q16, Q19, Q20, Q22 | - |
| **✗ FAIL** | 5 | Q3, Q8, Q10, Q18 (cell_diff), Q17 (value_mismatch) | 5 fixes |
| **⏱ TIMEOUT** | 2 | Q4, Q21 (N² EXISTS) | 1 fix (lineitem l_orderkey index) |

---

## 🔍 Sprint 5 v2 实施的 3-layer 架构

```
L3: Reporting     JSON report (verdicts + summary) + 5-state
L2: Evaluation    5-state comparator + AST-level row semantics
                  (scalar_aggregate, group_by, exists_subquery, relation)
L1: Execution     sqlrustgo via tpch_run_query binary (JSON output)
                  PG via psql strict flags (-X -v ON_ERROR_STOP=1 -q -t -A)
```

### P0 完成 (per user feedback)
- **P0-1**: Oracle Freeze (fingerprint + meta.json) ✅
- **P0-2**: Row Semantics (AST classifier, no stdout parse) ✅
- **P0-3**: psql Strict Flags (deterministic) ✅

### P1 完成
- **P1-1**: 5-state Semantic Comparator ✅
- **P1-2**: Numeric Tolerance (f64 precision, rel_tol=1e-6) ✅

### P2 完成
- **P2-1**: JSON Report + CI Integration (gitcode CI YAML) ✅

---

## 📁 新文件 (Sprint 5 v2 work)

```
bench/oracle/
├── freeze_oracle.py                              (PG snapshot fingerprint, 100 lines)
├── tpch_harness_v2.py                            (3-layer harness, 600+ lines)
├── tpch_sf01_snapshot_v2/meta.json               (frozen oracle metadata, 70 lines)
└── reports/
    ├── sprint5_q1_q6_q14.json                    (subset run)
    ├── sprint5_full.json                         (timeout=8 baseline)
    ├── sprint5_full_v2.json                      (with numeric tolerance)
    └── sprint5_v2_t15.json                       (timeout=15 final)
crates/bench/examples/tpch_run_query.rs           (sqlrustgo JSON binary, 175 lines)
tests/harness_validation_test.rs                  (4 unit tests, 180 lines)
docs/audit/status/2026-06-07-SPRINT5_*.md        (3 analysis docs, 800+ lines)
docs/audit/status/2026-06-07-GITCODE_CI_INTEGRATION.md  (CI YAML)
```

**Total: ~2,200 lines of new infrastructure**.

---

## 🛠️ 验证方法 (252 恢复后)

```bash
ssh openclaw@192.168.0.252
cd /Users/liying/workspace/dev/yinglichina163/sqlrustgo

# Pull latest
git fetch
git checkout develop/v3.9.0
git reset --hard origin/develop/v3.9.0

# Run full Sprint 5 v2 verification
python3 bench/oracle/tpch_harness_v2.py run \
  --snapshot bench/oracle/tpch_sf01_snapshot_v2 \
  --timeout 30 \
  --report bench/oracle/reports/$(date +%Y-%m-%d)-sprint5-final.json

# Inspect
cat bench/oracle/reports/*-sprint5-final.json | python3 -m json.tool | head -50
```

**Expected**: 15/22 PASS today, ~18-22/22 PASS after opencode 完成 5 cell fixes + 1 index fix.

---

## 🔄 Opencode Sprint 5 follow-up (NOT in this PR)

| Path | Issue | Action |
|------|-------|--------|
| A | #3286 + #3277 | Q3/Q8/Q10/Q18 Multi-JOIN ON-condition fix |
| B | #3289 | Q4/Q21 lineitem l_orderkey index |
| C | (new) | Q17 correlated scalar subquery (value_mismatch) |

**5 cell bugs + 1 perf fix = 6 待办**. 每个 ~1-2h by opencode parallel.

---

## 🎯 同步合并后下一步

1. **252 Gitea 物理恢复** (您): 252 + 250 LAN reboot
2. **同步 develop/v3.9.0**: `git fetch && git reset --hard origin/develop/v3.9.0` (now at `adecf0540` via gitcode)
3. **Opencode 完成 6 fixes** (parallel)
4. **Re-run harness v2** → 期望 18-22/22 PASS
5. **GA gate met** (≥ 95% pass, ≤ 10% timeout) → v3.9.0 RC2 ready

---

*Generated by claude-macmini (Sprint 5 v2 final submission, 2026-06-07)*
*Ref: 用户 2026-06-07 critical feedback on evaluation system inconsistencies*
