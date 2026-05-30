# PATTERN_GATE_FALSE_POSITIVE.md

> **Pattern**: Gate False Positive Detection and Resolution  
> **Trigger**: Gate report claims PASS but actual execution shows FAIL  
> **Source**: v3.6.0 Beta Gate (2026-05-30) — 0/8 actual vs 7/9 claimed  
> **Author**: Hermes Agent  
> **Date**: 2026-05-30  

---

## Context

Gate False Positive occurs when a Gate report claims PASS but actual execution would show FAIL. This was observed in v3.6.0 Beta Gate, where the report claimed 7/9 PASS but actual execution showed 0/8.

---

## Trigger Conditions

This pattern is triggered when:

1. Gate report claims PASS but no actual command output is included
2. Gate report references "documented results" without recent execution
3. Gate report has not been updated after code changes
4. Document State ≠ Execution State

---

## Failure Mode

### Wrong Action: Trust report without verification

```
❌ 错误做法:
1. 读 Gate 报告
2. 看到 "B1 Build: ✅ PASS" 
3. 不执行验证就接受结论
4. 继续下一步开发
```

**Why it fails**: 文档可能过时或包含未验证的 Claim。

### Wrong Action: Update report without executing

```
❌ 错误做法:
1. 执行了部分 Gate 命令
2. 部分 PASS，部分 FAIL
3. 更新报告只记录 PASS 项
4. 不记录 FAIL 项
```

**Why it fails**: 违反 Truthfulness Framework G-03。

---

## Correct Action

### Right Action: Execute and verify before accepting

```
✅ 正确做法:
1. 读 Gate 报告
2. 检查是否有实际命令输出作为 Evidence
3. 如果没有，执行 Gate 命令验证
4. 记录实际输出
5. 对比报告 vs 实际
6. 发现差异则更新报告并标注
```

### Step-by-Step Verification

```bash
# Step 1: 检查报告中的 Evidence
grep -A10 "B1" docs/releases/v3.6.0/BETA_GATE_REPORT.md
# 查找: Actual Command + Actual Output

# Step 2: 如果无 Evidence，执行验证
cargo build --release --workspace 2>&1 | tee /tmp/b1_output.txt
cargo test --lib --workspace 2>&1 | tee /tmp/b2_output.txt

# Step 3: 记录实际输出
echo "B1: $(tail -1 /tmp/b1_output.txt)"

# Step 4: 对比报告 vs 实际
# 如果一致 → 接受
# 如果不一致 → 标注差异，更新报告
```

---

## Example: v3.6.0 Beta Gate False Positive

### Timeline

| 时间 | 事件 | 结果 |
|------|------|------|
| 2026-05-29 | v3.6.0 Beta Gate 执行 | 声称 7/9 PASS |
| 2026-05-30 | Beta Gate 检查 | 0/8 实际通过 |
| 2026-05-30 | 用户发现 | 门禁形同虚设 |
| 2026-05-30 | 原因 | 文档齐全但未实际执行 |

### Root Cause

- Beta Gate 入口文档存在（BETA_GATE_CHECKLIST.md）
- 但 script `verify_beta_entry.sh` 从未实际执行
- 文档状态 ≠ 执行状态

---

## Prevention

1. **Rule G-03**: Gate 报告必须包含实际命令和输出
2. **Evidence Chain**: Claim → Actual Command → Actual Output → Conclusion
3. **Freshness 标记**: 报告必须标注"最后验证时间"
4. **定期复验**: 每次版本迭代前复验 Gate 报告

---

## Related Patterns

- `PATTERN_EVIDENCE_CHAIN.md` — Evidence Chain 构建
- `PATTERN_COVERAGE_DISPUTE.md` — 覆盖率争议解决

---

## SSOT 引用

- `docs/governance/GATE_CONDITIONS.md` — G-05: Document State ≠ Execution State
- `docs/governance/adr/ADR-001-truthfulness-framework.md` — G-03: Gate 报告必须包含实际命令