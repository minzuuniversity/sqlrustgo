# PATTERN_COVERAGE_DISPUTE.md

> **Pattern**: Coverage Dispute Resolution  
> **Trigger**: Coverage number appears below threshold or inconsistent across measurements  
> **Source**: v3.7.0 GA — EX-v350-006 false positive (2026-05-29)  
> **Author**: Hermes Agent  
> **Date**: 2026-05-30  

---

## Context

Coverage disputes arise when different measurement methods produce conflicting results for the same codebase. This pattern was observed in v3.7.0 when a user measured 84.98% using `--lib only` and accused GA_GATE_REPORT of falsifying data showing 84.99%.

---

## Trigger Conditions

This pattern is triggered when:

1. Coverage number appears below threshold (e.g., 84.98% < 85%)
2. Coverage numbers differ between reports (GA_GATE_REPORT vs RC_GATE_REPORT)
3. Different measurement methods are suspected (--lib only vs --tests + --lib)
4. User creates EX/Debt entry based on coverage dispute

---

## Failure Mode

### Wrong Action: Immediately create EX/Debt entry

```
❌ 错误做法:
1. 读 GA_GATE_REPORT
2. 用 --lib only 测得 84.98%
3. 创建 EX-v350-006: "覆盖率测量方法不一致"
4. 指控 GA_GATE_REPORT 造假
5. 用户纠正: "综合方法在 RC2 就确立了"
```

**Why it fails**: 没有检查历史文档，使用了错误的测量方法（--lib only），错误指控了正确的报告。

---

## Correct Action

### Right Action: Verify measurement method before disputing

```
✅ 正确做法:
1. 读 SPEC/历史 Gate 报告 (检查顺序: RC_GATE_REPORT → GA_GATE_REPORT → GATE_SPEC_MASTER)
2. 确定 SSOT 中定义的测量方法
3. 使用相同的测量方法实测
4. 对比实测 vs 报告
5. 如有真正差异才创建 EX/Debt 条目
```

### Step-by-Step Resolution

```bash
# Step 1: 检查历史文档中的测量方法
git show origin/develop/v3.5.0:docs/releases/v3.5.0/RC_GATE_REPORT.md | grep -A5 "Coverage"

# Step 2: 使用 SSOT 定义的测量方法实测
for crate in "${L1_CRATES[@]}"; do
    if cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null | \
       grep "^TOTAL" | grep -q "%"; then
        echo "$crate: $(...) --tests"
    else
        echo "$crate: $(...) --lib (fallback)"
    fi
done

# Step 3: 对比实测 vs 报告
# 如果实测与报告一致 → 接受报告
# 如果实测与报告不同 → 进一步调查
```

---

## Example: v3.7.0 Coverage Dispute

### Timeline

| 时间 | 事件 | 行动 | 结果 |
|------|------|------|------|
| 2026-05-28 | RC2 commit aa830bcd | 确立综合方法 | --tests + --lib fallback |
| 2026-05-29 | GA_GATE_REPORT 发布 | 使用综合方法 | 84.99% |
| 2026-05-29 | 用户用 --lib only 测量 | 错误方法 | 84.98% |
| 2026-05-29 | 用户创建 EX-v350-006 | 错误指控 | 假阳性 |
| 2026-05-29 | 用户纠正 | "必须先检查历史文档" | Pattern discovered |

### Root Cause

- 用户未检查 RC_GATE_REPORT.md 就使用单一测量方法
- 综合方法在 RC2 commit aa830bcd 就已确立
- 用户使用 --lib only 是错误方法（--tests 优先）

---

## Prevention

1. **Rule**: 在准备指控前，先检查历史文档（检查顺序: RC_GATE_REPORT → GA_GATE_REPORT → GATE_SPEC_MASTER）
2. **SSOT 引用**: 任何覆盖率 Claim 必须包含 SSOT 引用
3. **测量方法记录**: 所有覆盖率测量必须记录使用的命令和参数
4. **Evidence Chain**: Claim → Actual Command → Actual Output → Conclusion

---

## Related Patterns

- `PATTERN_EVIDENCE_CHAIN.md` — Evidence Chain 构建
- `PATTERN_GATE_FALSE_POSITIVE.md` — Gate 假阳性检测

---

## SSOT 引用

- `docs/governance/GATE_CONDITIONS.md` — CONDITIONAL PASS 语义
- `docs/governance/adr/ADR-001-truthfulness-framework.md` — G-06: 文档 Claim 必须有 Freshness 标记
- `references/v350-comprehensive-coverage-verification-2026-05-29.md` — 综合覆盖率验证