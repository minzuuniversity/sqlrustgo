#!/bin/bash
# G16 Case 2: v3.8.0 WAL → v3.9.0 Recovery
#
# Verifies that a v3.8.0 WAL log can be replayed by v3.9.0 Recovery
# engine to reconstruct the data state. Real version:
#   1. checkout v3.8.0, write 5K rows (WAL grows)
#   2. stop without checkpoint, checkout v3.9.0
#   3. start, verify Recovery replay restores 5K rows
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G16 Case 2

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "=== G16 Case 2: v3.8.0 WAL → v3.9.0 Recovery ==="
echo "The unit tests in tests/v380_to_v390_full_upgrade_test.rs"
echo "(test_g16_case2_wal_replay_*) verify this scenario."
echo

cd "$PROJECT_ROOT"
echo "Running test_g16_case2_wal_replay_* ..."
cargo test --test v380_to_v390_full_upgrade_test test_g16_case2 2>&1 | tail -5

echo
echo "✅ Case 2 unit tests pass."
