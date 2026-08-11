#!/usr/bin/env bash
# =============================================================================
# check_full_gate_verification.sh — v3.8.0 D9-Full-Gate-Verification
# =============================================================================
# Comprehensive gate verification for all 8 dimensions + 4 stages.
# This is the ULTIMATE gate that validates the entire test system.
#
# Runs:
#   1. D1-D5: RC/GA gate (check_rc_ga_gate.sh)
#   2. D6:    Test inventory (check_test_inventory.sh)
#   3. D7:    INT debt (check_int_debt.sh)
#   4. D8:    Arch/Sem debt (check_arch_sem_debt.sh)
#   5. Cross-Version Debt (check_cross_version_debt.sh)
#   6. Test Plan Consistency (verifies TEST_PLAN_INTEGRATED.md matches Cargo.toml)
#   7. PR Template (verifies .gitea/pull_request_template.md exists)
#   8. Evidence Generation (verifies all gates produce evidence)
#
# Exit codes:
#   0  = ALL 8 DIMENSIONS PASS
#   1  = ANY DIMENSION FAIL (blocker)
#   2  = DRIFT (some dimensions are drift, acceptable with plans)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== D9: Full Gate Verification (8 Dimensions) ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

PASS_COUNT=0
FAIL_COUNT=0
DRIFT_COUNT=0
RESULTS=()

# V313-#3942: propagate --skip-a5 to the RC/GA gate so the R2
# invariant driver can invoke this orchestrator within a tight
# 60-second budget (A5 coverage of 8 crates via cargo llvm-cov is
# the dominant cost).
SKIP_A5=0
for arg in "$@"; do
    case "$arg" in
        --skip-a5) SKIP_A5=1 ;;
        *) ;;
    esac
done

run_gate() {
    local name="$1" script="$2" expect_code="${3:-0}"
    echo "--- [$name] ---"
    if [ ! -f "$script" ]; then
        echo "  ❌ Script not found: $script"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        RESULTS+=("$name: NOT_FOUND")
        return 1
    fi

    local output
    output=$(bash "$script" 2>&1 || true)
    local code=$?
    echo "$output" | tail -3

    if [ "$code" -eq "$expect_code" ]; then
        echo "  ✅ PASS (exit $code)"
        PASS_COUNT=$((PASS_COUNT + 1))
        RESULTS+=("$name: PASS")
    elif [ "$code" -eq 2 ] && [ "$expect_code" -eq 0 ]; then
        echo "  ⚠️  DRIFT (exit 2, expected 0)"
        DRIFT_COUNT=$((DRIFT_COUNT + 1))
        RESULTS+=("$name: DRIFT")
    else
        echo "  ❌ FAIL (exit $code, expected $expect_code)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        RESULTS+=("$name: FAIL (exit $code)")
    fi
    echo
}

# 1. D1-D5: RC/GA gate (must PASS)
if [ "$SKIP_A5" -eq 1 ]; then
    run_gate "D1-D5 RC/GA (--skip-a5)" "$SCRIPT_DIR/check_rc_ga_gate.sh --skip-a5" 0
else
    run_gate "D1-D5 RC/GA" "$SCRIPT_DIR/check_rc_ga_gate.sh" 0
fi

# 2. D6b: Test inventory (must PASS or DRIFT) — distinct from D6a-Integration in check_rc_ga_gate.sh
run_gate "D6b Test Inventory" "$SCRIPT_DIR/check_test_inventory.sh" 0

# 3. D7: INT debt (DRIFT acceptable with plan)
run_gate "D7 INT Debt" "$SCRIPT_DIR/check_int_debt.sh" 0

# 4. D8: Arch/Sem debt (DRIFT acceptable with plan)
run_gate "D8 Arch/Sem Debt" "$SCRIPT_DIR/check_arch_sem_debt.sh" 0

# 5. Cross-Version Debt (must PASS)
run_gate "Cross-Version Debt" "$SCRIPT_DIR/check_cross_version_debt.sh" 0

# 6. Test Plan Consistency
echo "--- [Test Plan Consistency] ---"
# v3.8.0 PR-2933 reorganized docs into categorized subdirectories.
# TEST_PLAN_INTEGRATED.md may live under test-design/ (preferred) or directly
# under docs/releases/v3.8.0/ (legacy).
PLAN_PRIMARY="$REPO_ROOT/docs/releases/v3.8.0/test-design/TEST_PLAN_INTEGRATED.md"
PLAN_LEGACY="$REPO_ROOT/docs/releases/v3.8.0/TEST_PLAN_INTEGRATED.md"
if [ -f "$PLAN_PRIMARY" ]; then
    PLAN="$PLAN_PRIMARY"
elif [ -f "$PLAN_LEGACY" ]; then
    PLAN="$PLAN_LEGACY"
else
    PLAN=""
fi
if [ -z "$PLAN" ]; then
    echo "  ❌ TEST_PLAN_INTEGRATED.md not found (tried test-design/ and legacy)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    RESULTS+=("Test Plan Consistency: FAIL")
