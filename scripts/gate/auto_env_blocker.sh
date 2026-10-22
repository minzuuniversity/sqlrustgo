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
#   bash scripts/gate/auto_env_blocker.sh [--check] [version]
#   bash scripts/gate/auto_env_blocker.sh --check v4.1.0
#   bash scripts/gate/auto_env_blocker.sh v3.8.0
#   bash scripts/gate/auto_env_blocker.sh        # 默认扫描所有 v* 目录
#
# 模式:
#   (默认)   写入模式 —— 改写文档、插入标记。这是 **pre-commit 工具** 的语义。
#   --check  只读模式 —— **不写任何文件**, 只报告哪些文件缺少标记。
#             CI 必须用这个模式: 写模式会改写 runner 的工作树, 而 CI 里
#             改写的结果既不会被提交, 也会污染后续步骤读到的文件
#             (实测: v4.1.0 下有 92 个 .md 会被改写)。
#
# 退出码:
#   0  完成
#   2  用法错误 / 指定的版本目录不存在
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

MODE="write"
VERSION=""
while [ $# -gt 0 ]; do
    case "$1" in
        --check) MODE="check" ;;
        --write) MODE="write" ;;
        -h|--help)
            sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        -*) echo "auto_env_blocker: unknown option: $1" >&2; exit 2 ;;
        *)  VERSION="$1" ;;
    esac
    shift
done

MARKER="<!-- env:blocked:no-ci -->"
PROVENANCE_PATTERN="gate_policy_eval_id|policy_eval_id"

if [ -n "$VERSION" ]; then
    if [ ! -d "docs/releases/$VERSION" ]; then
        # #5446: the version comes from CI's detection step now. A missing
        # release directory must be visible, not silently treated as "nothing
        # to do" — the old code exited 0 and the step looked green while
        # checking a release line that does not exist.
        echo "auto_env_blocker: release directory not found: docs/releases/$VERSION" >&2
        exit 2
    fi
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
total_would_add=0

for dir in "${RELEASE_DIRS[@]}"; do
    echo "=== Processing $dir ==="
    while IFS= read -r -d '' md_file; do
        total_processed=$((total_processed + 1))
        # Read first 500 chars
        header=$(head -c 500 "$md_file")

        # Check if has env:blocked marker
        if echo "$header" | grep -qF "env:blocked:no-ci"; then
            total_skipped=$((total_skipped + 1))
            continue
        fi

        # Check if has gate_policy_eval_id (Gate Report provenance)
        if echo "$header" | grep -qE "$PROVENANCE_PATTERN"; then
            total_skipped=$((total_skipped + 1))
            continue
        fi

        if [ "$MODE" = "check" ]; then
            total_would_add=$((total_would_add + 1))
            echo "  WOULD_ADD: $md_file"
            continue
        fi

        # Insert env:blocked:no-ci after H1 heading
        # Find first H1 line, insert after it.
        #
        # #5403: the previous implementation used
        #   h1_line=$(grep -n "^# " "$md_file" | head -1 | cut -d: -f1)
        # Under `set -euo pipefail` that pipeline exits non-zero when the file
        # has **no** H1 heading (grep prints nothing and returns 1), so the
        # script aborted mid-way — the `if [ -z "$h1_line" ]` branch that was
        # supposed to handle "no H1" was unreachable dead code. awk always
        # returns 0 for a readable file and prints nothing when there is no
        # match, so the empty result now reaches the branch as intended.
        h1_line=$(awk '/^# /{print NR; exit}' "$md_file")
        if [ -z "$h1_line" ]; then
            # No H1, insert at top
            h1_line=0
        fi

        # Use awk to insert marker.
        #
        # `line == 0` means "no H1 in this file" → the marker goes in front of
        # the very first line. The old `NR == line` comparison could never
        # match NR against 0, so a heading-less file was reported as "size
        # didn't increase" and left unmarked. That path was unreachable
        # before #5403's pipefail fix, so the defect was invisible.
        tmpfile=$(mktemp)
        awk -v marker="$MARKER" -v line="$h1_line" '
            line == 0 && NR == 1 { print marker; print; next }
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
        total_added=$((total_added + 1))
        echo "  Added: $md_file"
    done < <(find "$dir" -name "*.md" -type f -print0)
done

echo ""
echo "=== Summary ==="
echo "Mode:         ${MODE}"
echo "Processed:    ${total_processed}"
if [ "$MODE" = "check" ]; then
    echo "Would add:   ${total_would_add} (read-only: no file was modified)"
    echo "Skipped:      ${total_skipped} (already has env:blocked or provenance)"
else
    echo "Added:        ${total_added}"
    echo "Skipped:      ${total_skipped} (already has env:blocked or provenance)"
fi
