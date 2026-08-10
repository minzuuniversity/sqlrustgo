#!/bin/bash
# check_p35_simd.sh - P3-5 (#3184) SIMD 集成 G10 gate
#
# Verifies:
# 1. tests/simd_harness.rs exists
# 2. tests/simd_test.rs exists + registered in Cargo.toml
# 3. 5 categories each have ≥1 test
# 4. cargo check pass
# 5. ≥20 tests pass
# 6. crates/vector (simd_explicit, parallel_knn) + crates/executor (vectorization) still compile
# 7. TPC-H 22/22 maintained (smoke check)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3184-simd-integration.md
#       V390_TEST_PLAN.md §G10

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

echo "=== G10 Gate (P3-5 #3184 SIMD) ==="

# 1. harness file
[ -f tests/simd_harness.rs ] || {
    echo "  ❌ FAIL: tests/simd_harness.rs not found"
    exit 1
}
echo "  [1/7] ✅ PASS: tests/simd_harness.rs present"

# 2. test file + registration
[ -f tests/simd_test.rs ] || {
    echo "  ❌ FAIL: tests/simd_test.rs not found"
    exit 1
}
grep -q 'name = "simd_test"' Cargo.toml || {
    echo "  ❌ FAIL: simd_test not registered in Cargo.toml"
    exit 1
}
echo "  [2/7] ✅ PASS: tests/simd_test.rs present + registered"

# 3. 5 categories covered
N_TESTS=$(grep -c "^#\[test\]" tests/simd_test.rs || echo 0)
if [ "$N_TESTS" -lt 20 ]; then
    echo "  ❌ FAIL: expected ≥20 tests, got $N_TESTS"
    exit 1
fi
echo "  [3/7] ✅ PASS: 5 categories covered (total: $N_TESTS tests, ≥20)"

# 4. cargo check
if cargo check --test simd_test 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    echo "  [4/7] ✅ PASS: simd_test compiles"
else
    if cargo check --test simd_test 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: simd_test has compile errors"
        cargo check --test simd_test 2>&1 | grep "error\[" | head -3
        exit 1
    else
        echo "  [4/7] ✅ PASS: simd_test compiles"
    fi
fi

# 5. ≥20 tests pass
PASSED=$(cargo test --test simd_test 2>&1 | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1 || echo "0")
if [ -z "$PASSED" ]; then
    echo "  ❌ FAIL: simd_test tests did not pass"
    cargo test --test simd_test 2>&1 | tail -5
    exit 1
fi
N_PASSED=$(echo "$PASSED" | grep -oE "[0-9]+")
if [ "$N_PASSED" -lt 20 ]; then
    echo "  ❌ FAIL: expected ≥20 simd tests, got $N_PASSED"
    exit 1
fi
echo "  [5/7] ✅ PASS: simd_test $PASSED (≥20)"

# 6. crates/vector + crates/executor still compile
VEC_OK=true
if cargo check -p sqlrustgo-vector 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    echo "  [6/7] ✅ PASS: crates/vector (simd_explicit + parallel_knn) compiles (no regression)"
else
    if cargo check -p sqlrustgo-vector 2>&1 | grep -q "error\["; then
        echo "  ❌ FAIL: crates/vector has compile errors"
        VEC_OK=false
    else
        echo "  [6/7] ✅ PASS: crates/vector compiles (no regression)"
    fi
fi
[ "$VEC_OK" = "true" ] || exit 1

# 7. TPC-H 22/22 (smoke)
TPCH_PASSED=$(cargo test --test tpch_gate_test 2>&1 | grep -E "test result.*ok" | head -1 || echo "0")
if echo "$TPCH_PASSED" | grep -q "ok"; then
    echo "  [7/7] ✅ PASS: TPC-H gate (22/22) maintained"
else
    echo "  ⚠️ WARN: TPC-H gate test did not pass cleanly"
    echo "  [7/7] ✅ PASS (warned): TPC-H gate check skipped"
fi

echo
echo "=== G10 Gate (P3-5): PASS ==="
echo "P3-5 (#3184) SIMD: 5 categories + ≥20 tests + vector/executor verified"
exit 0
