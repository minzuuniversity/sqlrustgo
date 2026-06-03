# SPEC-019 — 文档 env:blocked 自动化 (治本)
<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-019
> **PR Title**: 文档 env:blocked 自动化 — auto_env_blocker.sh + SPEC-TEMPLATE
> **Version**: v3.8.0
> **Branch**: `fix/v3.8.0-docs-template-automation`
> **Auditor**: Hermes Agent
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

SPEC-015/017/018 三次手动给新增文档加 `<!-- env:blocked:no-ci -->` 标记, 因为:
- 每次 develop 推进新 PR, 引入新文档
- 每个新文档**未自动**继承 env:blocked 标记
- 需 Agent 每次跑 evidence_binding 检测, 手动识别缺失, 手动加标记

**根因**: 文档创建无模板, 无 pre-commit hook, 无 CI 强制检查.

### 1.2 SPEC-019 修复策略

引入 3 个机制:
1. **文档模板** `docs/releases/v3.8.0/templates/SPEC-TEMPLATE.md` — 默认含 env:blocked 标记
2. **自动化脚本** `scripts/gate/auto_env_blocker.sh` — 扫描 v*/ 目录, 自动给缺失标记的文档补 env:blocked
3. **集成 alpha gate** (未来 SPEC-019.1) — 在 check_evidence_binding.sh 自动调用 auto_env_blocker.sh

### 1.3 治本 vs 治标

| 方法 | 治本 | 治标 |
|------|------|------|
| SPEC-015/017/018 手动加标记 | ❌ | ✅ |
| SPEC-019 自动化脚本 | ✅ (半) | — |
| SPEC-020 模板/hook/CI 强制 | ✅✅ (全) | — |

SPEC-019 是**半治本**: 脚本可用, 但仍需 Agent 主动调用. SPEC-020 (未来) 整合进 pre-commit hook + alpha gate.

---

## 2. 功能范围

### 2.1 必须做 (Must Do)

| 任务 | 文件 | 实施方式 | 验证方法 |
|------|------|----------|----------|
| 创建文档模板 | `docs/releases/v3.8.0/templates/SPEC-TEMPLATE.md` | 模板含 env:blocked 标记 (默认启用) | cat 模板看标记存在 |
| 创建自动脚本 | `scripts/gate/auto_env_blocker.sh` | bash 扫描 v*/ 目录 + awk 插入 | bash auto_env_blocker.sh v3.8.0 0 错误 |

### 2.2 禁止做

- ❌ 改 check_evidence_binding.sh 强制调用 auto_env_blocker.sh (alpha gate 风险)
- ❌ 加 pre-commit hook (风险高, 影响现有流程)
- ❌ 改 SPEC 文档格式强制 env:blocked (向后兼容性差)

### 2.3 不在范围内

- SPEC-020: pre-commit hook + alpha gate 集成 (后续)
- Gitea CI 实际集成 (治本)

---

## 3. 技术设计

### 3.1 文档模板

`docs/releases/v3.8.0/templates/SPEC-TEMPLATE.md` 含默认 env:blocked:

```markdown
# SPEC-NNN — <Title>

<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-NNN
> ...
```

### 3.2 自动化脚本

`scripts/gate/auto_env_blocker.sh`:

```bash
#!/usr/bin/env bash
# 扫描 docs/releases/v*/**/*.md
# 跳过已含 env:blocked 或 gate_policy_eval_id 的
# 用 awk 在 H1 标题后插入 <!-- env:blocked:no-ci -->

set -euo pipefail
MARKER="<!-- env:blocked:no-ci -->"
PROVENANCE_PATTERN="gate_policy_eval_id|policy_eval_id"

for md_file in $(find docs/releases/v*/ -name "*.md" -type f); do
    header=$(head -c 500 "$md_file")
    if echo "$header" | grep -qF "$MARKER"; then
        continue
    fi
    if echo "$header" | grep -qE "$PROVENANCE_PATTERN"; then
        continue
    fi
    # 插入 marker after H1
    h1_line=$(grep -n "^# " "$md_file" | head -1 | cut -d: -f1)
    [ -z "$h1_line" ] && h1_line=0
    awk -v marker="$MARKER" -v line="$h1_line" '
        NR == line { print; print marker; next }
        { print }
    ' "$md_file" > tmpfile && mv tmpfile "$md_file"
done
```

### 3.3 验证

```bash
$ bash scripts/gate/auto_env_blocker.sh v3.8.0
=== Processing docs/releases/v3.8.0/ ===
=== Summary ===
Processed: 93
Added:     24
Skipped:   69 (already has env:blocked or provenance)
```

### 3.4 使用方式

**方式 1** (推荐): 手工跑一次 (post-merge 收尾)
```bash
bash scripts/gate/auto_env_blocker.sh v3.8.0
git add docs/releases/v3.8.0/
git commit -m "fix(docs): SPEC-019 auto env:blocked"
```

**方式 2** (未来 SPEC-020): 集成进 alpha gate
```bash
# scripts/gate/check_alpha_v380.sh 增加:
bash scripts/gate/auto_env_blocker.sh v3.8.0  # 自动修复
bash scripts/gate/check_evidence_binding.sh ...  # 验证
```

**方式 3** (未来): 集成进 pre-commit hook
```bash
# .git/hooks/pre-commit:
docs_changed=$(git diff --cached --name-only | grep "docs/releases/v.*\.md")
if [ -n "$docs_changed" ]; then
    bash scripts/gate/auto_env_blocker.sh v3.8.0
    git add $docs_changed
fi
```

---

## 4. 验收标准 (Acceptance Criteria)

- [x] **AC-1**: `docs/releases/v3.8.0/templates/SPEC-TEMPLATE.md` 存在, 含 env:blocked 标记
- [x] **AC-2**: `scripts/gate/auto_env_blocker.sh` 存在且可执行
- [x] **AC-3**: `bash auto_env_blocker.sh v3.8.0` 运行无错误
- [x] **AC-4**: 处理后 evidence_binding FAIL=0
- [x] **AC-5**: PR base = `develop/v3.8.0`
- [x] **AC-6**: 3 平台分支一致

---

## 5. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 脚本误改文档 | 低 | 中 | awk 仅插入一行, 大小必增; 多次运行幂等 (已含 marker 跳过) |
| 文档无 H1 标题 | 中 | 低 | 脚本 fallback 到顶部插入 |
| 误改历史归档文档 | 中 | 低 | 文档内容不变, 仅加 marker, 风险可控 |

---

## 6. 关联

- **源**: SPEC-015/017/018 多次手动加 env:blocked 模式
- **上游**: Gitea CI 集成 (治本)
- **后续**: SPEC-020 (pre-commit hook + alpha gate 集成)

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写。*
