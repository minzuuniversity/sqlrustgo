# PATTERN_EVIDENCE_CHAIN.md

> **Pattern**: Evidence Chain Construction  
> **Trigger**: Claim without verifiable evidence, or evidence without clear chain  
> **Source**: v3.7.0 GA — multiple evidence chain failures  
> **Author**: Hermes Agent  
> **Date**: 2026-05-30  

---

## Context

Evidence Chain is the backbone of Governance Replayability. A Claim without Evidence is a Failed Claim. This pattern was repeatedly violated in v3.7.0 GA:

- EX-v350-006: Claim without proper evidence (wrong measurement method)
- v3.6.0 Beta Gate: Report claims PASS without actual command output
- Coverage disputes: Multiple claims without source verification

---

## Trigger Conditions

This pattern is triggered when:

1. Writing a Gate report (must include actual commands)
2. Creating an Issue or EX entry (must include evidence)
3. Making a Claim in documentation (must include source)
4. Closing an Issue (must include PR evidence)

---

## Failure Mode

### Wrong Action: Write Claim without Evidence

```
❌ 错误做法:
1. 读文档
2. 下结论（Claim）
3. 写入报告
4. 无 Evidence 支持
```

**Why it fails**: 违反 Truthfulness Framework G-01。No Evidence = FAIL。

### Wrong Action: Provide Evidence without Chain

```
❌ 错误做法:
1. 提供数字 "84.99%"
2. 说 "来自实测"
3. 无命令、无输出、无来源
4. 无法追溯
```

**Why it fails**: Evidence 无 Chain，无法验证。

---

## Correct Action

### Right Action: Build Complete Evidence Chain

```
✅ 正确做法:
1. Claim: "Parser coverage 78.18% at RC2"
2. Actual Command: cargo llvm-cov test -p sqlrustgo-parser --all-features --tests
3. Actual Output: TOTAL ... 78.18%
4. SSOT Reference: RC_GATE_REPORT.md commit aa830bcd §5
5. Conclusion: Claim verified with综合方法 measurement
```

### Evidence Chain Template

```markdown
## Evidence Chain

### Claim
[Clear statement of what is being claimed]

### Evidence Type
[实测|SSOT引用|历史文档]

### Actual Command
```bash
[Exact command executed]
```

### Actual Output
```
[Actual stdout/stderr from command]
```

### SSOT Reference
[If applicable: document name + section/commit]

### Conclusion
[Verdict: Accepted/Rejected/Needs Investigation]
```

---

## Example: v3.7.0 Coverage Evidence Chain

### Claim
"Parser coverage 78.18% at RC2, meets Alpha threshold ≥75%"

### Evidence Type
实测 (comprehensive method)

### Actual Command
```bash
cargo llvm-cov test -p sqlrustgo-parser --all-features --tests 2>/dev/null | \
  grep "^TOTAL"
```

### Actual Output
```
TOTAL
regions      : 59.41%
functions    : 82.71%
lines        : 78.18%
```

### SSOT Reference
`RC_GATE_REPORT.md` commit aa830bcd §5, `GATE_CONDITIONS.md` §Alpha A5

### Conclusion
✅ Claim verified — 78.18% ≥ 75% Alpha threshold

---

## Prevention

1. **Truthfulness Framework G-01**: Claim ≠ Evidence — any Claim needs Evidence
2. **Truthfulness Framework G-03**: Gate reports must include actual commands
3. **SSOT Cross Check**: Verify thresholds against SSOT before claiming
4. **Evidence Chain Template**: Use standardized template for all Claims

---

## Related Patterns

- `PATTERN_COVERAGE_DISPUTE.md` — Coverage measurement verification
- `PATTERN_GATE_FALSE_POSITIVE.md` — Gate report verification

---

## SSOT 引用

- `docs/governance/GATE_CONDITIONS.md` — G-01: Claim ≠ Evidence
- `docs/governance/adr/ADR-001-truthfulness-framework.md` — Truthfulness Framework
- `docs/governance/replay/REPLAY_v3.7.0_GA.md` — v3.7.0 事件图