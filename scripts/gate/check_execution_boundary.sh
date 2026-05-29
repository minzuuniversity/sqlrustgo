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
echo "[3/3] Checking for legacy DML entry points (must use execute_dml_vtu)..."

LEGACY_FUNCS="execute_insert_sql\|execute_update_sql\|execute_delete_sql"
if grep -rn "$LEGACY_FUNCS" crates/executor/src/ 2>/dev/null | grep -v "_test\|#\[test\]"; then
    echo "❌ LEGACY DML ENTRY POINTS DETECTED"
    echo "All DML must use execute_dml_vtu()"
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
    echo "  ExecutionEngine.execute_dml_vtu()"
    echo ""
    echo "Forbidden patterns:"
    echo "  - storage.insert/update/delete outside VTU"
    echo "  - TransactionManager::begin/commit/rollback outside ExecutionEngine"
    echo "  - Legacy execute_insert_sql / execute_update_sql / execute_delete_sql"
    exit 1
fi