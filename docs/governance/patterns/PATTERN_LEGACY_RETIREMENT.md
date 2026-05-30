# PATTERN_LEGACY_RETIREMENT.md

> **Pattern**: Legacy Issue Retirement  
> **Trigger**: Issue exists across multiple versions without resolution  
> **Source**: v3.7.0 GA — INT-1~INT-4 legacy issues (2026-05-30)  
> **Author**: Hermes Agent  
> **Date**: 2026-05-30  

---

## Context

Legacy Issues are Issues that:
1. Exist across multiple versions (v1.2.0~v3.7.0 = 7 versions for INT-1)
2. Involve architecture changes requiring multiple PRs
3. Cannot be resolved in a single release cycle

In v3.7.0, INT-1 (DML 不经过 WAL) was first recorded as a formal issue but existed since v1.2.0.

---

## Trigger Conditions

This pattern is triggered when:

1. An Issue exists across 3+ versions without resolution
2. An Issue requires multiple PRs to resolve (PR-830~PR-840 for INT-1)
3. An Issue involves core architecture changes (AV-001~AV-010)
4. An Issue is closed but reopened as "legacy" in next version

---

## Failure Mode

### Wrong Action: Close Issue without resolution

```
❌ 错误做法:
1. Issue Created
2. 无法在当前版本解决
3. 直接 Close (不标注 LEGACY)
4. 下个版本重新创建相同的 Issue
```

**Why it fails**: Issue 跨版本追踪断裂，无法追溯历史。

### Wrong Action: Mark as P0/P1 without LEGACY designation

```
❌ 错误做法:
1. INT-1 标记为 P0
2. 放入当前版本 milestone
3. 无法按时完成
4. 版本延期或 Issue 状态混乱
```

**Why it fails**: P0/P1 是紧急度标记，不是 Legacy 标记。

---

## Correct Action

### Right Action: Mark as LEGACY with planned resolution

```
✅ 正确做法:
1. Issue Created
2. 评估: 跨版本 + 需要多 PR → 标记为 LEGACY
3. 分配到正确版本的 milestone (v3.8.0，不是 v3.7.0)
4. 指定 PR DAG (PR-830~PR-840)
5. 记录: FIRST_APPEARED, AFFECTED_VERSIONS, ROOT_CAUSE
6. LEGACY Issue 可以在 GA 后继续处理
```

### LEGACY Issue Template

```markdown
# [LEGACY] Issue Title

> **Status**: LEGACY
> **First Appeared**: v{VERSION}
> **Affected Versions**: v{START} ~ v{END} ({N} versions)
> **Root Cause**: [Brief description]
> **Planned Resolution**: v{NEXT}.x PR-{NN0}~PR-{NN0}
> **PR DAG**: [PR-800 → PR-810 → ...]

## Description

[Detailed description of the issue]

## Resolution Plan

### Phase 0 — Architecture Freeze
PR: PR-800, PR-810
Deliverables: [...]

### Phase 1 — Transaction Core
PR: PR-820, PR-830, PR-840
Deliverables: [...]
```

---

## Example: INT-1 — DML 不经过 WAL

### Issue History

| 版本 | 状态 | 说明 |
|------|------|------|
| v1.2.0 | 首次发现 | DML 直接调用 storage |
| v3.7.0 | 正式记录 | ARCHITECTURE_VIOLATIONS.md |
| v3.7.0 | 延期 v3.8.0 | PR-830~PR-840 |
| v3.8.0 | 计划解决 | PR DAG: PR-800~PR-900 |

### Resolution Plan

```
PR-800: COM_QUERY AST Routing
PR-810: ExecutionEngine → Router
PR-820: TransactionManager Session Binding
PR-830: WAL + WriteBuffer 接入
PR-840: DML Transaction Interception
```

---

## Prevention

1. **ADR-005**: Legacy Issue 必须使用 `LEGACY` 标记
2. **跨版本追踪**: LEGACY Issue 必须记录 FIRST_APPEARED 和 AFFECTED_VERSIONS
3. **PR DAG**: LEGACY Issue 必须关联 PR DAG
4. **GA 后处理**: LEGACY Issue 可以在 GA 后继续处理，不算违规

---

## Related Patterns

- `PATTERN_ARCHITECTURE_DEBT.md` — Architecture debt tracking
- `ADR-005-legacy-gate-retirement.md` — Legacy Gate Retirement ADR

---

## SSOT 引用

- `docs/governance/adr/ADR-005-legacy-gate-retirement.md` — Legacy Issue 规范
- `docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md` — 架构违规基线
- `docs/releases/v3.8.0/DEVELOPMENT_PLAN.md` — PR DAG