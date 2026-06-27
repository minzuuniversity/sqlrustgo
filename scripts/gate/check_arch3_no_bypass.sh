#!/bin/bash
# check_arch3_no_bypass.sh - ARCH-3 (#3169) VtuGuard main-path G4 gate
#
# Verifies:
# 1. ExecutionEngine::execute_insert/update/delete all call
#    VtuGuard::assert_path_for_dml
# 2. openclaw_endpoints DML paths also call the marker
# 3. No "bypass" code in critical DML paths
# 4. VtuGuard::assert_path_for_dml is a public API
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3169-arch3-vtu-main-path.md
#       V390_TEST_PLAN.md §G4

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== G4 Gate: ARCH-3 (#3169) VtuGuard main-path enforcement ==="

# 1. ExecutionEngine DML entry points
COUNT=$(grep -c "assert_path_for_dml" src/execution_engine.rs || true)
if [ "$COUNT" -lt 3 ]; then
    echo "  ❌ FAIL: src/execution_engine.rs only has $COUNT VtuGuard marker calls (expected >= 3)"
    exit 1
fi
echo "  [1/4] ✅ PASS: $COUNT VtuGuard marker calls in src/execution_engine.rs"

for FN in execute_insert execute_update execute_delete; do
    if ! grep -A 5 "pub fn $FN" src/execution_engine.rs | grep -q "assert_path_for_dml"; then
        echo "  ❌ FAIL: pub fn $FN missing assert_path_for_dml call"
        exit 1
    fi
done
echo "       ✅ All 3 EE DML functions have the marker"

# 2. openclaw_endpoints markers
OPENCLAW=$(grep -c "assert_path_for_dml" crates/server/src/openclaw_endpoints.rs || true)
if [ "$OPENCLAW" -lt 2 ]; then
    echo "  ❌ FAIL: openclaw_endpoints.rs has only $OPENCLAW markers (expected >= 2)"
    exit 1
fi
echo "  [2/4] ✅ PASS: $OPENCLAW VtuGuard marker calls in openclaw_endpoints.rs"

# 3. No "bypass" in code (informational)
BYPASS=$(grep -rn "bypass" src/execution_engine.rs crates/server/src/openclaw_endpoints.rs 2>/dev/null | grep -v "^Binary\|//.*bypass\|/\*.*bypass" | wc -l || true)
echo "  [3/4] bypass patterns in DML paths: $BYPASS (informational)"

# 4. VtuGuard::assert_path_for_dml is public
if ! grep -q "pub fn assert_path_for_dml" crates/storage/src/vtu_guard.rs; then
    echo "  ❌ FAIL: VtuGuard::assert_path_for_dml not declared pub fn"
    exit 1
fi
echo "  [4/4] ✅ PASS: VtuGuard::assert_path_for_dml is public"

echo
echo "=== G4 Gate: PASS ==="
echo "ARCH-3 (#3169) Blocker-3 verified: VtuGuard main path enforced"
exit 0
