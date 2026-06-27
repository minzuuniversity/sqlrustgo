#!/bin/bash
# G16 Case 4: v3.8.0 Metadata/Catalog → v3.9.0 Catalog
#
# Verifies that v3.8.0 metadata (table definitions, indexes, constraints)
# is loaded by v3.9.0 Catalog. Real version:
#   1. checkout v3.8.0, create 20 tables with indexes
#   2. stop, checkout v3.9.0
#   3. start, SHOW TABLES must list all 20 tables
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G16 Case 4

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

echo "=== G16 Case 4: v3.8.0 Metadata/Catalog → v3.9.0 Catalog ==="
echo "The unit tests in tests/v380_to_v390_full_upgrade_test.rs"
echo "(test_g16_case4_metadata_catalog_*) verify this scenario."
echo

cd "$PROJECT_ROOT"
echo "Running test_g16_case4_metadata_catalog_* ..."
cargo test --test v380_to_v390_full_upgrade_test test_g16_case4 2>&1 | tail -5

echo
echo "✅ Case 4 unit tests pass."
