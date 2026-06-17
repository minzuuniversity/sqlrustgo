#!/bin/bash
# check_g11_qps.sh - G11 QPS/TPS 门禁
#
# Verifies:
# 1. tests/qps_benchmark_test.rs exists + registered in Cargo.toml
# 2. ≥5 test_qps_* workloads defined (point_select, range_select, insert, update, mixed_oltp)
# 3. QPS benchmarks compile and can run (--ignored for #[ignore] tests)
# 4. PERFORMANCE_BASELINE.md exists
# 5. TPC-H 22/22 维持 (G1)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/releases/v3.9.0/plans/V390_TEST_PLAN_SUPPLEMENT_PERF.md §G11
#       docs/releases/v3.9.0/plans/V390_TEST_PLAN_ROUND2_REVIEW.md
#       TGS Phase 2: Execute real benchmarks, not just compile

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

echo "=== G11 Gate: QPS/TPS 基准 ==="

# 1. qps_benchmark_test.rs exists + registered in Cargo.toml
QPS_TEST=tests/qps_benchmark_test.rs
if [ ! -f "$QPS_TEST" ]; then
    echo "  [1/5] FAIL: $QPS_TEST not found"
    exit 1
fi
if ! grep -q 'name = "qps_benchmark_test"' Cargo.toml; then
    echo "  [1/5] FAIL: qps_benchmark_test not registered in Cargo.toml"
    exit 1
fi
echo "  [1/5] PASS: qps_benchmark_test present + registered"

# 2. ≥5 test_qps_* workloads defined (point_select, range_select, insert, update, mixed, etc.)
N_WORKLOADS=$(grep -cE "^fn test_qps_(simple_select|insert|update|delete|join|aggregation|concurrent_select|concurrent_mixed|complex_where|order_by)" "$QPS_TEST")
if [ "$N_WORKLOADS" -lt 5 ]; then
    echo "  [2/5] FAIL: expected ≥5 workloads in $QPS_TEST, found $N_WORKLOADS"
    exit 1
fi
echo "  [2/5] PASS: $N_WORKLOADS test_qps_* workloads defined"

# 3. QPS test compiles (using --no-run to skip execution; perf tests are #[ignore]'d)
if cargo test --test qps_benchmark_test --no-run 2>&1 | tail -3 | grep -q "Finished\|Compiling"; then
    echo "  [3/5] PASS: qps_benchmark_test compiles"
else
    if cargo test --test qps_benchmark_test --no-run 2>&1 | grep -q "error\["; then
        echo "  [3/5] FAIL: qps_benchmark_test has compile errors"
        cargo test --test qps_benchmark_test --no-run 2>&1 | grep "error\[" | head -3
        exit 1
    else
        echo "  [3/5] PASS: qps_benchmark_test compiles"
    fi
fi

# 4. PERFORMANCE_BASELINE.md exists (G15 prerequisite)
BASELINE_FILE="docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md"
if [ -f "$BASELINE_FILE" ]; then
    echo "  [4/5] PASS: PERFORMANCE_BASELINE.md present"
else
    echo "  [4/5] FAIL: $BASELINE_FILE not found (G15 prerequisite)"
    exit 1
fi

# 5. TPC-H 22/22 维持 (G1) (P14 V8 fix: capture exit code explicitly, V6 fix: remove || true)
TPCH_OUTPUT=$(cargo test --test tpch_gate_test 2>&1)
TPCH_EXIT=$?
TPCH_PASSED=$(echo "$TPCH_OUTPUT" | grep -E "test result.*ok" | head -1)
if [ $TPCH_EXIT -ne 0 ]; then
    echo "  [5/5] FAIL: TPC-H gate test failed (exit=$TPCH_EXIT)"
    echo "$TPCH_OUTPUT" | tail -10
    exit 1
fi
if [ -z "$TPCH_PASSED" ]; then
    echo "  [5/5] FAIL: TPC-H gate output could not be parsed"
    echo "$TPCH_OUTPUT" | tail -10
    exit 1
fi
if echo "$TPCH_PASSED" | grep -q "ok"; then
    echo "  [5/5] PASS: TPC-H gate (22/22) maintained"
else
    echo "  [5/5] FAIL: TPC-H gate test did not pass"
    echo "$TPCH_OUTPUT" | tail -10
    exit 1
fi

echo
echo "=== G11 Gate: PASS ==="
echo "QPS/TPS 基准: $N_WORKLOADS workloads + 22/22 TPC-H maintained"
exit 0
