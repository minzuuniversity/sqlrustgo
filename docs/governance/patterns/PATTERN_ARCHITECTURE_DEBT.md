# PATTERN_ARCHITECTURE_DEBT.md

> **Pattern**: Architecture Debt Tracking  
> **Trigger**: Architecture violations detected but not formally tracked  
> **Source**: v3.7.0 GA — 10 architecture violations (AV-001~AV-010)  
> **Author**: Hermes Agent  
> **Date**: 2026-05-30  

---

## Context

Architecture debt differs from code debt:
- **Code debt**: Missing tests, style violations — can be measured by coverage/linter
- **Architecture debt**: Design violations that require architectural changes — cannot be measured by coverage

v3.7.0 has 10 architecture violations (7 CRITICAL + 3 HIGH) that passed all Gates.

---

## Trigger Conditions

This pattern is triggered when:

1. Architecture violation detected (AV-001~AV-010 in v3.7.0)
2. Architecture debt crosses multiple versions without resolution
3. Code quality Gates (A1-A5, B1-B8) PASS but architecture quality degrades
4. Execution engine grows from 4658→6829 lines (47% growth)

---

## Failure Mode

### Wrong Action: Ignore architecture debt until major release

```
❌ 错误做法:
1. 发现架构违规
2. "这是已知问题，等下个版本解决"
3. 继续添加新功能
4. 架构债务累积
5. 最终需要大规模重构
```

**Why it fails**: INT-1 跨越 7 个版本（v1.2.0~v3.7.0），因为一直没有系统性追踪。

### Wrong Action: Use normal Issue tracking for architecture debt

```
❌ 错误做法:
1. AV-001 标记为 P0
2. 放入正常 milestone
3. 期望在单版本解决
4. 无法解决，因为需要多 PR
```

**Why it fails**: 架构债务需要 PR DAG，不是单一 Issue。

---

## Correct Action

### Right Action: Architecture Debt Registry + PR DAG

```
✅ 正确做法:
1. 创建 ARCHITECTURE_VIOLATIONS.md 基线
2. 分类: CRITICAL / HIGH / MEDIUM
3. 冻结: 禁止新增 CRITICAL/HIGH
4. 追踪: 每周减少 2 个
5. PR DAG: 关联到 v3.8.0 PR-800~PR-900
6. 验收: integration test for architecture compliance
```

### Architecture Violation Template

```markdown
## Architecture Violation Entry

### ID
AV-{NNN}

### Rule
[F{n}-{SHORT_NAME}]

### File
[path/to/file.rs]

### Lines
[Line numbers]

### Description
[Brief description]

### Severity
[CRITICAL|HIGH|MEDIUM]

### Status
[Frozen|Work in Progress|Resolved]

### Resolution
[If resolved: PR number + commit]
[If pending: Planned PR]
```

---

## Example: INT-1 — DML 不经过 WAL

### Architecture Violation Details

```
AV-001: F1-DML_WITHOUT_TXN, trigger.rs, 427,505,507,529
AV-002: F1-DML_WITHOUT_TXN, harness.rs, 274,315,375
AV-003: F1-DML_WITHOUT_TXN, merge.rs, 88,107
AV-004: F1-DML_WITHOUT_TXN, parallel_vector_executor.rs, 689,706,724,743
AV-005: F1-DML_WITHOUT_TXN, parallel_executor.rs, 10处
AV-006: F1-DML_WITHOUT_TXN, local_executor.rs, 1054
AV-007: F1-DML_WITHOUT_TXN, vector_executor.rs, 193,220,242,293
```

### Root Cause

DML 操作直接调用 storage，跳过 TransactionManager + WAL。

### Resolution Plan

v3.8.0 PR-830~PR-840:
```
PR-830: WAL + WriteBuffer 接入
PR-840: DML Transaction Interception
```

---

## Prevention

1. **Baseline Freeze**: CRITICAL/HIGH 禁止新增
2. **Weekly Reduction**: 每周减少 2 个 architecture violations
3. **PR DAG Tracking**: 架构债务关联到 PR DAG
4. **Integration Gate**: 新增 Architecture Integration Gate
5. **Execution Engine Size Budget**: 主执行器行数预算（<1500 行）

---

## Related Patterns

- `PATTERN_LEGACY_RETIREMENT.md` — Legacy issue retirement
- `ADR-005-legacy-gate-retirement.md` — Legacy Gate Retirement ADR

---

## SSOT 引用

- `docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md` — 架构违规基线
- `docs/releases/v3.8.0/DEVELOPMENT_PLAN.md` — PR DAG
- `docs/governance/GATE_CONDITIONS.md` — Gate conditions