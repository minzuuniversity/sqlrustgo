#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

echo "=== Running Mainline Path Check ==="
echo "Verifies DML operations go through Transaction + WAL"
echo ""

ERRORS=0

echo "[1/3] Checking executor/src/ for DML without txn/wal..."

# Find direct storage mutation calls in executor that don't go through txn/wal
while IFS= read -r file; do
    if grep -q "storage\.\(insert\|update\|delete\)" "$file" 2>/dev/null; then
        if ! grep -L "txn\|transaction\|wal" "$file" > /dev/null 2>&1; then
            echo "❌ F1: $file - DML without txn/wal"
            ERRORS=$((ERRORS + 1))
        fi
    fi
done < <(find crates/executor/src -name "*.rs" -type f)

echo "[2/3] Checking isolated modules not used in mainline..."

# Check that isolated modules aren't imported in production paths
ISOLATED=("parallel_executor" "vec_simd" "expr-legacy" "local_executor_dml")
for module in "${ISOLATED[@]}"; do
    if grep -r "use sqlrustgo_$module\|mod $module" crates/*/src/*.rs 2>/dev/null | grep -v "test\|tests" > /dev/null; then
        echo "❌ F2: Isolated module '$module' used in production"
        ERRORS=$((ERRORS + 1))
    fi
done

echo "[3/3] Checking WAL integration in executors..."

# Check that main executors have WAL
MAIN_EXECUTORS=(
    "crates/executor/src/transactional_executor.rs"
    "crates/executor/src/executor.rs"
)
for exec in "${MAIN_EXECUTORS[@]}"; do
    if [ -f "$exec" ]; then
        if ! grep -q "wal\|Wal\|WalStorage" "$exec" 2>/dev/null; then
            echo "❌ R1: $exec - missing WAL integration"
            ERRORS=$((ERRORS + 1))
        fi
    fi
done

echo ""
if [ $ERRORS -eq 0 ]; then
    echo "✅ Mainline path check passed!"
    echo "All DML goes through Transaction + WAL"
    exit 0
else
    echo "❌ Found $ERRORS mainline violation(s)"
    exit 1
fi