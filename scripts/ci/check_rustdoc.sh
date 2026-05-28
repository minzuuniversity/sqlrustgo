#!/bin/bash
# scripts/ci/check_rustdoc.sh - Rustdoc CI Check
# 验证代码文档可生成，无警告
set -euo pipefail

echo "=== Rustdoc CI Check ==="
echo "Branch: ${GITHUB_REF:-$(git rev-parse --abbrev-ref HEAD)}"
echo "Commit: ${GITHUB_SHA:-$(git rev-parse HEAD)}"
echo ""

# Check if rustdoc is available
if ! command -v cargo &> /dev/null; then
    echo "ERROR: cargo not found"
    exit 1
fi

# Get version
cargo version

echo ""
echo "=== Running cargo doc --all-features --no-deps ==="

# Generate documentation with all features
# --no-deps: don't document dependencies
# --document-private-items: document private items (from .cargo/config.toml)
cargo doc --all-features --no-deps 2>&1

DOC_STATUS=$?

if [ $DOC_STATUS -eq 0 ]; then
    echo ""
    echo "=== Rustdoc check PASSED ==="
    echo "Documentation generated successfully."
    echo ""
    echo "Output location: target/doc/"
    echo "To view locally: cargo doc --open"
else
    echo ""
    echo "=== Rustdoc check FAILED ==="
    echo "cargo doc exited with status $DOC_STATUS"
    exit $DOC_STATUS
fi