#!/usr/bin/env bash
#
# scripts/gate/check_drift_not_pass.sh
#
# Purpose: P14 DRIFT != PASS - detect gates that treat DRIFT (exit 2) as PASS
#
# Vulnerability V5: check_full_gate_verification.sh's run_gate() function:
#   elif [ "$code" -eq 2 ] && [ "$expect_code" -eq 0 ]; then
#       echo "  ⚠️  DRIFT (exit 2, expected 0)"
#       DRIFT_COUNT=$((DRIFT_COUNT+1))
#   ...
# DRIFT is logged but final summary treats it as not-a-blocker.
# This violates P14 (DRIFT must NOT silently pass).
#
# Coverage: P14 (DRIFT != PASS), P5 (Governance > Features)
#
# Strategy: scan all gate scripts for anti-patterns that:
#   1. Accept exit 2 as success
#   2. Use `|| true` to swallow exit codes
#   3. Return 0 from a function/branch that includes DRIFT handling
#
# Exit codes:
#   0 = PASS (no DRIFT-accepting patterns found)
#   1 = FAIL (anti-patterns found)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== P14 DRIFT != PASS - Anti-pattern Detector ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

FAIL=0
FINDINGS=()

# ---------------------------------------------------------------------------
# Check 1: gate scripts that accept exit 2 as success
# ---------------------------------------------------------------------------
echo "[1/4] Scanning for DRIFT-accepting patterns..."

# Pattern: [ "$code" -eq 2 ] followed by non-error handling (no exit 1 / FAIL)
# This is a heuristic — grep for the suspicious pattern.
drift_pattern=$(grep -rnE '\[\s*"\$?(code|exit_code|status)"?\s*-eq\s*2\s*\]' \
    scripts/gate/ 2>/dev/null | head -20)

if [ -n "$drift_pattern" ]; then
    echo "  Found $(echo "$drift_pattern" | wc -l | tr -d ' ') potential DRIFT-handling sites"
    while IFS=: read -r filepath linenum content; do
        [ -z "$filepath" ] && continue

        # Check if this site accepts DRIFT (no exit 1 nearby)
        # Look at next 5 lines for exit 1
        next_5=$(sed -n "$((linenum+1)),$((linenum+5))p" "$filepath" 2>/dev/null)
        if ! echo "$next_5" | grep -qE 'exit [1-9]|FAIL|return [1-9]|BLOCKERS'; then
            FINDINGS+=("$filepath:$linenum: accepts exit 2 without explicit FAIL")
            FAIL=$((FAIL+1))
            echo "    ❌ $filepath:$linenum - accepts exit 2 without exit 1"
        fi
    done <<< "$drift_pattern"
fi

# ---------------------------------------------------------------------------
# Check 2: `|| true` to swallow exit codes (V6)
# ---------------------------------------------------------------------------
echo
echo "[2/4] Scanning for '|| true' patterns that swallow errors..."

swallow_pattern=$(grep -rnE 'cargo [a-z]+ [^|]+\|\|\s*true|>\s*/[^[:space:]]+\s*2>&1\s*\|\|\s*true' \
    scripts/gate/ 2>/dev/null | head -20)

if [ -n "$swallow_pattern" ]; then
    echo "  Found $(echo "$swallow_pattern" | wc -l | tr -d ' ') sites using '|| true' or swallow patterns"
    while IFS=: read -r filepath linenum content; do
        [ -z "$filepath" ] && continue
        FINDINGS+=("$filepath:$linenum: uses '|| true' to swallow exit codes")
        echo "    ⚠️  $filepath:$linenum"
    done <<< "$swallow_pattern"
fi

# ---------------------------------------------------------------------------
# Check 3: gate scripts that use `grep` on test output without verifying pattern
# ---------------------------------------------------------------------------
echo
echo "[3/4] Scanning for grep-on-stdout without result validation..."

grep_pattern=$(grep -rnE 'cargo test[^|]+\|[^|]*grep' scripts/gate/ 2>/dev/null | head -10)

if [ -n "$grep_pattern" ]; then
    echo "  Found $(echo "$grep_pattern" | wc -l | tr -d ' ') grep-on-stdout sites"
    while IFS=: read -r filepath linenum content; do
        [ -z "$filepath" ] && continue

        # Check if exit code is also checked (defense in depth)
        if ! grep -q "PIPESTATUS\|\\\$?" "$filepath" 2>/dev/null; then
            FINDINGS+=("$filepath:$linenum: grep on cargo test stdout without PIPESTATUS check")
            FAIL=$((FAIL+1))
            echo "    ❌ $filepath:$linenum"
        fi
    done <<< "$grep_pattern"
