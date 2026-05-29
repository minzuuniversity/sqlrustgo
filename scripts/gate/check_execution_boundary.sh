#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

echo "=== Running Execution Boundary Check ==="
echo "Verifies all storage access goes through ExecutionEngine"
echo ""

ERRORS=0

echo "[1/2] Checking for direct storage access outside ExecutionEngine..."

# Find storage.* calls that are NOT inside execute_internal or tests
while IFS= read -r file; do
    if grep -n "storage\." "$file" 2>/dev/null | grep -v "execute_internal\|_test\|#\[test\]"; then
        echo "❌ DIRECT STORAGE ACCESS in $file"
        grep -n "storage\." "$file" | grep -v "execute_internal\|_test\|#\[test\]" | head -3
        echo ""
        ERRORS=$((ERRORS + 1))
    fi
done < <(find crates/executor/src -name "*.rs" -type f)

echo "[2/2] Checking for direct TransactionManager calls outside ExecutionEngine..."

# Find direct txn manager calls outside ExecutionEngine impl
if grep -rn "TransactionManager::" crates/executor/src/ 2>/dev/null | grep -v "impl.*ExecutionEngine\|_test\|#\[test\]"; then
    echo "❌ DIRECT TRANSACTION MANAGER ACCESS"
    ERRORS=$((ERRORS + 1))
fi

echo ""
if [ $ERRORS -eq 0 ]; then
    echo "✅ Execution boundary check passed!"
    echo "All storage access goes through ExecutionEngine"
    exit 0
else
    echo "❌ Found $ERRORS execution boundary violation(s)"
    echo ""
    echo "IMPORTANT: All storage access must go through:"
    echo "  ExecutionEngine.execute_internal()"
    echo ""
    echo "Forbidden patterns:"
    echo "  - storage.insert/update/delete outside execute_internal"
    echo "  - TransactionManager::begin/commit/rollback outside ExecutionEngine"
    exit 1
fi