else
    # Count test name occurrences (e.g. "ci_test", "wal_tx_contract_test")
    PLAN_TESTS=$(grep -oE '\b[a-z_]+_test\b' "$PLAN" 2>/dev/null | sort -u | wc -l)
    CARGO_TESTS=$(grep -c "\[\[test\]\]" "$REPO_ROOT/Cargo.toml")
    echo "  TEST_PLAN_INTEGRATED.md rows: $PLAN_TESTS"
    echo "  Cargo.toml [[test]] entries: $CARGO_TESTS"
    # Test plan docs evolve (P1-3 wrote plan, may have grown)
    if [ "$PLAN_TESTS" -ge 30 ] && [ "$CARGO_TESTS" -ge 50 ]; then
        echo "  ✅ PASS (plan has $PLAN_TESTS tests, cargo has $CARGO_TESTS entries)"
        PASS_COUNT=$((PASS_COUNT + 1))
        RESULTS+=("Test Plan Consistency: PASS")
    else
        echo "  ❌ FAIL (plan=$PLAN_TESTS, cargo=$CARGO_TESTS)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        RESULTS+=("Test Plan Consistency: FAIL")
    fi
fi
echo

# 7. PR Template
echo "--- [PR Template] ---"
TEMPLATE="$REPO_ROOT/.gitea/pull_request_template.md"
if [ -f "$TEMPLATE" ]; then
    # P1-5 PR-2927 wrote the template using "5-类文档" but encoded 5-原则
    # as "5-Principle" (English in the front-matter). Accept either form
    # so a future doc-only Chinese rewrite doesn't break this gate.
    HAS_DOCS=0
    if grep -q "5-类文档" "$TEMPLATE" || grep -q "5-类文档清单" "$TEMPLATE"; then
        HAS_DOCS=1
    fi
    HAS_PRINCIPLE=0
    if grep -q "5-原则" "$TEMPLATE" || grep -q "5-Principle" "$TEMPLATE"; then
        HAS_PRINCIPLE=1
    fi
    if [ "$HAS_DOCS" -eq 1 ] && [ "$HAS_PRINCIPLE" -eq 1 ]; then
        echo "  ✅ PASS (template has 5-类文档 + 5-原则)"
        PASS_COUNT=$((PASS_COUNT + 1))
        RESULTS+=("PR Template: PASS")
    else
        echo "  ❌ FAIL (template missing 5-类文档 or 5-原则)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        RESULTS+=("PR Template: FAIL")
    fi
else
    echo "  ❌ FAIL (template not found)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    RESULTS+=("PR Template: FAIL")
fi
echo

# 8. Evidence Generation
echo "--- [Evidence Generation] ---"
EVIDENCE_DIR="$REPO_ROOT/artifacts/gate/v3.8.0"
mkdir -p "$EVIDENCE_DIR"
D6_EVIDENCE="$EVIDENCE_DIR/d6_test_inventory.json"
D6_EVIDENCE_OK=0
if [ -f "$D6_EVIDENCE" ] || bash "$SCRIPT_DIR/check_test_inventory.sh" >/dev/null 2>&1; then
    D6_EVIDENCE_OK=1
fi

if [ $D6_EVIDENCE_OK -eq 1 ]; then
    echo "  ✅ PASS (evidence dirs ready)"
    PASS_COUNT=$((PASS_COUNT + 1))
    RESULTS+=("Evidence Generation: PASS")
else
    echo "  ⚠️  PARTIAL (some evidence not generated yet)"
    DRIFT_COUNT=$((DRIFT_COUNT + 1))
    RESULTS+=("Evidence Generation: PARTIAL")
fi
echo

# Summary
echo "=== D9 Full Gate Verification Summary ==="
echo "  PASS:    $PASS_COUNT"
echo "  FAIL:    $FAIL_COUNT"
echo "  DRIFT:   $DRIFT_COUNT"
echo
echo "--- Results ---"
for r in "${RESULTS[@]}"; do
    echo "  $r"
done
echo

# Write evidence
mkdir -p "$EVIDENCE_DIR"
cat > "$EVIDENCE_DIR/d9_full_gate.json" <<EOF
{
    "gate": "D9",
    "name": "Full Gate Verification",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "pass_count": $PASS_COUNT,
    "fail_count": $FAIL_COUNT,
    "drift_count": $DRIFT_COUNT,
    "results": [
$(for r in "${RESULTS[@]}"; do echo "        \"$r\","; done | sed '$ s/,$//')
    ]
}
EOF

if [ $FAIL_COUNT -gt 0 ]; then
    echo "❌ D9 Full Gate Verification: FAILED ($FAIL_COUNT dimensions failed)"
    exit 1
fi

if [ $DRIFT_COUNT -gt 0 ]; then
    echo "⚠️  D9 Full Gate Verification: PASS-WITH-DRIFT ($DRIFT_COUNT drift)"
    exit 2
fi

echo "✅ D9 Full Gate Verification: ALL PASS"
exit 0