fi

# ---------------------------------------------------------------------------
# Check 4: Verify run_gate() in check_full_gate_verification.sh specifically
# ---------------------------------------------------------------------------
echo
echo "[4/4] Specific check: check_full_gate_verification.sh run_gate()"

full_gate_script="$SCRIPT_DIR/check_full_gate_verification.sh"
if [ -f "$full_gate_script" ]; then
    # Look for run_gate function and its DRIFT handling
    # Check if DRIFT pattern exists AND is followed by proper failure handling
    drtift_line=$(grep -nE '\[\s*"\$?code"?\s*-eq\s*2\s*\]' "$full_gate_script" 2>/dev/null | head -1)
    if [ -n "$drtift_line" ]; then
        linenum=$(echo "$drtift_line" | cut -d: -f1)
        # Check next 10 lines for proper DRIFT handling (return 1 or exit 1)
        next_10=$(sed -n "$((linenum)),$((linenum+10))p" "$full_gate_script" 2>/dev/null)
        if echo "$next_10" | grep -qE 'return\s+1|exit\s+1'; then
            echo "  ✅ PASS: run_gate() properly handles DRIFT with failure code"
        else
            echo "  ❌ FAIL: $full_gate_script accepts exit 2 (DRIFT) without proper failure handling"
            FINDINGS+=("$full_gate_script: run_gate() accepts DRIFT as PASS")
            FAIL=$((FAIL+1))
        fi
    else
        echo "  ✅ PASS: run_gate() no longer has DRIFT pattern"
    fi
else
    echo "  ⚠️  INFO: $full_gate_script not found (D-gate may be inactive)"
fi

# ---------------------------------------------------------------------------
# Baseline regression detection
# ---------------------------------------------------------------------------
DRIFT_BASELINE="$PROJECT_ROOT/tests/baseline/drift_baseline.json"
mkdir -p "$(dirname "$DRIFT_BASELINE")"

if [ ! -f "$DRIFT_BASELINE" ]; then
    echo
    echo "  ℹ️  First run: auto-creating DRIFT-anti-pattern baseline..."
    {
        echo "{"
        echo "  \"created_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
        echo "  \"total_findings\": $FAIL,"
        echo "  \"findings\": ["
        first=1
        for f in "${FINDINGS[@]}"; do
            [ $first -eq 0 ] && echo ","
            printf "    %s" "$(python3 -c "import json,sys; print(json.dumps(sys.argv[1]))" "$f")"
            first=0
        done
        echo
        echo "  ]"
        echo "}"
    } > "$DRIFT_BASELINE"
    echo "  Created: $DRIFT_BASELINE"
    echo "  ℹ️  Re-run to verify no NEW anti-patterns."
    FAIL=0
else
    BASELINE_FINDINGS=$(python3 -c "
import json
with open('$DRIFT_BASELINE') as f:
    data = json.load(f)
for finding in data.get('findings', []):
    print(finding)
" 2>/dev/null)

    NEW_FINDINGS=()
    for f in "${FINDINGS[@]}"; do
        is_known=0
        while IFS= read -r baseline_f; do
            [ -z "$baseline_f" ] && continue
            if [ "$f" = "$baseline_f" ]; then
                is_known=1
                break
            fi
        done <<< "$BASELINE_FINDINGS"
        if [ "$is_known" -eq 0 ]; then
            NEW_FINDINGS+=("$f")
        fi
    done

    if [ ${#NEW_FINDINGS[@]} -gt 0 ]; then
        echo
        echo "  ⚠️  REGRESSION: ${#NEW_FINDINGS[@]} NEW anti-pattern(s) (not in baseline):"
        for f in "${NEW_FINDINGS[@]}"; do
            echo "    - $f"
        done
        FINDINGS=("${NEW_FINDINGS[@]}")
        FAIL=${#NEW_FINDINGS[@]}
    else
        echo
        echo "  ✅ No NEW anti-patterns vs baseline"
        FINDINGS=()
        FAIL=0
    fi
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo
echo "=== P14 Summary ==="
echo "  Total anti-patterns: $FAIL"

if [ "$FAIL" -gt 0 ]; then
    echo
    echo "  Specific findings:"
    for f in "${FINDINGS[@]}"; do
        echo "    - $f"
    done
    echo
    echo "❌ FAIL — P14 violations. DRIFT-accepting patterns found in gate scripts."
    echo "   Required action: fix gate scripts to exit 1 on DRIFT (not exit 0)."
    exit 1
fi

echo
echo "✅ PASS — P14 satisfied. No gate script treats DRIFT as PASS."
exit 0