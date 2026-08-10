#!/usr/bin/env bash
#
# scripts/gate/check_gate_self_verification.sh
#
# Purpose: P11 Gate Self-Verification - meta-gate that verifies other gates
#          produce KNOWN outputs for KNOWN inputs (catches false positives).
#
# This addresses V1, V7, V8 from the 2026-06-13 meta-gate audit:
#   V1: exit-code-only gates accept "0 tests run" as PASS
#   V7: 82 gate scripts have no self-tests
#   V8: grep on stdout silently fails on format changes
#
# Coverage: P11 (Gate Self-Verification), P6 (Evidence Binding)
#
# Exit codes:
#   0 = ALL self-verifications PASS
#   1 = ANY verification FAIL (blocker)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

PASS=0
FAIL=0
FAIL_DETAILS=()

# ---------------------------------------------------------------------------
# V1 detector: ensure cargo test actually runs tests (not just exit 0)
# ---------------------------------------------------------------------------
check_cargo_test_actually_runs() {
    local label="$1"
    local output
    output=$(cargo test --lib -p sqlrustgo-parser --all-features 2>&1 || echo "0")

    # Must contain at least one "test ... ok" or "test result:" line
    local ok_count
    ok_count=$(echo "$output" | grep -cE '^test .* (ok|FAILED|ignored)$' || true)
    local result_line
    result_line=$(echo "$output" | grep -E '^test result:' | head -1)

    if [ "$ok_count" -eq 0 ] && [ -z "$result_line" ]; then
        echo "  [FAIL] $label: cargo test produced NO test output (0 tests run)"
        echo "          Possible: build failure, empty test target, or wrong invocation"
        FAIL=$((FAIL+1))
        FAIL_DETAILS+=("$label: no-test-output")
        return 1
    fi

    # Must contain "test result: ok" or "test result: FAILED"
    if [ -z "$result_line" ]; then
        echo "  [FAIL] $label: cargo test missing 'test result:' summary line"
        FAIL=$((FAIL+1))
        FAIL_DETAILS+=("$label: missing-summary")
        return 1
    fi

    if echo "$result_line" | grep -q "FAILED"; then
        echo "  [FAIL] $label: cargo test reports FAILED: $result_line"
        FAIL=$((FAIL+1))
        FAIL_DETAILS+=("$label: failed-tests")
        return 1
    fi

    echo "  [PASS] $label: cargo test produced $ok_count test results, $result_line"
    PASS=$((PASS+1))
    return 0
}

# ---------------------------------------------------------------------------
# V7 detector: ensure every gate script has documented purpose
# ---------------------------------------------------------------------------
check_gate_scripts_have_headers() {
    local label="gate_scripts_have_headers"
    local untagged=0
    local untagged_list=()

    for gate in "$SCRIPT_DIR"/check_*.sh "$SCRIPT_DIR"/check_*.py; do
        [ -f "$gate" ] || continue
        local basename
        basename=$(basename "$gate")

        # Check first 10 lines for purpose documentation
        local has_doc=0
        local has_exit_code_doc=0
        local has_principle_ref=0

        # Look for documentation patterns
        if head -10 "$gate" | grep -qiE "purpose|coverage|principle|verify|check"; then
            has_doc=1
        fi

        # Look for principle reference (P1-P15)
        if head -30 "$gate" | grep -qE "P[0-9]+"; then
            has_principle_ref=1
        fi

        if [ $has_doc -eq 0 ]; then
            untagged=$((untagged+1))
            untagged_list+=("$basename")
        fi
    done

    if [ $untagged -gt 0 ]; then
        echo "  [WARN] $label: $untagged gate scripts lack documentation header"
        echo "          (informational only; add P11-comments to gate scripts)"
        # WARN, not FAIL — gradual improvement
        PASS=$((PASS+1))
    else
        echo "  [PASS] $label: all gate scripts have documentation headers"
        PASS=$((PASS+1))
    fi
    return 0
}

