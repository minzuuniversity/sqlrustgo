#!/usr/bin/env bash
# auto_env_blocker.sh — SPEC-019 自动化检测 docs/releases/*/*.md 文档
# 自动加 env:blocked:no-ci 标记 (无 Gitea CI 时)
#
# 行为:
#   1. 扫描 docs/releases/v*/**/*.md
#   2. 对每个文档:
#      - 若 H1 标题后已有 env:blocked:no-ci → 跳过
#      - 若 H1 标题后已有 gate_policy_eval_id → 跳过 (有 provenance)
#      - 否则: 插入 <!-- env:blocked:no-ci -->
#
# Usage:
#   bash scripts/gate/auto_env_blocker.sh [version]
#   bash scripts/gate/auto_env_blocker.sh v3.8.0
#   bash scripts/gate/auto_env_blocker.sh        # 默认扫描所有 v* 目录
#
# 配合 pre-commit hook 自动化: 新 .md 文件 commit 前自动加 env:blocked

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

VERSION="${1:-}"
MARKER="<!-- env:blocked:no-ci -->"
PROVENANCE_PATTERN="gate_policy_eval_id|policy_eval_id"

if [ -n "$VERSION" ]; then
    RELEASE_DIRS=("docs/releases/$VERSION")
else
    # Find all v* directories
    RELEASE_DIRS=()
    for d in docs/releases/v*/; do
        if [ -d "$d" ]; then
            RELEASE_DIRS+=("$d")
        fi
    done
fi

if [ "${#RELEASE_DIRS[@]}" -eq 0 ]; then
    echo "No docs/releases/v*/ directories found"
    exit 0
fi

total_processed=0
total_added=0
total_skipped=0

for dir in "${RELEASE_DIRS[@]}"; do
    echo "=== Processing $dir ==="
    while IFS= read -r -d '' md_file; do
        ((total_processed++))
        # Read first 500 chars
        header=$(head -c 500 "$md_file")

        # Check if has env:blocked marker
        if echo "$header" | grep -qF "env:blocked:no-ci"; then
            ((total_skipped++))
            continue
        fi

        # Check if has gate_policy_eval_id (Gate Report provenance)
        if echo "$header" | grep -qE "$PROVENANCE_PATTERN"; then
            ((total_skipped++))
            continue
        fi

        # Insert env:blocked:no-ci after H1 heading
        # Find first H1 line, insert after it
        h1_line=$(grep -n "^# " "$md_file" | head -1 | cut -d: -f1)
        if [ -z "$h1_line" ]; then
            # No H1, insert at top
            h1_line=0
        fi

        # Use awk to insert marker
        tmpfile=$(mktemp)
        awk -v marker="$MARKER" -v line="$h1_line" '
            NR == line { print; print marker; next }
            { print }
        ' "$md_file" > "$tmpfile"

        # Verify content didn't change except insertion
        new_size=$(wc -c < "$tmpfile")
        old_size=$(wc -c < "$md_file")
        if [ "$new_size" -le "$old_size" ]; then
            echo "  ERROR: size didn't increase for $md_file (old=$old_size, new=$new_size)"
            rm -f "$tmpfile"
            continue
        fi

        mv "$tmpfile" "$md_file"
        ((total_added++))
        echo "  Added: $md_file"
    done < <(find "$dir" -name "*.md" -type f -print0)
done

echo ""
echo "=== Summary ==="
echo "Processed: $total_processed"
echo "Added:     $total_added"
echo "Skipped:   $total_skipped (already has env:blocked or provenance)"
