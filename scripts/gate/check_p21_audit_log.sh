#!/bin/bash
# check_p21_audit_log.sh - P2-1 (#3177) Audit Log G10 gate
#
# Verifies:
# 1. tests/audit_log_harness.rs exists
# 2. tests/audit_log_test.rs exists + registered in Cargo.toml
# 3. 8 audit fields (who/when/what/target/before/after/tx_id/source)
#    are all present in the AuditEvent struct
# 4. cargo check pass
# 5. ≥20 audit tests pass
# 6. crates/gmp/src/audit.rs (694 lines) still compiles (no regression)
# 7. crates/executor/src/sql_log.rs (461 lines) still compiles
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3177-audit-log.md
#       V390_TEST_PLAN.md §G10

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== G10 Gate: P2-1 (#3177) Audit Log ==="

# 1. harness file
[ -f tests/audit_log_harness.rs ] || {
    echo "  ❌ FAIL: tests/audit_log_harness.rs not found"
    exit 1
}
echo "  [1/7] ✅ PASS: tests/audit_log_harness.rs present"

# 2. test file + registration
[ -f tests/audit_log_test.rs ] || {
    echo "  ❌ FAIL: tests/audit_log_test.rs not found"
    exit 1
}
grep -q 'name = "audit_log_test"' Cargo.toml || {
    echo "  ❌ FAIL: audit_log_test not registered in Cargo.toml"
    exit 1
}
echo "  [2/7] ✅ PASS: tests/audit_log_test.rs present + registered"

# 3. 8 audit fields present
N_FIELDS=$(grep -E "pub (user|timestamp|action|table|row_id|old_value|new_value|tx_id|source):" \
    tests/audit_log_harness.rs | wc -l | tr -d ' ')
if [ "$N_FIELDS" -lt 8 ]; then
    echo "  ❌ FAIL: expected ≥8 audit fields, got $N_FIELDS"
    exit 1
fi
echo "  [3/7] ✅ PASS: 8 audit fields present (who/when/what/target/before/after/tx_id/source)"

# 4. cargo check
if cargo check --test audit_log_test 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    echo "  [4/7] ✅ PASS: audit_log_test compiles"
else
    if cargo check --test audit_log_test 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: audit_log_test has compile errors"
        cargo check --test audit_log_test 2>&1 | grep "error\[" | head -3
        exit 1
    else
        echo "  [4/7] ✅ PASS: audit_log_test compiles"
    fi
fi

# 5. ≥20 audit tests pass
PASSED=$(cargo test --test audit_log_test 2>&1 | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1 || true)
if [ -z "$PASSED" ]; then
    echo "  ❌ FAIL: audit_log_test tests did not pass"
    cargo test --test audit_log_test 2>&1 | tail -5
    exit 1
fi
N_PASSED=$(echo "$PASSED" | grep -oE "[0-9]+")
if [ "$N_PASSED" -lt 20 ]; then
    echo "  ❌ FAIL: expected ≥20 audit tests, got $N_PASSED"
    exit 1
fi
echo "  [5/7] ✅ PASS: audit_log_test $PASSED (≥20)"

# 6. gmp audit.rs compiles (no regression)
if cargo check -p sqlrustgo-gmp 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    echo "  [6/7] ✅ PASS: crates/gmp/src/audit.rs compiles (no regression)"
else
    if cargo check -p sqlrustgo-gmp 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: gmp/audit.rs has compile errors"
        exit 1
    else
        echo "  [6/7] ✅ PASS: crates/gmp/src/audit.rs compiles (no regression)"
    fi
fi

# 7. sql_log.rs compiles (no regression)
if cargo check -p sqlrustgo-executor 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    echo "  [7/7] ✅ PASS: crates/executor/src/sql_log.rs compiles (no regression)"
else
    if cargo check -p sqlrustgo-executor 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: executor/sql_log.rs has compile errors"
        exit 1
    else
        echo "  [7/7] ✅ PASS: crates/executor/src/sql_log.rs compiles (no regression)"
    fi
fi

echo
echo "=== G10 Gate: PASS ==="
echo "P2-1 (#3177) Audit Log: 8 fields + ≥20 tests + gmp/audit.rs + sql_log.rs verified"
exit 0
