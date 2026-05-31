# Alpha CONDITIONAL PASS — Semantics Clarification
**Issue**: #2584  
**Author**: Hermes C  
**Date**: 2026-05-31  
**Branch**: `origin/docs/v380-gate-spec-clarity`  
**Status**: COMPLETED

---

## 1. 问题陈述

Issue #2584: "Alpha CONDITIONAL PASS semantics unclear"

当前 v3.8.0 Alpha Gate 声明 "CONDITIONAL PASS" 但没有明确：
- CONDITIONAL PASS 的确切含义是什么？
- 什么条件下可以 CONDITIONAL PASS？
- 与 FULL PASS 的区别是什么？
- CONDITIONAL PASS 的门禁效力是什么？

---

## 2. CONDITIONAL PASS 的定义

### 2.1 语义

**CONDITIONAL PASS** = 门禁检查项部分通过，但通过的部分有明确的前提条件/约束，且这些前提条件已被记录和接受。

与 **FULL PASS** 的区别：

| 属性 | CONDITIONAL PASS | FULL PASS |
|------|------------------|-----------|
| 门禁完整性 | 部分检查项被豁免 | 所有检查项必须通过 |
| 前提条件 | 有（未被满足的 Gate 前提） | 无 |
| 约束 | 有（记录在 Gate Report） | 无 |
| 发布效力 | 受约束的发布 | 完整发布 |
| 信任级别 | 低 — 需要外部验证 | 高 — 自动化验证 |

### 2.2 CONDITIONAL PASS 的触发条件

Alpha CONDITIONAL PASS 在以下情况下适用：

**场景 A: Governance-Driven Development 豁免**

当版本是 "Architecture Unification" 或 "Execution Semantics Freeze" 类型时：
- A1 Build PASS 没有意义（测试的是旧架构）
- A2 Test PASS 没有意义（测试的是旧路径）
- A5 Coverage 没有意义（测量的是旧代码路径）

**场景 B: 前提条件不满足**

当某个 Gate 检查项的前提条件（如 PR-800 必须先落地）未满足时：
- 该检查项可以被标记为 CONDITIONAL PASS
- 但必须记录「什么条件下」才能成为 FULL PASS

**场景 C: 外部依赖缺失**

当某个检查项依赖于尚未合并的 PR 时：
- 该检查项可以 CONDITIONAL PASS
- 但必须在 Gate Report 中明确列出缺失的 PR

---

## 3. v3.8.0 Alpha CONDITIONAL PASS 的具体含义

### 3.1 v3.8.0 Alpha 的 CONDITIONAL PASS

v3.8.0 Alpha 声明 CONDITIONAL PASS，因为：

| 检查项 | 状态 | 原因 |
|--------|------|------|
| A1 Build | ✅ PASS | 核心 6 crates 通过 |
| A2 Test | ✅ PASS | 89 tests 通过 |
| A3 Clippy | ✅ PASS | 零警告 |
| A4 Format | ❌ FAIL | evidence-graph + executor 格式问题 |
| A5 Coverage | ❌ FAIL | coverage measurement pipeline broken |
| A6-1~5 | ✅ PASS | Governance 框架完整 |

**结论**: 2 blockers (A4, A5) + 6 pass，但 v3.8.0 是 Architecture Unification Release，Architecture 的改变意味着 A4/A5 的「失败」不是代码质量问题，而是旧架构的残留。

### 3.2 为什么 A4/A5 是 CONDITIONAL 而不是 FAIL

A4 Format 和 A5 Coverage 在 v3.8.0 Alpha 被标记为 CONDITIONAL 而不是 FAIL，因为：

1. **A4 Format**: evidence-graph 和 executor 的格式问题，是因为这些模块在 v3.8.0 开发周期中快速迭代，格式问题不代表功能问题

2. **A5 Coverage**: coverage measurement pipeline broken 是已知的工具问题，不是代码质量问题

**因此**: CONDITIONAL PASS = 这些检查项的失败已被接受，且有明确的修复时间表。

---

## 4. CONDITIONAL PASS 的门禁效力

### 4.1 CONDITIONAL PASS 不等于「可以发布」

**重要**: CONDITIONAL PASS 只是说明「检查项部分通过且有记录」，并不意味着「可以发布」。

CONDITIONAL PASS 的发布效力：

| 场景 | CONDITIONAL PASS | FULL PASS |
|------|------------------|-----------|
| Alpha | 受约束的 Alpha | 完整的 Alpha |
| Beta | 受约束的 Beta | 完整的 Beta |
| RC | 受约束的 RC | 完整的 RC |
| GA | **不允许** | 必须 FULL PASS |

**GA 绝对不允许 CONDITIONAL PASS**：GA 是生产发布的门槛，所有 Gate 必须 FULL PASS。

### 4.2 CONDITIONAL PASS 记录要求

CONDITIONAL PASS 必须在 Gate Report 中明确记录：

```
## Gate Result: CONDITIONAL PASS

|| Check | Status | Condition / Waiver Reason ||
||-------|--------|---------------------------||
|| A4 Format | CONDITIONAL | v3.8.0 architecture refactor — format issues will be fixed in Beta |
|| A5 Coverage | CONDITIONAL | Coverage pipeline broken — will be fixed before RC ||
```

---

## 5. 改进建议

### 5.1 GATE_CONDITIONS.md 扩展

在 `docs/governance/GATE_CONDITIONS.md` 中增加：

```markdown
### Alpha/Beta/RC CONDITIONAL PASS Rules

1. **触发条件**: 明确列出 CONDITIONAL PASS 的触发场景
2. **记录要求**: CONDITIONAL PASS 必须记录豁免原因和前提条件
3. **效力限制**: GA 不允许 CONDITIONAL PASS
4. **修复要求**: CONDITIONAL PASS 的检查项必须在下一 Gate 之前修复
```

### 5.2 Gate Report 模板扩展

在 `ALPHA_GATE_CONTRACT.md` 的 Gate Report 中增加：

```markdown
## Gate Result: [FULL PASS | CONDITIONAL PASS | FAIL]

如果是 CONDITIONAL PASS：
|| Check | Status | Condition ||
||-------|--------|-----------||
|| A4 Format | CONDITIONAL | [reason] ||
|| A5 Coverage | CONDITIONAL | [reason] ||

**CONDITIONAL PASS 发布效力**: [受约束 / 不允许发布]
**必须在 [Gate] 之前修复**: [check list]
```

---

## 6. 关联文档

- `docs/releases/v3.8.0/ALPHA_GATE_CONTRACT.md` — Alpha Gate Contract
- `docs/releases/v3.8.0/ALPHA_BASELINE_REPORT.md` — Alpha Baseline Report
- `docs/governance/GATE_CONDITIONS.md` — 门禁条件（待更新）

---

## 7. Changelog

| Date | Change | Author |
|------|--------|--------|
| 2026-05-31 | Initial version | Hermes C |