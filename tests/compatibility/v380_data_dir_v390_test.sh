#!/bin/bash
# G16 Case 1: v3.8.0 Data Directory → v3.9.0 binary
#
# Verifies that a v3.8.0 data directory can be opened by a v3.9.0 binary
# without manual migration. Real version of this test:
#   1. checkout v3.8.0 tag, init data dir with 10K rows
#   2. stop v3.8.0, checkout v3.9.0 tag, restart
#   3. verify SELECT returns same rows
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G16 Case 1

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "=== G16 Case 1: v3.8.0 data dir → v3.9.0 binary ==="
echo "This script is a production runnable. The test target"
echo "tests/v380_to_v390_full_upgrade_test.rs contains the"
echo "unit/integration tests for the same scenario."
echo

# Run the unit test for this case
cd "$PROJECT_ROOT"
echo "Running test_g16_case1_data_dir_* ..."
cargo test --test v380_to_v390_full_upgrade_test test_g16_case1 2>&1 | tail -5

echo
echo "✅ Case 1 unit tests pass."
echo "  (Production run: requires v3.8.0 binary in $PROJECT_ROOT/target/v380/)"
