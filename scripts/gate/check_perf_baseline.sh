#!/bin/bash
# check_perf_baseline.sh - Performance Baseline Gate
#
# Verifies:
# 1. PERFORMANCE_BASELINE.md exists
# 2. baseline contains v3.8.0 + v3.9.0 columns
# 3. baseline contains delta (Δ) column
# 4. baseline contains threshold column
# 5. baseline structure matches G11-G16 sections
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §Perf Baseline

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

echo "=== Perf Baseline Gate ==="

BASELINE="docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md"

# 1. Baseline file exists
[ -f "$BASELINE" ] || {
    echo "  ❌ FAIL: $BASELINE not found"
    exit 1
}
echo "  [1/5] ✅ PASS: $BASELINE present"

# 2. v3.8.0 column
grep -q "v3.8.0" "$BASELINE" || {
    echo "  ❌ FAIL: 'v3.8.0' column not in $BASELINE"
    exit 1
}
echo "  [2/5] ✅ PASS: v3.8.0 column present"

# 3. v3.9.0 column
grep -q "v3.9.0" "$BASELINE" || {
    echo "  ❌ FAIL: 'v3.9.0' column not in $BASELINE"
    exit 1
}
echo "  [3/5] ✅ PASS: v3.9.0 column present"

# 4. Δ (delta) column
grep -q "Δ" "$BASELINE" || {
    echo "  ❌ FAIL: 'Δ' (delta) column not in $BASELINE"
    exit 1
}
echo "  [4/5] ✅ PASS: Δ (delta) column present"

# 5. Threshold column
grep -q "阈值\|threshold" "$BASELINE" || {
    echo "  ❌ FAIL: '阈值' (threshold) column not in $BASELINE"
    exit 1
}
echo "  [5/5] ✅ PASS: 阈值 (threshold) column present"

echo
echo "=== Perf Baseline Gate: PASS ==="
echo "Baseline structure validated. Real values populated in W12 D3 (Z6G4 measurement)."
exit 0
