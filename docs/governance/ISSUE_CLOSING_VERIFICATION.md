# GMP-Platform Issue 关闭验证规范

> **版本**: 1.0
> **日期**: 2026-05-24
> **目的**: 确保每个关闭的 Issue 都有 PR 证据，防止问题被虚假关闭
> **适用范围**: 所有 Issue 关闭操作

---

## 一、核心规则

### 1.1 禁止的模式

```
❌ 直接关闭 Issue（无 PR 证据）
❌ 手动标记为 closed 但未合并代码
❌ 用 comments 引用 PR 但未真正关联
```

### 1.2 正确的模式

```
✅ Issue 关闭前：验证关联 PR 已合并到目标分支
✅ 使用 Gitea API 验证 closed_by_pr 非空
✅ Issue body 中记录 PR 编号
✅ 合并后验证测试通过
```

---

## 二、Issue 关闭验证流程

### 2.1 关闭前检查清单

```bash
# Issue 关闭前必须执行
echo "=== Issue 关闭验证 ==="
echo ""

# 1. 检查 Issue 是否有 PR 引用
ISSUE_NUM=XXX
API_RESPONSE=$(curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues/${ISSUE_NUM}")

# 2. 验证 closed_by_pr 非空
CLOSED_BY_PR=$(echo "$API_RESPONSE" | grep -oP '"closed_by_pr":\K[^{]+' || echo "")

if [ -z "$CLOSED_BY_PR" ] || [ "$CLOSED_BY_PR" = "null" ]; then
    echo "❌ 禁止关闭: Issue #${ISSUE_NUM} 没有关联的已合并 PR"
    echo "   解决方案: 创建 PR → 合并 → 再关闭 Issue"
    exit 1
fi

echo "✅ Issue #${ISSUE_NUM} 验证通过: $CLOSED_BY_PR"
```

### 2.2 Issue 创建模板（包含 PR 引用字段）

创建 Issue 时，建议包含以下字段：

```markdown
## Issue 标题
[{标签}] {简短描述}

## Issue 内容
### 任务描述
{描述}

### 关联 PR
- 修复 PR: #{PR号} (待创建/已合并)

### 验收条件
- [ ] {条件 1}
- [ ] {条件 2}

### PR 合并证据
- PR #{PR号}: {合并日期}
- Commit: {commit hash}
```

---

## 三、Gitea API 验证

### 3.1 检查 Issue 是否有关联的已合并 PR

```bash
#!/bin/bash
# verify_issue_close.sh — Issue 关闭验证脚本
set -euo pipefail

ISSUE_NUM="${1:-}"

if [ -z "$ISSUE_NUM" ]; then
    echo "Usage: $0 <issue_number>"
    exit 1
fi

API_URL="http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues/${ISSUE_NUM}"
RESPONSE=$(curl -s "$API_URL")

# 解析 closed_by_pr
CLOSED_BY_PR=$(echo "$RESPONSE" | python3 -c "
import sys, json
data = json.load(sys.stdin)
pr = data.get('closed_by_pr', None)
if pr:
    print(f'{pr.get(\"merged_at\", \"\")}')
else:
    print('')
" 2>/dev/null || echo "")

STATE=$(echo "$RESPONSE" | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(data.get('state', 'unknown'))
" 2>/dev/null || echo "unknown")

echo "Issue #$ISSUE_NUM"
echo "  State: $STATE"
echo "  closed_by_pr: $CLOSED_BY_PR"

if [ "$STATE" = "closed" ] && [ -z "$CLOSED_BY_PR" ]; then
    echo ""
    echo "⚠️  WARNING: Issue #${ISSUE_NUM} is closed but has no PR evidence!"
    echo "   This violates the governance rules."
    exit 1
elif [ -n "$CLOSED_BY_PR" ]; then
    echo ""
    echo "✅ Issue #${ISSUE_NUM} is properly closed with PR evidence."
    exit 0
else
    echo ""
    echo "ℹ️  Issue #${ISSUE_NUM} is still open."
    exit 0
fi
```

### 3.2 批量检查脚本

```bash
#!/bin/bash
# batch_verify_issues.sh — 批量验证已关闭 Issue
set -euo pipefail

echo "=== 批量验证已关闭 Issue ==="
echo ""

CLOSED_ISSUES=$(curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues?state=closed&per_page=100" | \
    python3 -c "
import sys, json
issues = json.load(sys.stdin)
for issue in issues:
    print(f'{issue[\"number\"]}: {issue[\"state\"]}: {issue.get(\"closed_by_pr\", {})}')
" 2>/dev/null || echo "")

echo "$CLOSED_ISSUES" | while IFS=: read -r num state pr; do
    if [ "$state" = "closed" ] && [ -z "$pr" ]; then
        echo "❌ Issue #$num: closed without PR evidence"
    fi
done
```

---

## 四、违规处理

### 4.1 虚假关闭的Issue

如果发现 Issue 被虚假关闭（无 PR 证据）：

1. **重新打开 Issue**
2. **创建 Issue 追踪违规行为**
3. **在 gate_lifecycle_tracking.md §七记录**

### 4.2 防止虚假关闭

- **Agent**: 关闭 Issue 前必须运行验证脚本
- **Human**: PR 合并后，PR author 负责关闭对应的 Issue
- **禁止**: 任何人手动在 Gitea UI 直接关闭 Issue

---

## 五、相关文档

| 文档 | 作用 |
|------|------|
| `gate_lifecycle_tracking.md` | 门禁生命周期追踪 |
| `governance_self_improvement.md` | 治理自我进化机制 |
| `GATE_EXEMPTIONS.md` | 门禁豁免记录 |
| `gate_spec_v130.md` | v1.3.0 门禁规范 |

---

*本文档由 hermes agent 创建，基于 SQLRustGo governance 体系*
*最后更新: 2026-05-24*
