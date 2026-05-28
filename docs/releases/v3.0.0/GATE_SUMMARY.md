# v3.0.0 Gate Summary

> **版本**: v3.0.0
> **发布日期**: 2026-05-07
> **阶段**: GA (General Availability)
> **状态**: ⚠️ 有问题通过

---

## 一、门禁阶段完成状态

| 阶段 | 状态 | 完成证据 | Commit |
|------|------|----------|--------|
| Alpha | ✅ PASS | (无独立报告，基于 Beta 通过推断) | - |
| Beta | ✅ PASS | `BETA_GATE_REPORT.md` - 22/22 PASS | 409 tests |
| RC | ⚠️ PASS | `RC_GATE_REPORT.md` - 8/12 PASS, 67% | 8 blockers |
| GA | ✅ PASS | `GA_GATE_REPORT.md` (with exemptions) | threshold lowered |

---

## 二、关键 Commit 记录

### GA Gate Fixes
- `65440326 Merge pull request 'fix(ga-gate): GA-6 coverage threshold 40%→22%' (#599)`
- `f32c2d7c fix(ga-gate): GA-6 coverage threshold 40%→22% (local 23.9%, CI higher)`
- `6c5f342e fix(gate): bash 3.2 compat, GA-6 40%, GA-14 SQL Corpus 95%`
- `c39b0f79 fix(gate): bash 3.2 compat - lower GA-6 to 40%`

### RC Gate
- `be1e7d46 fix(ga-gate): unify SQL Corpus threshold to ≥98%`
- `c8e6f28b fix(ga-gate): unify SQL Corpus threshold to ≥98% (GA-GAP-03)`

---

## 三、门禁阈值异常（阈值腐蚀追踪）

**⚠️ 严重问题**: v3.0.0 是历史上门禁最混乱的版本。GA-6 覆盖率阈值在两周内连续降级两次，从规范定义的 85% 一路降至 22%，以"符合"现实而非提升质量。

### GA-6 覆盖率阈值腐蚀轨迹

| 阶段 | Commit | 阈值 | 说明 |
|------|--------|------|------|
| **初始设计** | c082401e | **85%** | 规范设计值（符合 gate_spec_v300.md） |
| **第一次降级** | ec9219bf | **40%** | "lower GA-6 coverage to 40%" — 降低 45 个百分点 |
| **最终执行** | f32c2d7c | **22%** | "coverage threshold 40%→22%" — 再降 18 个百分点 |

> **阈值腐蚀 = 职业道德问题**。不是"适应现实"，是向现实妥协。
> 正确的做法是：发现问题 → 补充测试 → 提升覆盖率 → 再发布。

### 根因分析

v3.0.0 引入了大量新模块（SSI、v3.0 major 版本），但测试覆盖率未同步跟上。
实际覆盖率约 23.9%（本地），CI 环境更高但仍远低于 85%。
为赶发布进度，阈值被连续两次降级。

| Gate | 规范定义 | 初始阈值 | 最终阈值 | 实际值 | 偏差 |
|------|---------|---------|---------|--------|------|
| GA-6 | 85% | 85% | **22%** | 23.9% | **-63%** |

---

## 四、覆盖率记录

| 阶段 | 目标 | 实际 | 状态 |
|------|------|------|------|
| Alpha | 50% | 43% | ❌ FAIL |
| Beta | ≥75% | ~76% | ✅ PASS |
| RC | ≥85% | ~76% | ❌ FAIL |
| GA | 85%→22% | 23.9% | ⚠️ 阈值被降至 22% |

---

## 五、RC Gate 结果

| Gate | 检查项 | 状态 | 结果 |
|------|--------|------|------|
| R1 | Build | ✅ PASS | Build successful |
| R2 | Test --all-features | ⚠️ TIMEOUT | Partial pass |
| R3 | Clippy | ✅ PASS | Zero warnings |
| R4 | Format | ✅ PASS | Format check passes |
| R5 | Coverage ≥85% | ❌ FAIL | ~76% |
| R6 | Cargo audit | ⚠️ SKIP | proc-macro-error unmaintained |
| R7 | Docs links | ✅ PASS | All links valid |
| R8 | SQL Corpus ≥95% | ✅ PASS | **95.0%** 精确达标 |
| R9 | TPC-H SF=1 | ✅ PASS | 22/22 queries |
| R10 | Performance baseline | ⚠️ FAIL | -70% regression |
| R11 | Sysbench QPS | ❌ FAIL | 1269 vs 50000 |
| R12 | Proof check | ✅ PASS | 20 proof files valid |

**Total**: 8/12 PASS | **BLOCKERS**: R5, R10, R11

---

## 六、Tags

- `v3.0.0-alpha` - Alpha 发布
- `v3.0.0` - GA 发布

---

## 七、结论

**v3.0.0** 已通过 GA 门禁，但存在严重的门禁阈值问题。

- **Commits**: 466（vs v2.6.0 的 1959）
- **关键变更**: SI→SSI, v3.0 major 版本
- **问题**: 
  - 门禁阈值被大幅降低以"符合"现实
  - 不是真正的质量达标
  - 是历史上门禁最混乱的版本