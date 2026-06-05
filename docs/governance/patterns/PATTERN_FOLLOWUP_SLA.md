# PATTERN_FOLLOWUP_SLA.md

> **Pattern**: Follow-up Issue 必须包含 SLA 字段 (owner, target_release, review_date, blocking)
> **Trigger**: 任何"1 周+ 工作量"或"v<X.Y.Z+1> 修复"的 follow-up Issue
> **Source**: v3.8.0 GA 治理示范评审 (2026-06-05)
> **Author**: Hermes Agent
> **Status**: ACTIVE
> **Cross-references**: `ADR-011-debt-state-machine.md`, `debt-registry.yaml`, `GA_GOVERNANCE_DEMO_v3.8.0.md`

---

## Context

v3.8.0 GA 创建 3 个 follow-up Issue (#3117, #3129, #3136) + 1 个 INT-3/2 follow-up (#3146) = 共 4 个 follow-up。**全部缺乏 SLA 字段**，导致：

1. **永久 follow-up 风险**: 没有 owner / target_release，没人主动跟进
2. **门禁无法自动化**: 不知道哪些 follow-up 已超期
3. **跨版本债务追踪盲区**: 旧 issue 沉入 backlog 失访

本 pattern 强制所有 follow-up Issue 包含 4 字段（owner / target_release / review_date / blocking）。

---

## 4 强制 SLA 字段

### 4.1 owner

- **类型**: 字符串 (个人 or 团队)
- **格式**: GitHub/Gitea username 或团队名
- **示例**: `openclaw`, `core-executor-team`
- **缺失后果**: 标记为 OPEN (未分配, 不进入 IN_PROGRESS)
- **必填状态**: 任何非 OPEN 状态

### 4.2 target_release

- **类型**: SemVer 版本字符串
- **格式**: `v<X.Y.Z>` 或 `v<X.Y.Z+1>`
- **示例**: `v3.9.0`, `v4.0.0`
- **缺失后果**: 标记为 BLOCKED (无明确目标, 无法排期)
- **必填状态**: IN_PROGRESS, BLOCKED, VERIFIED

### 4.3 review_date

- **类型**: ISO 8601 日期
- **格式**: `YYYY-MM-DD`
- **示例**: `2026-09-01`
- **缺失后果**: 季度审查 (90 天后) 自动提醒
- **必填状态**: IN_PROGRESS, BLOCKED
- **推荐间隔**: 90 天 (季度审查)

### 4.4 blocking (数组)

- **类型**: 字符串数组
- **格式**: 依赖项 ID + 简短描述
- **示例**:
  ```yaml
  blocking:
    - "MemoryStorage::in_transaction stub 修复"
    - "INT-2 主路径集成"
  ```
- **缺失后果**: OPEN (无依赖可识别)
- **必填状态**: BLOCKED

---

## Issue Body 模板 (Follow-up)

```markdown
## 来源

#<parent_issue> (<audit_report> 报告 §<N>)

## 类型

🟡/🔴 Follow-up of #<parent_issue>

## 修复方向 (具体)

1. [步骤 1]
2. [步骤 2]
3. [步骤 3]

## 估计时间

[N hours/days/weeks]

## SLA 字段 (强制)

| 字段 | 值 |
|------|-----|
| **owner** | `<username>` |
| **target_release** | `v<X.Y.Z>` |
| **review_date** | `<YYYY-MM-DD>` |
| **blocking** | [依赖 1, 依赖 2, ...] |

## 当前真实状态

[调查发现的实际状态, 含文件:行号 / commit]

## 关联

- 父 Issue: #<parent_issue>
- 审计 PR: #<audit_pr>
- 债务注册表: `docs/governance/debt/debt-registry.yaml`
```

---

## 验证 (手动 / 自动化)

### 手动验证 (Issue 创建时)

```bash
# 验证 issue body 包含 4 字段
for field in owner target_release review_date blocking; do
    grep -q "$field" ISSUE_BODY.md || echo "MISSING: $field"
done
```

### 自动化 (v3.9.0 gate)

```bash
# scripts/gate/check_followup_sla.sh
for issue in 3117 3129 3136 3146; do
    body=$(curl -s "<api>/issues/$issue" | jq -r .body)
    for field in owner target_release review_date blocking; do
        echo "$body" | grep -q "$field" || echo "❌ #$issue missing $field"
    done
done
```

---

## 失败模式

### Wrong Action 1: Follow-up Issue 缺 SLA 字段

```
❌ 错误做法:
Title: "修 #3109 ARCH-3 VTU 集成"
Body: "v3.9.0 修复, 2-3 周"
```

**Why it fails**:
- 不知道谁负责
- 不知道何时完成
- 不知道依赖什么
- 季度审查时无法追踪

**Correct**:
```
✅ Title: "🔴 follow-up of #3109: ARCH-3 VTU 主路径集成 (2-3 周)"
✅ Body SLA:
  | owner | openclaw |
  | target_release | v3.9.0 |
  | review_date | 2026-09-01 |
  | blocking | [MemoryStorage::in_transaction, INT-2 主路径集成] |
```

### Wrong Action 2: review_date 设太远 (> 6 月)

```
❌ 错误做法: review_date = 2027-01-01 (1 年后)
```

**Why it fails**:
- 季度审查漏过
- 阻塞关系失访
- "永久 follow-up" 风险

**Correct**: review_date ≤ 3 月后 (季度审查)

### Wrong Action 3: blocking 列表空白

```
❌ 错误做法: blocking: [] (但实际依赖其他债务)
```

**Why it fails**:
- 不知道并行机会
- 排期冲突无法识别

**Correct**: 即使"无依赖", 也写 `blocking: [v3.9.0 GA 前准备]`

---

## 应用示例 (v3.8.0 GA)

### #3117: openclaw_endpoints DML bypass VtuGuard 迁移

```markdown
| 字段 | 值 |
|------|-----|
| owner | openclaw |
| target_release | v3.9.0 |
| review_date | 2026-09-01 |
| blocking | ["ARCH-3 VTU 主路径集成 (优先)"] |
| estimated_effort | "1-2 weeks" |
```

### #3129: ARCH-3 修复路径

```markdown
| 字段 | 值 |
|------|-----|
| owner | openclaw |
| target_release | v3.9.0 |
| review_date | 2026-09-01 |
| blocking | ["MemoryStorage::in_transaction stub (INT-3 修复需要)"] |
| estimated_effort | "2-3 weeks" |
```

### #3136: check_cross_version_debt.sh 1 周全量升级

```markdown
| 字段 | 值 |
|------|-----|
| owner | openclaw |
| target_release | v3.9.0 |
| review_date | 2026-09-01 |
| blocking | [] |
| estimated_effort | "1 week" |
```

### #3146: INT-3/2 集成

```markdown
| 字段 | 值 |
|------|-----|
| owner | openclaw |
| target_release | v3.9.0 |
| review_date | 2026-09-01 |
| blocking | ["MemoryStorage::in_transaction stub (INT-3 修复需要)"] |
| estimated_effort | "2 weeks total" |
```

---

## 维护

| 项目 | 值 |
|------|-----|
| 文档版本 | PATTERN_FOLLOWUP_SLA-1.0 |
| 最后更新 | 2026-06-05 |
| 维护者 | Hermes Agent |
| 状态 | ACTIVE |
| 关联 | ADR-011-debt-state-machine.md, debt-registry.yaml, GA_GOVERNANCE_DEMO_v3.8.0.md |