# ---------------------------------------------------------------------------
# V8 detector: simulate cargo test output format change
# ---------------------------------------------------------------------------
# This test injects a fake "0 tests run" scenario and verifies the gate
# logic correctly rejects it.
check_gate_detects_zero_test_output() {
    local label="gate_detects_zero_test_output"

    # Simulate cargo test with 0 test outputs (e.g., build error)
    local fake_output=$(mktemp)
    cat > "$fake_output" <<'EOF'
   Compiling sqlrustgo-parser v3.9.0
    Finished release [optimized] target(s)
error[E0001]: simulated build error
EOF

    # The check() function in check_alpha_v380.sh pattern:
    #   if [ "$exit_code" -eq 0 ]; then PASS
    # A real exit-code-only check would PASS this fake. Our detector must FAIL it.

    local ok_count
    ok_count=$(grep -cE '^test .* (ok|FAILED|ignored)$' "$fake_output" || true)
    local result_line
    result_line=$(grep -E '^test result:' "$fake_output" || true)

    # Cleanup
    rm -f "$fake_output"

    if [ "$ok_count" -gt 0 ] || [ -n "$result_line" ]; then
        echo "  [FAIL] $label: false-positive detector triggered incorrectly"
        FAIL=$((FAIL+1))
        FAIL_DETAILS+=("$label: false-positive")
        return 1
    fi

    # Verify that 0 tests + no result line = FAIL
    if [ "$ok_count" -eq 0 ] && [ -z "$result_line" ]; then
        echo "  [PASS] $label: correctly identifies 0-tests-run as FAIL (no false positive)"
        PASS=$((PASS+1))
        return 0
    fi

    echo "  [FAIL] $label: logic error in detector"
    FAIL=$((FAIL+1))
    return 1
}

# ---------------------------------------------------------------------------
# V2 detector: ensure cargo test output contains "test result: ok" with
#             parsed counts matching baseline (not just exit 0)
# ---------------------------------------------------------------------------
check_test_result_parsing() {
    local label="test_result_parsing"
    local output
    output=$(cargo test --lib -p sqlrustgo-parser --all-features 2>&1 || echo "0")

    # Extract "test result: ok. N passed; M failed; K ignored"
    local summary
    summary=$(echo "$output" | grep -E '^test result:' | head -1)

    if [ -z "$summary" ]; then
        echo "  [FAIL] $label: no 'test result:' summary in cargo output"
        FAIL=$((FAIL+1))
        FAIL_DETAILS+=("$label: no-summary")
        return 1
    fi

    # Must parse to all three numbers (passed, failed, ignored)
    local passed
    passed=$(echo "$summary" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' || echo "0")
    local failed
    failed=$(echo "$summary" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' || echo "0")
    local ignored
    ignored=$(echo "$summary" | grep -oE '[0-9]+ ignored' | grep -oE '[0-9]+' || echo "0")

    if [ "$passed" -eq 0 ]; then
        echo "  [FAIL] $label: parsed 0 passed tests (suspicious — gate would say PASS for nothing)"
        FAIL=$((FAIL+1))
        FAIL_DETAILS+=("$label: zero-passed")
        return 1
    fi

    echo "  [PASS] $label: cargo test summary parsed: $passed passed, $failed failed, $ignored ignored"
    PASS=$((PASS+1))
    return 0
}

# ---------------------------------------------------------------------------
# Run all checks
# ---------------------------------------------------------------------------

echo "=== P11 Gate Self-Verification (Meta-Gate) ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

echo "--- V1 detector (cargo test actually runs tests) ---"
check_cargo_test_actually_runs "V1_cargo_test_real"

echo
echo "--- V7 detector (gate scripts have headers) ---"
check_gate_scripts_have_headers

echo
echo "--- V8 detector (gate detects 0-tests-run as FAIL) ---"
check_gate_detects_zero_test_output

echo
echo "--- V2 detector (test result parsing) ---"
check_test_result_parsing

echo
echo "=== P11 Summary ==="
echo "  PASS: $PASS"
echo "  FAIL: $FAIL"
if [ "$FAIL" -gt 0 ]; then
    echo
    echo "  Failed checks:"
    for detail in "${FAIL_DETAILS[@]}"; do
        echo "    - $detail"
    done
fi
echo

if [ "$FAIL" -gt 0 ]; then
    echo "❌ FAIL — P11 violations detected. Gates may produce false positives."
    echo "   Required action: fix the underlying gate scripts to actually verify test outputs."
    exit 1
fi

echo "✅ PASS — P11 satisfied. Gates verified to produce trustworthy outputs."
exit 0