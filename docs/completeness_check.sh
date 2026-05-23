#!/bin/bash
# docs/completeness_check.sh — 版本文档完整性检查
# 用法: bash docs/completeness_check.sh [version]
# 示例: bash docs/completeness_check.sh 1.2.0
# 必须输出: "N/21 PASS" 或 "N/21 FAIL"

set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 1.2.0"
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOC_DIR="$REPO_ROOT/docs/releases/v$VERSION"

# 21 个必需文档（与 VERSION_DOCS_SPEC.md 同步）
REQUIRED_DOCS=(
    "00-release-summary.md"
    "RELEASE_NOTES.md"
    "CHANGELOG.md"
    "DEVELOPMENT_PLAN.md"
    "TEST_PLAN.md"
    "RELEASE_GATE_CHECKLIST.md"
    "FEATURE_MATRIX.md"
    "PERFORMANCE_REPORT.md"
    "USER_MANUAL.md"
    "ARCHITECTURE_DECISIONS.md"
    "ARCHITECTURE_DEVIATION_REPORT.md"
    "COMPREHENSIVE_STATUS_REPORT.md"
    "DEPLOYMENT_GUIDE.md"
    "INCREMENTAL_UPDATE_GUIDE.md"
    "TEST_DESIGN.md"
    "EVALUATION_REPORT.md"
    "COVERAGE_REPORT.md"
    "INTEGRATION_STATUS.md"
    "ISSUE_TRACKER.md"
    "PROGRESS_REPORT.md"
    "FEATURE_TRACKING_MATRIX.md"
)

# 检查函数
check_docs() {
    local missing=0
    local missing_list=""

    echo "=== Document Completeness Check for v$VERSION ==="
    echo ""

    for doc in "${REQUIRED_DOCS[@]}"; do
        if [ ! -f "$DOC_DIR/$doc" ]; then
            echo "  MISSING: $doc"
            missing=$((missing + 1))
            missing_list="$missing_list $doc"
        else
            echo "  OK: $doc"
        fi
    done

    echo ""
    echo "=========================================="
    local total=${#REQUIRED_DOCS[@]}
    local found=$((total - missing))

    if [ $missing -eq 0 ]; then
        echo "  $found/$total PASS — All required documents present"
        return 0
    else
        echo "  $found/$total FAIL — $missing documents missing:$missing_list"
        return 1
    fi
}

# 版本一致性检查
check_version_consistency() {
    echo ""
    echo "=== Version Consistency Check ==="

    local wrong_version=0
    for doc in "$DOC_DIR"/*.md; do
        if [ -f "$doc" ]; then
            # 检查文档中是否出现旧版本号（排除当前版本）
            local doc_name=$(basename "$doc")
            local content=$(cat "$doc" 2>/dev/null || echo "")

            # 检查是否有版本号但与当前版本不匹配（简单检查）
            if echo "$content" | grep -qE "^> \*\*版本\*\*: v[0-9]" && ! echo "$content" | grep -qE "^> \*\*版本\*\*: v$VERSION"; then
                echo "  WARNING: $doc_name has different version marker"
                wrong_version=$((wrong_version + 1))
            fi
        fi
    done

    if [ $wrong_version -eq 0 ]; then
        echo "  Version consistency: PASS"
        return 0
    else
        echo "  Version consistency: $wrong_version warnings"
        return 1
    fi
}

# 主流程
check_docs
DOCS_RESULT=$?

check_version_consistency
VERSION_RESULT=$?

echo ""
echo "=========================================="
if [ $DOCS_RESULT -eq 0 ] && [ $VERSION_RESULT -eq 0 ]; then
    echo "  Overall: PASS"
    exit 0
else
    echo "  Overall: FAIL"
    exit 1
fi
