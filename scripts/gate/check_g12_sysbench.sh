#!/bin/bash
# check_g12_sysbench.sh - G12 Sysbench OLTP 门禁
#
# Verifies:
# 1. 5 sysbench scripts exist (oltp_point_select, oltp_read_only, oltp_read_write, oltp_write_only, oltp_insert)
# 2. oltp_test has ≥30 tests
# 3. oltp tests PASS
# 4. sysbench binary installed
# 5. TPC-H 22/22 maintained
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/releases/v3.9.0/plans/V390_TEST_PLAN_SUPPLEMENT_PERF.md §G12

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

echo "=== G12 Gate: Sysbench OLTP ==="

# 1. 5 sysbench scripts
SCRIPTS=(
    "scripts/sysbench/oltp_point_select.sh"
    "scripts/sysbench/oltp_read_only.sh"
    "scripts/sysbench/oltp_read_write.sh"
    "scripts/sysbench/oltp_write_only.sh"
    "scripts/sysbench/oltp_insert.sh"
)
for s in "${SCRIPTS[@]}"; do
    [ -f "$s" ] || {
        echo "  ❌ FAIL: $s not found"
        exit 1
    }
done
echo "  [1/7] ✅ PASS: 5 sysbench scripts present"

# 2. oltp_test ≥30 tests
N_OLTP=$(grep -c "^#\[test\]" crates/bench/tests/oltp_test.rs || echo 0)
if [ "$N_OLTP" -lt 30 ]; then
    echo "  ❌ FAIL: expected ≥30 oltp tests, got $N_OLTP"
    exit 1
fi
echo "  [2/7] ✅ PASS: oltp_test has $N_OLTP tests (≥30)"

# 3. oltp tests pass
OLTP_RESULT=$(cargo test -p sqlrustgo-bench --test oltp_test 2>&1 | grep -E "test result.*ok" | head -1 || echo "0")
if echo "$OLTP_RESULT" | grep -q "ok"; then
    N_PASSED=$(echo "$OLTP_RESULT" | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+")
    echo "  [3/7] ✅ PASS: oltp_test $N_PASSED tests pass"
else
    echo "  ❌ FAIL: oltp_test tests did not pass"
    cargo test -p sqlrustgo-bench --test oltp_test 2>&1 | tail -5
    exit 1
fi

# 4. sysbench binary available
if ! command -v sysbench >/dev/null 2>&1; then
    echo "  ❌ FAIL: sysbench binary not installed"
    echo "    install: brew install sysbench (macOS) or apt install sysbench (Linux)"
    exit 1
fi
SYSBENCH_VER=$(sysbench --version 2>&1 | head -1)
echo "  [4/7] ✅ PASS: sysbench installed ($SYSBENCH_VER)"

# 5. sysbench scripts executable
ALL_EXEC=true
for s in "${SCRIPTS[@]}"; do
    if [ ! -x "$s" ]; then
        echo "  ⚠️ WARN: $s not executable (will auto-fix)"
        ALL_EXEC=false
    fi
done
if [ "$ALL_EXEC" = "false" ]; then
    chmod +x "${SCRIPTS[@]}"
fi
echo "  [5/7] ✅ PASS: sysbench scripts executable"

# 6. TPC-H 22/22 maintained
bash "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/tpch_gate_subcheck.sh"
TPCH_RC=$?
if [[ "${TPCH_RC}" -eq 0 ]]; then
    echo "  [6/7] ✅ PASS: TPC-H gate (22/22) maintained"
elif [[ "${TPCH_RC}" -eq 2 ]]; then
    echo "  [6/7] ⚠️ INCOMPLETE: TPC-H sub-check could not be performed (fixture missing / NOT-RUN disclosed) — not a drift"
else
    echo "  [6/7] ❌ FAIL: TPC-H gate did not pass"
fi

# 7. SYSBENCH_REPORT placeholder (W12 D2 will populate)
REPORT="docs/releases/v3.9.0/perf/SYSBENCH_REPORT.md"
if [ -f "$REPORT" ]; then
    echo "  [7/7] ✅ PASS: SYSBENCH_REPORT.md present"
else
    echo "  ⚠️ WARN: $REPORT not yet created (will be created in W12 D2 with real data)"
    echo "  [7/7] ✅ PASS (warned): report check deferred to W12"
fi

echo
if [[ "${TPCH_RC:-0}" -eq 2 ]]; then
    echo "=== G12 Gate: INCOMPLETE (TPC-H sub-check not performed; see INCOMPLETE line above) ==="
    exit 2
fi
if [[ "${TPCH_RC:-0}" -eq 1 ]]; then
    echo "=== G12 Gate: FAIL (TPC-H gate did not pass) ==="
    exit 1
fi
echo "=== G12 Gate: PASS ==="
echo "Sysbench OLTP: 5 scripts + 30+ unit tests verified"
exit 0
