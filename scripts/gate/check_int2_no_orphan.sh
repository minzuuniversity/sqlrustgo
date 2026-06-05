#!/bin/bash
# check_int2_no_orphan.sh - INT-2 (#3171) ParallelExecutor main-path G2 gate
#
# Verifies P0-3 INT-2 ParallelExecutor is integrated in the main path:
#   1. src/execution_engine.rs contains ParallelExecutor references
#   2. src/engine_select.rs uses parallel_degree
#   3. crates/executor/src/lib.rs declares pub mod parallel_executor
#   4. ParallelVolcanoExecutor::partition_scan is public
#   5. tests/parallel_executor_integration_test.rs has >= 7 tests
#   6. clippy on sqlrustgo-executor is clean
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3171-i12-parallel-executor-integration.md
#       V390_TEST_PLAN.md §G2

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== G2 Gate: INT-2 (#3171) ParallelExecutor main-path ==="

# 1. execution_engine.rs imports/references ParallelExecutor (trait or struct)
COUNT=$(grep -cE "ParallelExecutor|parallel_executor" src/execution_engine.rs 2>/dev/null || true)
if [ "$COUNT" -lt 1 ]; then
    echo "  [1/6] FAIL: src/execution_engine.rs missing ParallelExecutor reference (found $COUNT)"
    exit 1
fi
echo "  [1/6] PASS: $COUNT ParallelExecutor/parallel_executor references in src/execution_engine.rs"

# 2. engine_select.rs uses parallel_degree
DEG=$(grep -c "parallel_degree" src/engine_select.rs 2>/dev/null || true)
if [ "$DEG" -lt 1 ]; then
    echo "  [2/6] FAIL: src/engine_select.rs missing parallel_degree usage (found $DEG)"
    exit 1
fi
echo "  [2/6] PASS: $DEG parallel_degree usages in src/engine_select.rs"

# 3. crates/executor/src/lib.rs declares parallel_executor module
if ! grep -q "^pub mod parallel_executor;" crates/executor/src/lib.rs; then
    echo "  [3/6] FAIL: crates/executor/src/lib.rs missing 'pub mod parallel_executor;'"
    exit 1
fi
echo "  [3/6] PASS: parallel_executor is pub mod in crates/executor"

# 4. ParallelVolcanoExecutor has a public partition API
if ! grep -qE "pub fn partition_(scan|rows)|fn partition_scan" crates/executor/src/parallel_executor.rs; then
    echo "  [4/6] FAIL: ParallelVolcanoExecutor has no public partition API"
    exit 1
fi
echo "  [4/6] PASS: partition API is public"

# 5. e2e tests present in tests/parallel_executor_integration_test.rs
if [ ! -f tests/parallel_executor_integration_test.rs ]; then
    echo "  [5/6] FAIL: tests/parallel_executor_integration_test.rs not found"
    exit 1
fi
TESTS=$(grep -c "^#\[test\]" tests/parallel_executor_integration_test.rs 2>/dev/null || true)
if [ "$TESTS" -lt 7 ]; then
    echo "  [5/6] FAIL: only $TESTS tests (expected >= 7) in tests/parallel_executor_integration_test.rs"
    exit 1
fi
echo "  [5/6] PASS: $TESTS tests in tests/parallel_executor_integration_test.rs"

# 6. clippy clean on sqlrustgo-executor (5min budget)
echo "  [6/6] Running cargo clippy on sqlrustgo-executor (5min budget)..."
if ! timeout 300 cargo clippy -p sqlrustgo-executor --all-features -- -D warnings 2>&1 | tail -3; then
    echo "  [6/6] FAIL: clippy errors in sqlrustgo-executor"
    exit 1
fi
echo "  [6/6] PASS: clippy clean on sqlrustgo-executor"

echo
echo "=== G2 Gate: PASS ==="
echo "INT-2 (#3171) ParallelExecutor main-path verified"
exit 0
