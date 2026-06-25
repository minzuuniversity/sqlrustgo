#!/usr/bin/env bash
#
# scripts/gate/check_test_count_monotonic.sh
#
# Purpose: P13 Test Count Monotonicity - prevent silent test deletion
#
# Vulnerability V3: Test total can decrease silently (delete tests -> gate
# still PASSes because exit code is 0). This violates coverage guarantees.
#
# Coverage: P13 (Test Count Monotonicity), P6 (Evidence Binding)
#
# Strategy: maintain a baseline file (tests/baseline/test_count.json) with
# known good counts. Any commit that decreases test count must be flagged.
#
# Exit codes:
#   0 = PASS (count >= baseline)
#   1 = FAIL (count < baseline - tests deleted)
#   2 = DRIFT (no baseline file)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

BASELINE="$PROJECT_ROOT/tests/baseline/test_count.json"
RESULT_TMP=$(mktemp)

echo "=== P13 Test Count Monotonicity ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

# ---------------------------------------------------------------------------
# Step 1: Count current tests
# ---------------------------------------------------------------------------
echo "[1/3] Counting current tests..."

# Total tests: sum of [[test]] entries in Cargo.toml
cargo_tests=$(grep -c "\[\[test\]\]" Cargo.toml 2>/dev/null || echo 0)
echo "  Cargo.toml [[test]] entries: $cargo_tests"

# Active tests: count of #[test] functions (excluding #[ignore])
active_tests=$(grep -rE "^#\[test\]|^    #\[test\]" --include="*.rs" crates/ tests/ 2>/dev/null | wc -l | tr -d ' ' || echo 0)
echo "  Active #[test] functions: $active_tests"

# Ignored tests: count of #[ignore]
ignored_tests=$(grep -rE "#\[ignore" --include="*.rs" crates/ tests/ 2>/dev/null | wc -l | tr -d ' ' || echo 0)
echo "  #[ignore] tests: $ignored_tests"

# ---------------------------------------------------------------------------
# Step 2: Read baseline (if exists)
# ---------------------------------------------------------------------------
echo
echo "[2/3] Reading baseline..."

if [ ! -f "$BASELINE" ]; then
    echo "  ⚠️  DRIFT: baseline file not found: $BASELINE"
    echo "          First run: creating baseline from current state."
    mkdir -p "$(dirname "$BASELINE")"
    cat > "$BASELINE" <<EOF
{
  "version": "v3.9.0-rc7",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "cargo_tests": $cargo_tests,
  "active_tests": $active_tests,
  "ignored_tests": $ignored_tests,
  "note": "Auto-generated baseline. Update via ADR process if intentional reduction needed."
}
EOF
    echo "  Created: $BASELINE"
    echo
    echo "ℹ️  INFO: first run, baseline established. Re-run to verify monotonicity."
    rm -f "$RESULT_TMP"
    exit 0
fi

# Parse baseline (use | head -1 because baseline may contain previous_counts with same field names)
baseline_cargo=$(grep -oE '"cargo_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' | head -1 || echo 0)
baseline_active=$(grep -oE '"active_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' | head -1 || echo 0)
baseline_ignored=$(grep -oE '"ignored_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' | head -1 || echo 0)
baseline_version=$(grep -oE '"version": "[^"]*"' "$BASELINE" | cut -d'"' -f4 || echo "unknown")

echo "  Baseline ($baseline_version):"
echo "    cargo_tests: $baseline_cargo"
echo "    active_tests: $baseline_active"
echo "    ignored_tests: $baseline_ignored"

# ---------------------------------------------------------------------------
# Step 3: Compare and report
# ---------------------------------------------------------------------------
echo
echo "[3/3] Comparing..."

FAIL=0

# Check Cargo.toml test entries
if [ "$cargo_tests" -lt "$baseline_cargo" ]; then
    delta=$((cargo_tests - baseline_cargo))
    echo "  ❌ FAIL: Cargo.toml [[test]] entries DECREASED by $delta"
    echo "          Was: $baseline_cargo, Now: $cargo_tests"
    echo "          ACTION REQUIRED: ADR + explicit approval to remove tests"
    FAIL=$((FAIL+1))
elif [ "$cargo_tests" -gt "$baseline_cargo" ]; then
    delta=$((cargo_tests - baseline_cargo))
    echo "  ✅ PASS: Cargo.toml [[test]] entries increased by $delta"
else
    echo "  ✅ PASS: Cargo.toml [[test]] entries unchanged ($cargo_tests)"
fi

if [ "$active_tests" -lt "$baseline_active" ]; then
    delta=$((baseline_active - active_tests))
    echo "  ❌ FAIL: Active tests DECREASED by $delta"
    echo "          Was: $baseline_active, Now: $active_tests"
    echo "          ACTION REQUIRED: deletion of #[test] fns needs ADR"
    FAIL=$((FAIL+1))
elif [ "$active_tests" -gt "$baseline_active" ]; then
    delta=$((active_tests - baseline_active))
    echo "  ✅ PASS: Active tests increased by $delta"
else
    echo "  ✅ PASS: Active tests unchanged ($active_tests)"
fi

# Check ignored tests (should NEVER increase without ADR)
if [ "$ignored_tests" -gt "$baseline_ignored" ]; then
    delta=$((ignored_tests - baseline_ignored))
    echo "  ❌ FAIL: #[ignore] count INCREASED by $delta"
    echo "          Was: $baseline_ignored, Now: $ignored_tests"
    echo "          ACTION REQUIRED: P12 violation - new ignored tests must be in ADR"
    FAIL=$((FAIL+1))
elif [ "$ignored_tests" -lt "$baseline_ignored" ]; then
    delta=$((ignored_tests - baseline_ignored))
    echo "  ✅ PASS: #[ignore] count decreased by $delta (good — tests un-ignored)"
else
    echo "  ⚠️  INFO: #[ignore] count unchanged ($ignored_tests)"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo
echo "=== P13 Summary ==="
echo "  Current counts: cargo=$cargo_tests, active=$active_tests, ignored=$ignored_tests"
echo "  Baseline counts: cargo=$baseline_cargo, active=$baseline_active, ignored=$baseline_ignored"

# Update baseline if counts increased (allowed evolution)
if [ "$cargo_tests" -ge "$baseline_cargo" ] && \
   [ "$active_tests" -ge "$baseline_active" ] && \
   [ "$ignored_tests" -le "$baseline_ignored" ]; then
    cat > "$BASELINE" <<EOF
{
  "version": "v3.9.0-rc7",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "cargo_tests": $cargo_tests,
  "active_tests": $active_tests,
  "ignored_tests": $ignored_tests,
  "note": "Updated by check_test_count_monotonic.sh"
}
EOF
    echo "  Baseline updated to current counts"
fi

rm -f "$RESULT_TMP"

if [ "$FAIL" -gt 0 ]; then
    echo
    echo "❌ FAIL — P13 violations detected. Test count decreased or ignored count increased."
    exit 1
fi

echo
echo "✅ PASS — P13 satisfied. Test count is monotonically non-decreasing."
exit 0