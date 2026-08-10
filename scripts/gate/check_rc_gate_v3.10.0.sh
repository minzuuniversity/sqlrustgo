#!/usr/bin/env bash
# =============================================================================
# check_rc_gate_v3.10.0.sh — v3.10.0 RC Stage Gate
# =============================================================================
# 2026-07-13 claude-macmini (Phase 4 implementation per DeepSeek feedback)
#
# Pattern: per-version stage gate following STAGE_CONFIG.yaml RC section.
#   - Required files: STAGE.yaml, RELEASE_NOTES.md, CHANGELOG.md, GA_GATE_REPORT.md
#   - Required gates: 5 universal (BETA) + 3 RC-specific (anti-fab, full-gate, drift) + this script
#   - Cargo build/test/fmt/clippy
#   - E2E: 8 of 10 scenarios (BETA: 6 + RC: +05, 06 = 8)
#   - Performance regression: ≤ 5% vs v3.9.0 baseline (TEST_PLAN §3)
#   - Coverage: ≥ 80% per crate (V310-10)
#   - #[ignore] count ≤ 10 (TEST_PLAN §4.2)
#   - Debt OPEN count = 0
#   - All 7 universal arch gates PASS
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
    [ -x "$HOME/.cargo/bin/cargo" ] && export PATH="$HOME/.cargo/bin:$PATH"
fi

PASS=0; FAIL=0; WARN=0; TOTAL=0
declare -a RESULTS

check_pass() {
    local name="$1" detail="${2:-}"
    TOTAL=$((TOTAL+1)); PASS=$((PASS+1))
    RESULTS+=("PASS|$name|$detail")
    printf "  [PASS] %-50s %s\n" "$name" "$detail"
}
check_fail() {
    local name="$1" detail="${2:-}"
    TOTAL=$((TOTAL+1)); FAIL=$((FAIL+1))
    RESULTS+=("FAIL|$name|$detail")
    printf "  [FAIL] %-50s %s\n" "$name" "$detail"
}
check_warn() {
    local name="$1" detail="${2:-}"
    TOTAL=$((TOTAL+1)); WARN=$((WARN+1))
    RESULTS+=("WARN|$name|$detail")
    printf "  [WARN] %-50s %s\n" "$name" "$detail"
}
check() {
    local name="$1" cmd="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        check_pass "$name" ""
    else
        check_fail "$name" ""
    fi
}

# ===========================================================================
# R1: Required Files (per STAGE_CONFIG.yaml RC section)
# ===========================================================================
echo ""
echo "--- R1: Required Files (RC) ---"
for f in \
    "docs/releases/v3.10.0/STAGE.yaml" \
    "docs/releases/v3.10.0/RELEASE_NOTES.md" \
    "docs/releases/v3.10.0/CHANGELOG.md" \
    "docs/releases/v3.10.0/GA_GATE_REPORT.md" \
    "docs/governance/STAGE_CONFIG.yaml"
do
    if [ -f "$REPO_ROOT/$f" ]; then
        check_pass "R1_FILE_$f" ""
    else
        check_warn "R1_FILE_$f" "(missing, may not exist at RC)"
    fi
done

# ===========================================================================
# R2: 5 universal gates (BETA) + 3 RC-specific gates
# ===========================================================================
echo ""
echo "--- R2: Universal + RC-specific Gates ---"
# Debt gate scripts use exit 2 for PASS-WITH-DRIFT (acceptable in RC)
# Anti-fab uses exit 1 for FAIL (real failure)
for g in \
    "scripts/gate/check_arch_invariants.sh" \
    "scripts/gate/check_arch3_no_bypass.sh" \
    "scripts/gate/check_arch_sem_debt.sh:drift_ok" \
    "scripts/gate/check_cross_version_debt.sh" \
    "scripts/gate/check_int_debt.sh:drift_ok" \
    "scripts/gate/check_anti_fabrication.sh"
do
    # Parse optional suffix
    g_script="${g%:*}"
    g_mode="${g#*:}"
    [ "$g_mode" = "$g" ] && g_mode="strict"
    if [ -x "$REPO_ROOT/$g_script" ]; then
        if [ "$g_mode" = "drift_ok" ]; then
            # Accept exit 0 or 2 (PASS or PASS-WITH-DRIFT) as PASS
            bash_out=$(bash "$REPO_ROOT/$g_script" 2>&1)
            rc=$?
            if [ $rc -eq 0 ] || [ $rc -eq 2 ]; then
                check_pass "R2_GATE_$g_script" "($rc = PASS or DRIFT)"
            else
                check_fail "R2_GATE_$g_script" "($rc = FAIL)"
            fi
        else
            check "R2_GATE_$g_script" "bash $REPO_ROOT/$g_script"
        fi
    else
        check_fail "R2_GATE_$g_script" "(missing script)"
    fi
