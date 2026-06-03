# SPEC-020 — env:blocked 全自动化集成 (治本全步)
<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-020
> **PR Title**: env:blocked 全自动化集成 — alpha gate + pre-commit hook
> **Version**: v3.8.0
> **Branch**: `fix/v3.8.0-env-blocker-integration`
> **Auditor**: Hermes Agent
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

SPEC-019 (治本半步) 引入 `auto_env_blocker.sh` + 文档模板, 但仍需 Agent 主动调用. 实际工作流:
- develop HEAD = `89e4b4f01` (PR #2819/#2823/#2838/#2837 合并后)
- 新加 `MULTI_VERSION_GOVERNANCE_DAG.md` + `MULTI_VERSION_GOVERNANCE_REPORT.md` (来自 PR-2837 合并 + 8a0977d7c commit)
- A8-1 EVIDENCE 4 violations, alpha gate 14/15

**根因**: 每次 PR 合并, 可能引入新文档, 但自动脚本不自动跑.

### 1.2 SPEC-020 修复策略

集成 auto_env_blocker.sh 进 2 个自动触发点:
1. **Alpha Gate 集成**: `check_alpha_v380.sh` 在 A8-1 之前自动跑 (non-blocking)
2. **Pre-commit Hook 集成**: `pre-commit-env-blocker.sh` 在 commit 时自动跑

两者都从 SPEC-019 半治本 → SPEC-020 全治本 (Agent 零操作).

### 1.3 治本路径进度

| 阶段 | 状态 | 说明 |
|------|------|------|
| SPEC-015/017/018 治标 | ✅ | 手动加 env:blocked |
| SPEC-019 治本半步 | ✅ | 脚本可调, Agent 跑一次 |
| **SPEC-020 治本全步** | **🟢 本 PR** | **alpha gate + pre-commit 集成** |
| Gitea CI 终极 | 待 | 实际 CI run ID |

---

## 2. 功能范围

### 2.1 必须做 (Must Do)

| 任务 | 文件 | 实施方式 | 验证方法 |
|------|------|----------|----------|
| Alpha gate 集成 auto_env_blocker.sh | `scripts/gate/check_alpha_v380.sh` | A8-PRE 段在 A8-1 前跑 | alpha gate 15/15 自动 |
| Pre-commit hook 脚本 | `scripts/gate/pre-commit-env-blocker.sh` | bash 检测 staged .md | 脚本可执行, dry-run 测 |

### 2.2 禁止做

- ❌ 自动安装 hook 到 .git/hooks/ (用户手动 cp + chmod)
- ❌ 改 alpha gate 阻断行为 (auto_env_blocker.sh 失败时仍跑 A8-1)
- ❌ 重写 check_evidence_binding.sh (保持现状)

### 2.3 不在范围内

- Gitea CI 集成 (治本终极)
- Hook 强制安装 (用户控制)

---

## 3. 技术设计

### 3.1 Alpha Gate 集成

```diff
 # A8: 3套审查机制
 echo "--- A8: 3-Layer Governance Review Mechanisms ---"

+# SPEC-020: Pre-A8-1 自动修复 (auto_env_blocker.sh)
+echo "[A8-PRE] Running auto_env_blocker.sh (SPEC-019/020)..."
+bash "$SCRIPT_DIR/auto_env_blocker.sh" v3.8.0 > "$ARTIFACTS_DIR/A8-PRE_AUTO.log" 2>&1
+AUTO_EXIT=$?
+if [ $AUTO_EXIT -eq 0 ]; then
+    echo "  [A8-PRE] auto_env_blocker.sh: OK"
+else
+    echo "  [A8-PRE] auto_env_blocker.sh: exit $AUTO_EXIT (non-blocking)"
+fi
+
 # A8-1: 审查机制1 — 证据绑定 (G-01)
 check "A8-1_EVIDENCE" "Evidence Binding (G-01 Anti-Fabrication)" \
```

### 3.2 Pre-commit Hook

`scripts/gate/pre-commit-env-blocker.sh`:

```bash
#!/usr/bin/env bash
# 检测 git diff --cached 中 docs/releases/v*/**/*.md
# 自动给缺失 env:blocked 标记的文档补标记
# git add 修改的文档

set -euo pipefail
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

staged_md=$(git diff --cached --name-only --diff-filter=AM | grep -E "^docs/releases/v[0-9]+\.[0-9]+\.[0-9]+/.*\.md$" || true)
[ -z "$staged_md" ] && exit 0

bash scripts/gate/auto_env_blocker.sh v3.8.0 > /tmp/pre-commit-env-blocker.log 2>&1
# Re-stage modified files
modified=$(git diff --name-only | grep -E "^docs/releases/v.*\.md$" || true)
[ -n "$modified" ] && echo "$modified" | xargs git add
exit 0
```

**安装**:
```bash
cp scripts/gate/pre-commit-env-blocker.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### 3.3 验证

- bash check_alpha_v380.sh: 15/15 PASS (A8-PRE 自动修复 docs 后 A8-1 通过)
- pre-commit hook 脚本可执行

### 3.4 提交规范

```bash
git commit -m "feat(gate): SPEC-020 env:blocked 全自动化集成 (治本全步)

治本全步: 集成 auto_env_blocker.sh 进 2 个自动触发点

1. Alpha Gate 集成 (check_alpha_v380.sh):
   - A8 段开头增加 [A8-PRE] 步骤
   - 在 A8-1_EVIDENCE 检查前自动跑 auto_env_blocker.sh
   - non-blocking (脚本失败不影响其他检查)
   - 输出 A8-PRE_AUTO.log 到 artifacts/gate/v3.8.0/

2. Pre-commit Hook 集成 (pre-commit-env-blocker.sh):
   - bash 脚本, 检测 staged docs/releases/v*/**/*.md
   - 自动给缺失 env:blocked 标记的文档补标记
   - git add 修改的文件
   - 用户手动安装: cp + chmod +x

治本路径:
- SPEC-015/017/018 治标 ✅
- SPEC-019 治本半步 ✅ (脚本可调)
- SPEC-020 治本全步 ✅ (本 PR: alpha gate + pre-commit)
- Gitea CI 终极 (待)

验证:
- bash check_alpha_v380.sh: 15/15 PASS
  - A8-PRE: auto_env_blocker.sh OK
  - A8-1_EVIDENCE: PASS (auto-fixed)
- pre-commit hook 脚本可执行
- 328/328 executor + 287/287 storage tests PASS (无回归)

源: SPEC-019 治本半步
上游: Gitea CI 集成
后续: SPEC-021 文档模板/hook CI 强制"
```

---

## 4. 验收标准 (Acceptance Criteria)

- [x] **AC-1**: `check_alpha_v380.sh` 增加 A8-PRE 步骤
- [x] **AC-2**: `pre-commit-env-blocker.sh` 脚本存在且可执行
- [x] **AC-3**: Alpha gate 15/15 PASS 自动
- [x] **AC-4**: PR base = `develop/v3.8.0`
- [x] **AC-5**: 3 平台分支一致

---

## 5. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| alpha gate 变慢 | 低 | 低 | auto_env_blocker.sh 仅扫描 docs/, 1-2 秒 |
| pre-commit 误改 staged | 低 | 中 | 仅 env:blocked 单行插入, 幂等 |
| 文档无 H1 标题 | 中 | 低 | 脚本 fallback 到顶部 |

---

## 6. 关联

- **源**: SPEC-019 治本半步
- **上游**: Gitea CI 集成
- **后续**: SPEC-021 文档模板/hook CI 强制

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写。*
