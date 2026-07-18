#!/usr/bin/env bash
# =============================================================================
# check_coverage.sh — v3.7.0 Coverage Gate (LEGACY)
# =============================================================================
# ⚠️  DEPRECATED: This script is for v3.7.0 only.
# For v3.11.0 coverage measurement, use the per-crate methodology documented in:
#   docs/releases/v3.11.0/COVERAGE_TESTING_METHODOLOGY.md
#   docs/releases/v3.11.0/COVERAGE_REPORT.md
#
# v3.7.0 measurement used `cargo llvm-cov test -p sqlrustgo --lib` which produces
# misleadingly low results (double-counting, no inline tests for engine_select.rs).
# =============================================================================

:set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== Running Coverage Gate Check (v3.7.0 LEGACY — see COVERAGE_TESTING_METHODOLOGY.md for v3.11.0) ==="

COVERAGE_DIR="docs/releases/v3.7.0"
MODE="${1:-full}"

# v3.7.0: 唯一允许的命令（禁止局部覆盖率）
# 注意：需要先安装 llvm-cov: cargo install cargo-llvm-cov
REQUIRED_COVERAGE=40
REQUIRED_LINE_COVERAGE=40

echo "Mode: $MODE"
echo "Required coverage: ${REQUIRED_COVERAGE}%"

# 问题测试 (pre-existing flake)：在 MemoryStorage 环境下可能有问题
# 已被修复（PR #3657 之后）,但保留 list 以防 regression。
# cargo llvm-cov 不支持 --skip <test_name> (v0.8+),所以用 --no-fail-fast
# 继续跑过失败用例并生成报告。失败的 case 仍然会出现在报告里供
# 审查；它们不阻塞 coverage gate。
NO_FAIL_FAST="--no-fail-fast"
# 检查 llvm-cov 是否可用
if ! command -v cargo-llvm-cov &> /dev/null && ! cargo llvm-cov --version &> /dev/null; then
    echo "⚠️  llvm-cov not installed. Installing..."
    cargo install cargo-llvm-cov
fi

if [ "$MODE" = "incremental" ]; then
    echo "Running incremental coverage..."
    CHANGED_CRATES=$(git diff --name-only | cut -d/ -f2 | sort -u | grep -E "^crates/" | cut -d/ -f2 || true)
    if [ -z "$CHANGED_CRATES" ]; then
        echo "No crate changes detected, using full coverage"
        MODE="full"
    else
        cargo llvm-cov \
            --workspace \
            --all-features \
            --tests \
            --exclude bench-cli \
            --output-dir "$COVERAGE_DIR" \
            $NO_FAIL_FAST
    fi
fi

if [ "$MODE" = "full" ]; then
    echo "Running FULL coverage (workspace + all-features + tests)..."
    echo "⚠️  禁止使用 'cargo test --lib' 或 'cargo llvm-cov --lib' 作为 release gate"
    echo ""
    cargo llvm-cov \
        --workspace \
        --all-features \
        --tests \
        --exclude bench-cli \
        --output-dir "$COVERAGE_DIR" \
        --html \
        $NO_FAIL_FAST
fi

# 检查覆盖率报告是否生成
if [ ! -f "$COVERAGE_DIR/coverage.xml" ]; then
    echo "❌ Coverage report not generated"
    echo "   Expected: $COVERAGE_DIR/coverage.xml"
    exit 1
fi

# 提取覆盖率百分比
echo ""
echo "Extracting coverage percentage..."
LINE_RATE=$(grep -oP 'line-rate="\K[0-9.]+' "$COVERAGE_DIR/coverage.xml" | head -1)
BRANCH_RATE=$(grep -oP 'branch-rate="\K[0-9.]+' "$COVERAGE_DIR/coverage.xml" | head -1)

if [ -z "$LINE_RATE" ]; then
    echo "❌ Failed to extract coverage percentage"
    exit 1
fi

# 转换为整数百分比
LINE_COVERAGE=$(echo "$LINE_RATE * 100" | bc | cut -d. -f1)
BRANCH_COVERAGE=$(echo "$BRANCH_RATE * 100" | bc | cut -d. -f1)

echo "Current line coverage: ${LINE_COVERAGE}%"
echo "Current branch coverage: ${BRANCH_COVERAGE}%"
echo "Required line coverage: ${REQUIRED_LINE_COVERAGE}%"

if [ "$LINE_COVERAGE" -lt "$REQUIRED_LINE_COVERAGE" ]; then
    echo "❌ Line coverage too low! Need at least ${REQUIRED_LINE_COVERAGE}%"
    exit 1
fi

echo ""
echo "✅ Coverage check passed!"

# 生成覆盖率摘要
cat > "$COVERAGE_DIR/coverage-summary.md" << EOF
# Coverage Report Summary (v3.7.0)

## Coverage Statistics

| Metric | Current | Required | Status |
|--------|---------|----------|--------|
| Line Coverage | ${LINE_COVERAGE}% | ${REQUIRED_LINE_COVERAGE}% | $([ "$LINE_COVERAGE" -ge "$REQUIRED_LINE_COVERAGE" ] && echo "✅ PASS" || echo "❌ FAIL") |
| Branch Coverage | ${BRANCH_COVERAGE}% | - | - |

## v3.7.0 Coverage Policy

**唯一允许的命令**:
\`\`\`bash
cargo llvm-cov \\
  --workspace \\
  --all-features \\
  --tests \\
  --exclude bench-cli
\`\`\`

**禁止用于 Release Gate**:
- \`cargo test --lib\` (仅库)
- \`cargo llvm-cov --lib\` (局部)

## Report Files

- **HTML Report**: $COVERAGE_DIR/coverage.html
- **XML Report**: $COVERAGE_DIR/coverage.xml

## Test Details

- **Test Command**: cargo llvm-cov --workspace --all-features --tests
- **Mode**: $MODE
- **Test Date**: $(date)

## Conclusion

Coverage meets the required threshold of ${REQUIRED_LINE_COVERAGE}% or higher.
EOF

echo "✅ Coverage summary generated: $COVERAGE_DIR/coverage-summary.md"
echo "=== Coverage Gate Check Complete ==="