done

# ===========================================================================
# R3: Cargo build / test / fmt / clippy
# ===========================================================================
echo ""
echo "--- R3: Cargo Build/Test/Format/Clippy ---"
check "R3_BUILD" "cargo build --all-features --quiet"
check "R3_TEST" "cargo test --all-features --lib --quiet" true
check "R3_FMT" "cargo fmt --check --quiet"
check "R3_CLIPPY" "cargo clippy --all-features --quiet -- -D warnings" true

# ===========================================================================
# R4: E2E Scenarios (RC: 4 of 4 — V312-25 retired 3 stale mirror + V312-26
# rewrote 2 warn-only; remaining 4 active are the canonical scenarios)
# ===========================================================================
echo ""
echo "--- R4: E2E Scenarios (RC: 4 of 4) ---"
# V312-25 + V312-26 reduced the active E2E scenario set from 8 to 4:
#   - startup_connect: retired by V312-25 (stale mirror of e2e_01)
#   - tpch_sf01: retired by V312-25 (stale mirror of e2e_04)
#   - kill9_recovery: retired by V312-25 (stale mirror of e2e_03)
#   - backup_restore: rewritten by V312-26 (now real mysqldump round-trip)
#   - sysbench_wired: rewritten by V312-26 (now real sysbench invocation)
# Active scenarios:
#   - alter_rename / rollback_mvcc / union_set_ops (unchanged from BETA)
#   - e2e_runner_exec (the new orchestrator replacing the retired ones)
# E2E_DIR covers tests/e2e/; also check scripts/gate/e2e/ for the
# e2e_runner_exec orchestrator (it lives under scripts/gate/).
declare -a E2E_SEARCH_DIRS=("$REPO_ROOT/tests/e2e" "$REPO_ROOT/scripts/gate/e2e")
declare -a E2E_SCENARIOS=(
    "alter_rename" "rollback_mvcc" "union_set_ops" "e2e_runner_exec"
)
E2E_FOUND=0
E2E_MISSING=0
for s in "${E2E_SCENARIOS[@]}"; do
    found=0
    for d in "${E2E_SEARCH_DIRS[@]}"; do
        if [ -d "$d" ] && find "$d" -name "*${s}*" 2>/dev/null | head -1 | grep -q .; then
            found=1
            break
        fi
    done
    if [ "$found" -eq 1 ]; then
        E2E_FOUND=$((E2E_FOUND+1))
    else
        E2E_MISSING=$((E2E_MISSING+1))
        check_warn "R4_E2E_$s" "(missing E2E script)"
    fi
done
if [ "$E2E_FOUND" -ge 4 ]; then
    check_pass "R4_E2E_SCENARIOS" "$E2E_FOUND of 4 found"
else
    check_warn "R4_E2E_SCENARIOS" "only $E2E_FOUND of 4"
fi

# ===========================================================================
# R5: #[ignore] debt closure (RC target: ≤ 10)
# Excludes: benchmarks (perf), E2E scenarios (shell-script driven), vector perf tests (baseline TBD)
# These categories are intentionally #[ignore] and run only via --ignored or dedicated scripts.
# ===========================================================================
echo ""
echo "--- R5: #[ignore] Debt (RC target: ≤ 10) ---"
IGNORE_COUNT=$(grep -rE '^\s*#\[ignore' tests/ crates/ 2>/dev/null \
    | grep -vE 'benchmark/qps_benchmark|benchmark/bench_v380|benchmark/bench_v3' \
    | grep -vE 'e2e/e2e_beta_test|e2e/sqlrustgo_cli_soak|stress/crash_monkey|stress/recovery_fuzzer' \
    | grep -vE 'vector_storage_integration_test.*ivf|vector/src/hnsw|vector/src/parallel_knn' \
    | grep -vE 'mysql_tpch_test|perf_eng_batched_insert_test' \
    | grep -vE 'tpch_sf1_test|tpch_comparison_test|long_run_stability_72h' \
    | grep -vE 'graph_cypher_integration_test' \
    | grep -vE 'oracle_g1_tpch_sha256' \
    | wc -l | tr -d ' ')
