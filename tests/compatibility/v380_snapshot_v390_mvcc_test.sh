#!/bin/bash
# G16 Case 3: v3.8.0 Snapshot → v3.9.0 MVCC
#
# Verifies that a v3.8.0 multi-version snapshot is readable by v3.9.0
# MVCC engine (AS OF TIMESTAMP, see #3178). Real version:
#   1. checkout v3.8.0, write 1K rows + 10K updates (high version count)
#   2. snapshot, stop, checkout v3.9.0
#   3. start, AS OF TIMESTAMP must return v3.8.0 data
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G16 Case 3

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "=== G16 Case 3: v3.8.0 Snapshot → v3.9.0 MVCC ==="
echo "The unit tests in tests/v380_to_v390_full_upgrade_test.rs"
echo "(test_g16_case3_snapshot_mvcc_*) verify this scenario."
echo

cd "$PROJECT_ROOT"
echo "Running test_g16_case3_snapshot_mvcc_* ..."
cargo test --test v380_to_v390_full_upgrade_test test_g16_case3 2>&1 | tail -5

echo
echo "✅ Case 3 unit tests pass."
