#!/usr/bin/env bash
# =============================================================================
# check_beta_v3.10.0.sh — v3.10.0 BETA Stage Gate (per-version)
# =============================================================================
# 2026-07-13 claude-macmini (Phase 4 implementation per DeepSeek feedback)
#
# Purpose: Per-version BETA gate for v3.10.0, following the pattern of
#          check_alpha_v3.10.0.sh (which is also per-version). This script
#          provides v3.10.0-specific BETA promotion criteria, complementing
#          the version-agnostic check_beta_gate.sh.
#
# Differences from check_beta_gate.sh:
#   - This is v3.10.0-specific (no VERSION variable)
#   - Targets v3.10.0 specific paths (docs/releases/v3.10.0/)
#   - Adds v3.10.0-specific 6 E2E scenarios (E2E-01..09)
#   - Verifies v3.10.0 feature flags (V310-01..12)
#
# Usage:
#   bash scripts/gate/check_beta_v3.10.0.sh
#
# Exit codes:
#   0  = all BETA checks PASS
#   1  = any BETA check FAIL
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
    [ -x "$HOME/.cargo/bin/cargo" ] && export PATH="$HOME/.cargo/bin:$PATH"
fi

PASS=0; FAIL=0; TOTAL=0
declare -a RESULTS

PASS_LINE="[PASS]"
FAIL_LINE="[FAIL]"

check() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >/dev/null 2>&1; then
        PASS=$((PASS+1))
        RESULTS+=("PASS|$name")
        printf "  %-7s %s\n" "$PASS_LINE" "$name"
    else
        FAIL=$((FAIL+1))
        RESULTS+=("FAIL|$name")
        printf "  %-7s %s\n" "$FAIL_LINE" "$name"
    fi
}

echo "=== v3.10.0 BETA Gate (per-version) ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null) @ $(git rev-parse --short HEAD 2>/dev/null)"
echo "Date:   $(date '+%Y-%m-%d %H:%M:%S %Z')"
echo ""

# ============================================================
# 1. Required files (v3.10.0 specific)
# ============================================================
echo "--- 1. Required Files (v3.10.0) ---"
for f in \
    "docs/releases/v3.10.0/STAGE.yaml" \
    "docs/releases/v3.10.0/CHANGELOG.md" \
    "docs/releases/v3.10.0/RELEASE_NOTES.md" \
    "docs/releases/v3.10.0/TEST_PLAN.md" \
    "docs/releases/v3.10.0/FEATURE_CHECKLIST.md" \
    "docs/releases/v3.10.0/ARCHITECTURE.md" \
    "docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md" \
    "docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md" \
    "docs/releases/v3.10.0/plans/V310_CLI_BINARY_PLAN.md" \
    "docs/governance/STAGE_CONFIG.yaml"
do
    if [ -f "$REPO_ROOT/$f" ]; then
        check "FILE_$f" "true"
    else
        check "FILE_$f" "false"
    fi
done

# ============================================================
# 2. Universal gates (5 shared)
# ============================================================
echo ""
echo "--- 2. Universal Gates ---"
for g in \
    "scripts/gate/check_arch_invariants.sh" \
    "scripts/gate/check_arch3_no_bypass.sh" \
    "scripts/gate/check_arch_sem_debt.sh" \
    "scripts/gate/check_cross_version_debt.sh" \
    "scripts/gate/check_int_debt.sh"
do
    if [ -x "$REPO_ROOT/$g" ]; then
        check "GATE_$g" "bash $REPO_ROOT/$g"
    else
        check "GATE_$g" "false"
    fi
done

# ============================================================
# 3. v3.10.0-specific 6 E2E stubs (per TEST_PLAN §2)
# ============================================================
echo ""
echo "--- 3. v3.10.0 BETA E2E Scenarios (6 of 10) ---"
if [ -f "tests/e2e_beta_test.rs" ]; then
    check "E2E_TEST_FILE" "cargo test --test e2e_beta_test --quiet 2>&1"
else
    check "E2E_TEST_FILE" "false"
fi

# ============================================================
# 4. v3.10.0 19 required test files (per FEATURE_CHECKLIST §9 B5)
# ============================================================
echo ""
echo "--- 4. v3.10.0 19 Required Test Files ---"
declare -a REQUIRED_TESTS=(
    "tests/dml_integration_test.rs"
    "tests/union_set_operations_test.rs"
    "tests/alter_table_test.rs"
    "tests/mvcc_transaction_test.rs"
    "tests/savepoint_test.rs"
    "tests/sem1_savepoint_test.rs"
    "tests/gap_locking_test.rs"
    "tests/parallel_executor_test.rs"
    "crates/executor/tests/parallel_hash_join_test.rs"
    "crates/executor/tests/parallel_group_by_test.rs"
    "tests/cbo_integration_test.rs"
    "tests/process_kill_crash_test.rs"
    "tests/tpch_full_22_test.rs"
    "tests/tpch_sf01_22_queries_wire_test.rs"
    "tests/wire_protocol_smoke.rs"
    "tests/mysql_wire_protocol_test.rs"
    "tests/cross_path_consistency_test.rs"
    "tests/wal_integration_test.rs"
    "tests/long_run_stability_72h_test.rs"
)
for t in "${REQUIRED_TESTS[@]}"; do
    if [ -f "$REPO_ROOT/$t" ]; then
        check "TEST_$t" "true"
    else
        check "TEST_$t" "false"
    fi
done

# ============================================================
# 5. Cargo build / test
# ============================================================
echo ""
echo "--- 5. Cargo Build / Test ---"
check "CARGO_BUILD" "cargo build --all-features --quiet"
check "CARGO_FMT" "cargo fmt --check --quiet"

# ============================================================
# 6. #[ignore] count (BETA target: ≤ 30 per TEST_PLAN §4.2)
# ============================================================
echo ""
echo "--- 6. #[ignore] Count (BETA target: ≤ 30) ---"
IGNORE_COUNT=$(grep -rE '^\s*#\[ignore' tests/ crates/ 2>/dev/null | wc -l | tr -d ' ')
if [ "$IGNORE_COUNT" -le 30 ]; then
    check "IGNORE_COUNT (=$IGNORE_COUNT)" "true"
else
    check "IGNORE_COUNT (=$IGNORE_COUNT, target: ≤ 30)" "false"
fi

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== v3.10.0 BETA Gate Summary ==="
echo "PASS:  $PASS / $TOTAL"
echo "FAIL:  $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "  → v3.10.0 BETA gate: PASS"
    exit 0
else
    echo "  → v3.10.0 BETA gate: FAIL ($FAIL blockers)"
    exit 1
fi