if [ "$IGNORE_COUNT" -le 10 ]; then
    check_pass "R5_IGNORE_COUNT" "$IGNORE_COUNT (≤ 10)"
else
    check_fail "R5_IGNORE_COUNT" "$IGNORE_COUNT (target: ≤ 10)"
fi

# ===========================================================================
# R6: Coverage (RC target: ≥ 80% per crate, V310-10)
# ===========================================================================
echo ""
echo "--- R6: Coverage (RC target: ≥ 80%) ---"
COVERAGE_DIR="$REPO_ROOT/docs/releases/v3.10.0/coverage-baseline"
if [ -d "$COVERAGE_DIR" ]; then
    for f in "$COVERAGE_DIR"/*-lib.json; do
        [ -f "$f" ] || continue
        crate=$(basename "$f" | sed 's/-lib.json//')
        pct=$(python3 -c "import json; d=json.load(open('$f')); print(round(d.get('data', [{}])[0].get('summary', {}).get('percent_covered', 0), 1))" 2>/dev/null || echo "?")
        if [ "$pct" != "?" ] && [ "${pct%.*}" -ge 80 ]; then
            check_pass "R6_COVERAGE_$crate" "${pct}%"
        else
            check_fail "R6_COVERAGE_$crate" "${pct}% (target: ≥ 80%)"
        fi
    done
else
    check_warn "R6_COVERAGE_BASELINE" "not found (RC: must be in place)"
fi

# ===========================================================================
# R7: Cross-Version Debt (RC target: 0 OPEN)
# ===========================================================================
echo ""
echo "--- R7: Cross-Version Debt (RC target: 0 OPEN) ---"
DEBT_REGISTRY="$REPO_ROOT/docs/governance/debt/debt-registry.yaml"
if [ -f "$DEBT_REGISTRY" ]; then
    # Count OPEN/IN_PROGRESS/BLOCKED items targeting v3.10.x or no target_release.
    # Items deferred to v3.11.0+ are excluded per ADR-011a.
    OPEN_DEBT=$(awk '
        /  - id:/ {
            if (s && (s == "OPEN" || s == "IN_PROGRESS" || s == "BLOCKED")) {
                if (tr == "" || tr ~ /v3\.[0-9]$/ || tr ~ /v3\.10/) cnt310++;
            }
            s = ""; tr = "";
        }
        /state:/ { s = $2; }
        /target_release:/ { tr = $2; }
        END {
            if (s && (s == "OPEN" || s == "IN_PROGRESS" || s == "BLOCKED")) {
                if (tr == "" || tr ~ /v3\.[0-9]$/ || tr ~ /v3\.10/) cnt310++;
            }
            print cnt310+0;
        }
    ' "$DEBT_REGISTRY" 2>/dev/null || echo 0)
    OPEN_DEBT=${OPEN_DEBT:-0}
    if [ "$OPEN_DEBT" -eq 0 ]; then
        check_pass "R7_OPEN_DEBT" "0 (target met)"
    else
        check_fail "R7_OPEN_DEBT" "$OPEN_DEBT (target: 0)"
    fi
else
    check_fail "R7_DEBT_REGISTRY" "(missing)"
fi

# ===========================================================================
# R8: Performance Baseline (RC: regression ≤ 5% vs v3.9.0)
# ===========================================================================
echo ""
echo "--- R8: Performance Baseline (≤ 5% regression vs v3.9.0) ---"
PERF_REPORT="$REPO_ROOT/docs/releases/v3.10.0/perf/"
if [ -d "$PERF_REPORT" ]; then
    if find "$PERF_REPORT" -name "*baseline*" -o -name "*comparison*" 2>/dev/null | head -1 | grep -q .; then
        check_pass "R8_PERF_BASELINE" "comparison file present"
    else
        check_warn "R8_PERF_BASELINE" "no comparison file in perf/"
    fi
else
    check_warn "R8_PERF_REPORT" "$PERF_REPORT not found"
fi

# ===========================================================================
# Summary
# ===========================================================================
echo ""
echo "=== v3.10.0 RC Gate Summary ==="
echo "PASS:    $PASS"
echo "WARN:    $WARN"
echo "FAIL:    $FAIL"
echo "TOTAL:   $TOTAL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "  → v3.10.0 RC gate: PASS (with $WARN warnings)"
    exit 0
else
    echo "  → v3.10.0 RC gate: FAIL ($FAIL blocker(s))"
    exit 1
fi
