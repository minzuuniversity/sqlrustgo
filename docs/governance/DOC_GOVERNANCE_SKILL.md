---
name: doc-governance
description: "Use when modifying docs/ directory files - enforces DOC_CHECK_CORRECTION_RULES.md 5-step workflow: discover issues → write plan → execute → verify → report. MANDATORY for any doc changes."
---

# Document Check and Correction Workflow

## Overview

This skill enforces a **5-step standardized workflow** for any documentation changes in the `docs/` directory. It prevents AI from fabricating evidence, over-modifying content, or skipping verification.

## When to Invoke

**MANDATORY** when user requests:
- "fix doc errors"
- "update docs"
- "correct version"
- "modify documentation"
- Any work on `docs/releases/v*/` or `docs/README.md`

## 5-Step Workflow

### Step 1: Discover Issues

1. Read target documents
2. Cross-reference version numbers/dates/status with related documents
3. List all issues with: file, line number, error content, evidence

**Evidence types allowed**:
- `git log` / git commit hash
- CHANGELOG.md header (version/date/status)
- GA_GATE_REPORT.md existence
- File system verification (`ls`)

**Output format**:

| # | File | Issue | Location | Evidence |
|---|------|-------|----------|----------|
| 1 | v3.7.0/CHANGELOG.md | Duplicate commit `b925f438` | Line 27-28 | Same commit appears twice |

### Step 2: Write Plan and Checklist

Write the **Correction Plan** and **Verification Checklist** before any edits.

**Correction Plan** format:

| # | Operation | File | Description |
|---|-----------|------|-------------|
| 1 | Delete duplicate line | v3.7.0/CHANGELOG.md | Remove line 28 with duplicate `b925f438` |
| 2 | Add version entry | v3.7.0/CHANGELOG.md | Insert v3.7.0 row before v3.6.0 |

**Verification Checklist** (execute after Step 3):

- [ ] Issue 1 fixed
- [ ] Issue 2 fixed
- [ ] No over-modification (commit logs, function descriptions unchanged)
- [ ] All referenced files exist (`ls` verification)
- [ ] git diff is clean (no unintended changes)
- [ ] New files added with `git add`

### Step 3: Execute Plan

1. Execute each edit using `edit` tool
2. Record git diff after each edit
3. For new files, run `git add` immediately after creation
4. **Minimum modification principle**: Only fix factual errors

**Allowed modifications**:
- Version numbers
- Dates
- Status markers (Alpha/Beta/GA)
- Duplicate entries
- Missing entries in version history

**Forbidden modifications**:
- Commit log content
- Function descriptions
- Technical architecture content
- Any substantive content

### Step 4: Verify with Checklist

1. Execute each verification item
2. Record results in work record
3. If any check fails → revert and re-execute Step 3

### Step 5: Output Work Report

Create `docs/governance/DOC_CHECK_CORRECTION_WORK_RECORD.md`:

```markdown
# 文档检查和纠正工作报告

## 一、基本信息
- 工作时间：
- 执行人：
- 工作范围：

## 二、发现的问题
[Issue table from Step 1]

## 三、执行的操作
[Operation table from Step 2]

## 四、复核检查结果
[Checklist results]

## 五、待提交文件状态
[git status output]

## 六、结论
```

---

## Anti-Patterns (BLOCKING)

| Anti-pattern | Correct behavior |
|-------------|------------------|
| "This doc just needs a quick fix" | Still follow all 5 steps |
| Skipping checklist verification | Must verify each item |
| "I'll update the content" | Only update status/version/date |
| "I'll delete the outdated parts" | Never delete original records |
| "I verified it in my head" | Must show `ls` / `git diff` evidence |

---

## Git Operation Safety

Before any doc modification:
```bash
# Create restore point
git stash push -m "doc-fix-backup-$(date +%Y%m%d%H%M%S)"
# OR ensure changes are staged before editing
git add <file>  # stage first for safety
```

After modification:
```bash
# Verify no unintended changes
git diff --stat
# Verify all referenced files exist
ls docs/releases/v3.7.0/*.md | wc -l
```

---

## Related Files

| File | Purpose |
|------|---------|
| `docs/governance/DOC_CHECK_CORRECTION_RULES.md` | Full rules and checklist template |
| `docs/governance/DOC_CHECK_CORRECTION_WORK_RECORD.md` | Work record template |
| `scripts/gate/check_docs_consistency.sh` | CI gate for doc consistency |

---

## CI Gate: check_docs_consistency.sh

This skill integrates with `scripts/gate/check_docs_consistency.sh` which validates:
1. Version history table completeness
2. Status consistency across CHANGELOG/VERSION_HISTORY/README
3. No duplicate commits in CHANGELOG.md
4. Required README.md exists for each version

**Run after Step 4**:
```bash
bash scripts/gate/check_docs_consistency.sh
```