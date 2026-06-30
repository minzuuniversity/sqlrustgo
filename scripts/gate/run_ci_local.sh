#!/bin/bash
# run_ci_local.sh — local CI-style gate runner
#
# Simulates a CI run by invoking each gate in check_g_all.sh
# with per-gate timeouts, skipping gates that need hardware not
# available on this machine (Z6G4/W12).
#
# This is NOT a substitute for actual CI. Output should be treated
# as "local smoke test of the gate pipeline", not as final GA sign-off.
#
# Usage:
#   bash scripts/gate/run_ci_local.sh           # default 600s/gate
#   TIMEOUT=300 bash scripts/gate/run_ci_local.sh  # 5min/gate
#
# Refs: docs/releases/v3.9.0/alpha/ALPHA_GATE_CONTRACT.md
#       openspec/changes/fix-gate-pre-existing-failures/

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

PER_GATE_TIMEOUT=${TIMEOUT:-600}  # 10min default per gate
REPORT_FILE="${REPORT_FILE:-artifacts/ci_local_run_$(date +%Y%m%d_%H%M%S).log}"
mkdir -p "$(dirname "$REPORT_FILE")"

# Format: gate|script|hardware_required|timeout_override
# hardware_required=yes -> skip on local (CI-only)
# hardware_required=no  -> run with timeout
GATES=(
    "G1|22/22 TPC-H|check_g1_tpch_22_22.sh|no|120"
    "G2|INT-2|check_int2_no_orphan.sh|no|60"
    "G3|INT-3|check_int3_single_expr.sh|no|60"
    "G4|ARCH-3|check_arch3_no_bypass.sh|no|60"
    "G5|SEM-1|check_sem1_savepoint.sh|no|60"
    "G6|Backup/Restore|check_backup_restore.sh|no|300"
    "G7|24h Soak|check_p13_soak_test.sh|no|60"
    "G8|Crash Matrix|check_p12_crash_test.sh|no|300"
    "G9|Upgrade|check_p14_upgrade_test.sh|no|300"
    "G11|QPS|check_g11_qps.sh|yes|n/a"
    "G12|Sysbench|check_g12_sysbench.sh|yes|n/a"
    "G13|24h Stability|check_g13_stability.sh|no|120"
    "G14|Real Crash|check_g14_real_crash.sh|yes|n/a"
    "G15|Perf Report|check_g15_perf_report.sh|no|60"
    "G16|Compatibility|check_g16_compatibility.sh|no|300"
)

PASSED=()
FAILED=()
SKIPPED=()
TIMED_OUT=()

echo "=== Local CI-style Gate Run ==="
echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Per-gate timeout: ${PER_GATE_TIMEOUT}s (some gates have shorter timeouts)"
echo "Hardware-required gates: SKIPPED (CI-only on Z6G4/W12)"
echo "Output: $REPORT_FILE"
echo

for entry in "${GATES[@]}"; do
    IFS='|' read -r gate label script hw gate_timeout <<< "$entry"
    if [ "$hw" = "yes" ]; then
        echo "[$gate] SKIP ($label — requires $script hardware)"
        SKIPPED+=("$gate")
        continue
    fi
    effective_timeout=$gate_timeout
    [ "$effective_timeout" -gt "$PER_GATE_TIMEOUT" ] && effective_timeout=$PER_GATE_TIMEOUT

    script_path="scripts/gate/$script"
    if [ ! -f "$script_path" ]; then
        echo "[$gate] FAIL ($label — script $script missing)"
        FAILED+=("$gate")
        continue
    fi

    echo -n "[$gate] running ($label, ${effective_timeout}s timeout) ... "
    start=$(date +%s)
    # Run with timeout, capture exit code and output
    set +e
    output=$(timeout "$effective_timeout" bash "$script_path" 2>&1)
    rc=$?
    set -e
    elapsed=$(( $(date +%s) - start ))

    if [ $rc -eq 0 ]; then
        echo "PASS (${elapsed}s)"
        PASSED+=("$gate")
    elif [ $rc -eq 124 ]; then
        echo "TIMEOUT (${elapsed}s, killed)"
        TIMED_OUT+=("$gate")
    else
        echo "FAIL (exit=$rc, ${elapsed}s)"
        # Capture last 5 lines of failure output
        echo "$output" | tail -5 | sed 's/^/    /'
        FAILED+=("$gate")
    fi
    # Append to log
    {
        echo "=== $gate ($label) — $(date -u +%H:%M:%S) — exit=$rc, ${elapsed}s ==="
        echo "$output" | tail -20
        echo
    } >> "$REPORT_FILE"
done

echo
echo "=== Summary ==="
echo "Started:  $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Total:    $((${#PASSED[@]} + ${#FAILED[@]} + ${#SKIPPED[@]} + ${#TIMED_OUT[@]}))"
echo "Passed:   ${#PASSED[@]} (${PASSED[*]:-none})"
echo "Failed:   ${#FAILED[@]} (${FAILED[*]:-none})"
echo "Skipped:  ${#SKIPPED[@]} (${SKIPPED[*]:-none}) [hardware required]"
echo "TimedOut: ${#TIMED_OUT[@]} (${TIMED_OUT[*]:-none})"
echo "Log:      $REPORT_FILE"

# Exit non-zero if any gate failed or timed out
[ ${#FAILED[@]} -eq 0 ] && [ ${#TIMED_OUT[@]} -eq 0 ]
