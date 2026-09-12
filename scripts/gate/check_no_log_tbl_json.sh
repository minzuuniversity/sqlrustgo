#!/usr/bin/env bash
# =============================================================================
# V400-00: File Governance Gate
# 
# 检查新提交是否引入 *.log, *.tbl, *.json 文件
# 
# 规则 (2026-09-08):
# - *.log, *.tbl, 大 *.json (>1MB) 禁止入仓
# - 例外: Cargo.toml, package.json, tsconfig.json, *.config.json, docs/governance/*.json
# =============================================================================
set -euo pipefail

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ALLOWLIST_PATTERNS=(
    "Cargo.toml"
    "package.json"
    "tsconfig.json"
    ".gitignore"
    ".gitattributes"
    "\.config\.json$"
    "docs/governance/.*\.json$"
    "docs/releases/.*/manifest\.json$"
    "docs/releases/.*/evidence/.*/summary\.json$"
    "\.claude/.*\.json$"
)

check_allowed() {
    local file="$1"
    for pattern in "${ALLOWLIST_PATTERNS[@]}"; do
        if [[ "$file" =~ $pattern ]]; then
            return 0
        fi
    done
    return 1
}

echo "=== V400-00 File Governance Gate ==="
echo "检查新提交是否引入禁止文件类型..."

# 获取当前 HEAD 和上一个 commit
if ! git rev-parse HEAD >/dev/null 2>&1; then
    echo -e "${YELLOW}警告: 不是 git 仓库，跳过检查${NC}"
    exit 0
fi

PREV_COMMIT="${1:-HEAD~1}"
CURRENT_COMMIT="${2:-HEAD}"

# 获取变更文件列表
CHANGED_FILES=$(git diff --name-only "$PREV_COMMIT".."$CURRENT_COMMIT" 2>/dev/null || echo "")

if [[ -z "$CHANGED_FILES" ]]; then
    echo -e "${GREEN}✓ 没有文件变更${NC}"
    exit 0
fi

VIOLATIONS=0
ALLOWED_JSON_COUNT=0
for file in $CHANGED_FILES; do
    # 检查文件大小 (超过 100MB)
    if [[ -f "$file" ]]; then
        SIZE=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
        if [[ "$SIZE" -gt 104857600 ]]; then
            echo -e "${RED}✗ 文件超过 100MB: $file ($(numfmt --to=iec "$SIZE"))${NC}"
            VIOLATIONS=$((VIOLATIONS + 1))
        fi
    fi
    
    # 检查文件类型
    case "$file" in
        *.log)
            echo -e "${RED}✗ 禁止 .log 文件: $file${NC}"
            VIOLATIONS=$((VIOLATIONS + 1))
            ;;
        *.tbl)
            echo -e "${RED}✗ 禁止 .tbl 文件: $file${NC}"
            VIOLATIONS=$((VIOLATIONS + 1))
            ;;
        *.json)
            if ! check_allowed "$file"; then
                echo -e "${RED}✗ 禁止 .json 文件: $file${NC}"
                VIOLATIONS=$((VIOLATIONS + 1))
            else
                ALLOWED_JSON_COUNT=$((ALLOWED_JSON_COUNT + 1))
            fi
            ;;
    esac
done

echo ""
echo "=== 检查结果 ==="
echo "变更文件总数: $(echo "$CHANGED_FILES" | wc -l)"
echo "允许的 .json 文件: $ALLOWED_JSON_COUNT"

if [[ "$VIOLATIONS" -gt 0 ]]; then
    echo -e "${RED}✗ 发现 $VIOLATIONS 个违规文件${NC}"
    echo ""
    echo "允许的 .json 模式:"
    for pattern in "${ALLOWLIST_PATTERNS[@]}"; do
        echo "  - $pattern"
    done
    exit 1
fi

echo -e "${GREEN}✓ 文件治理 gate PASS${NC}"
exit 0
