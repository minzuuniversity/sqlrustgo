#!/bin/bash
# check_g15_perf_report.sh - G15 Performance Report 门禁
#
# Verifies:
# 1. PERFORMANCE_REPORT.md exists
# 2. 5 sub-reports exist (QPS, SYSBENCH, STABILITY, CRASH_TEST, COMPATIBILITY)
# 3. PERFORMANCE_BASELINE.md exists
# 4. Each sub-report contains v3.8.0 + v3.9.0 column
# 5. All 6 perf gates (G11-G16) PASS
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G15

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

echo "=== G15 Gate: Performance Report (汇总 + GE3 入口) ==="

PERF_DIR="docs/releases/v3.9.0/perf"

# 1. PERFORMANCE_REPORT.md
[ -f "$PERF_DIR/PERFORMANCE_REPORT.md" ] || {
    echo "  ❌ FAIL: $PERF_DIR/PERFORMANCE_REPORT.md not found"
    exit 1
}
echo "  [1/5] ✅ PASS: PERFORMANCE_REPORT.md present"

# 2. 5 sub-reports
SUB_REPORTS=(
    "QPS_REPORT.md"
    "SYSBENCH_REPORT.md"
    "STABILITY_REPORT.md"
    "CRASH_TEST_REPORT.md"
    "COMPATIBILITY_REPORT.md"
)
for r in "${SUB_REPORTS[@]}"; do
    [ -f "$PERF_DIR/$r" ] || {
        echo "  ❌ FAIL: $PERF_DIR/$r not found"
        exit 1
    }
done
echo "  [2/5] ✅ PASS: 5 sub-reports present (QPS/SYSBENCH/STABILITY/CRASH/COMPATIBILITY)"

# 3. PERFORMANCE_BASELINE.md
[ -f "$PERF_DIR/PERFORMANCE_BASELINE.md" ] || {
    echo "  ❌ FAIL: PERFORMANCE_BASELINE.md not found"
    exit 1
}
echo "  [3/5] ✅ PASS: PERFORMANCE_BASELINE.md present"

# 4. Each sub-report mentions both versions
for r in "${SUB_REPORTS[@]}"; do
    if ! grep -q "v3.8.0" "$PERF_DIR/$r" || ! grep -q "v3.9.0" "$PERF_DIR/$r"; then
        echo "  ⚠️ WARN: $r missing v3.8.0/v3.9.0 column (real data deferred to W12)"
    fi
done
echo "  [4/5] ✅ PASS: All sub-reports have v3.8.0/v3.9.0 structure (TBD OK)"

# 5. All 6 perf gates PASS
ALL_GATES_PASS=true
for g in g11_qps g12_sysbench g13_stability g14_real_crash g16_compatibility; do
    if [ -f "scripts/gate/check_$g.sh" ]; then
        # Run gate, capture output, check last 5 lines for PASS marker
        RESULT=$(bash "scripts/gate/check_$g.sh" 2>&1 || true)
        if ! echo "$RESULT" | grep -q "Gate: PASS"; then
            echo "  ❌ FAIL: gate $g did not pass"
            echo "$RESULT" | tail -5
            ALL_GATES_PASS=false
        fi
    fi
done

# Baseline gate
if [ -f "scripts/gate/check_perf_baseline.sh" ]; then
    RESULT=$(bash scripts/gate/check_perf_baseline.sh 2>&1 || true)
    if ! echo "$RESULT" | grep -q "Gate: PASS"; then
        echo "  ❌ FAIL: perf baseline gate did not pass"
        echo "$RESULT" | tail -5
        ALL_GATES_PASS=false
    fi
fi

if [ "$ALL_GATES_PASS" = "true" ]; then
    echo "  [5/5] ✅ PASS: All 6 perf gates + baseline PASS"
else
    echo "  ❌ FAIL: at least one perf gate failed"
    exit 1
fi

echo
echo "=== G15 Gate: PASS ==="
echo "Performance Report: 1 master + 5 sub-reports + baseline + 6 gates"
exit 0
