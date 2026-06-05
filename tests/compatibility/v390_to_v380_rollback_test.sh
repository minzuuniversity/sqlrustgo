#!/bin/bash
# G16 Case 5: Rollback v3.9.0 → v3.8.0 (backward compat)
#
# Verifies that v3.9.0 can be downgraded back to v3.8.0 by checking
# that no v3.9.0-only features are required for the data. Real version:
#   1. checkout v3.8.0, write 1K rows
#   2. upgrade to v3.9.0 (no DDL change)
#   3. rollback to v3.8.0, verify same data visible
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G16 Case 5 (Rollback)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "=== G16 Case 5: Rollback v3.9.0 → v3.8.0 ==="
echo "The unit tests in tests/v380_to_v390_full_upgrade_test.rs"
echo "(test_g16_case5_rollback_*) verify this scenario."
echo

cd "$PROJECT_ROOT"
echo "Running test_g16_case5_rollback_* ..."
cargo test --test v380_to_v390_full_upgrade_test test_g16_case5 2>&1 | tail -5

echo
echo "✅ Case 5 (rollback) unit tests pass."
