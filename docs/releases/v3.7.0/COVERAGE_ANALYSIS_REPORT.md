# v3.7.0 Coverage Analysis Report

> **版本**: v3.7.0  
> **分支**: `develop/v3.7.0` (commit `f8cf815f`)  
> **日期**: 2026-05-30  
> **Gate Type**: Beta Coverage Gate  
> **Auditor**: Hermes Agent

---

## 0. 执行摘要

| 维度 | 状态 | 数据 |
|------|------|------|
| L1 Crates Average | ✅ PASS | ≥75% (Beta threshold) |
| Per-Crate Minimum | ✅ PASS | ≥50% per crate |
| Parser | ⚠️ 待提升 | 见缺口分析 |
| Executor | ✅ 已达标 | 见实测数据 |

**SSOT 参考**: `Governance-Gate-Phases.md`  
**Beta Coverage 阈值**: 平均 ≥75%（每 crate ≥50%）

---

## 1. 覆盖率测量方法

**测量标准**: 根据 RC2 (commit aa830bcd) 确立的综合方法

```bash
# 综合方法：--tests 优先，--lib fallback
for crate in "${L1_CRATES[@]}"; do
    cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null | grep "^TOTAL"
done
```

**L1 Crates 列表**:
- sqlrustgo-types
- sqlrustgo-parser
- sqlrustgo-planner
- sqlrustgo-optimizer
- sqlrustgo-executor
- sqlrustgo-storage
- sqlrustgo-transaction
- sqlrustgo-catalog

---

## 2. 实测数据 (来源: GA_GATE_REPORT.md Section 5)

| Crate | 覆盖率 | Beta 阈值 | 状态 |
|-------|--------|-----------|------|
| types | 87.65% | ≥50% | ✅ |
| parser | 78.18% | ≥50% | ✅ |
| planner | 89.39% | ≥50% | ✅ |
| optimizer | 83.67% | ≥50% | ✅ |
| executor | 83.00% | ≥50% | ✅ |
| storage | 81.75% | ≥50% | ✅ |
| transaction | 87.79% | ≥50% | ✅ |
| catalog | 88.52% | ≥50% | ✅ |
| **平均** | **84.99%** | ≥75% | ✅ |

---

## 3. 缺口分析

### 3.1 Parser 覆盖率 (78.18%)

**根本原因**: 嵌套测试模式导致子测试覆盖率无法计入  
**Issue**: I#2580 [debt:v3.5.0] Parser coverage structural deficiency  
**计划**: v3.8.0 修复

### 3.2 Executor 覆盖率 (83.00%)

**状态**: 已达标 Beta 阈值  
**Issue**: I#2582 [debt:v3.5.0] Executor coverage below GA threshold  
**备注**: 83% > 75% Beta threshold，但 < 85% GA threshold

---

## 4. Beta Coverage Gate 判定

| 检查项 | 标准 | 实际 | 状态 |
|--------|------|------|------|
| L1 平均覆盖率 | ≥75% | 84.99% | ✅ PASS |
| Per-crate 最低 | ≥50% | 78.18% (parser) | ✅ PASS |

**Beta Coverage Gate**: ✅ **PASS**

---

## 5. 遗留缺口 (GA Target)

| Crate | 当前 | GA Target | 缺口 |
|-------|------|-----------|------|
| parser | 78.18% | 85% | +6.82pp |
| optimizer | 83.67% | 85% | +1.33pp |
| executor | 83.00% | 85% | +2.00pp |

**计划**: v3.8.0 GA 前提升至 85%

---

## 6. Evidence Chain

```
Coverage Analysis (f8cf815f)
├── 数据来源: GA_GATE_REPORT.md Section 5
├── 测量方法: 综合法 (--tests + --lib fallback)
└── Beta Gate: ✅ PASS (84.99% avg, ≥75% threshold)
```