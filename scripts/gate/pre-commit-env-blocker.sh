#!/usr/bin/env bash
# STATUS: DEPRECATED — see scripts/gate/README.md
# Reason: not invoked by any active gate (audit 2026-06-04)
# Action:  do not add new callers; restore via git history if needed

# pre-commit hook — SPEC-020 自动化 docs env:blocked
#
# 安装: cp scripts/gate/pre-commit-env-blocker .git/hooks/pre-commit
#       chmod +x .git/hooks/pre-commit
#
# 行为:
#   - 检测 git diff --cached 中 docs/releases/v*/**/*.md 文件
#   - 自动给缺失 env:blocked 标记的文档补标记
#   - git add 修改的文档

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Detect staged .md files in docs/releases/v*/
staged_md=$(git diff --cached --name-only --diff-filter=AM | grep -E "^docs/releases/v[0-9]+\.[0-9]+\.[0-9]+/.*\.md$" || true)

if [ -z "$staged_md" ]; then
    exit 0  # No staged v*/docs, skip
fi

# Run auto_env_blocker.sh (dry-run mode: only print what would change)
echo "[pre-commit] Checking env:blocked markers on staged docs..."
bash scripts/gate/auto_env_blocker.sh v3.8.0 > /tmp/pre-commit-env-blocker.log 2>&1
AUTO_EXIT=$?

# Check if any staged files were modified by auto_env_blocker.sh
modified=$(echo "$staged_md" | while read -r f; do
    if git diff --name-only | grep -qF "$f"; then
        echo "$f"
    fi
done)

if [ -n "$modified" ]; then
    echo "[pre-commit] Adding env:blocked markers to:"
    echo "$modified" | sed 's/^/  /'
    echo "$modified" | xargs git add
fi

if [ $AUTO_EXIT -ne 0 ]; then
    echo "[pre-commit] WARNING: auto_env_blocker.sh exited $AUTO_EXIT (non-blocking)"
fi

exit 0
