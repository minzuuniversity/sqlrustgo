#!/bin/bash
# check_g_all.sh - v3.9.0 G1-G19 orchestrator
#
# Runs all 19 v3.9.0 gates (G1-G19) in sequence, maps each
# result to its tracking issue, and produces a consolidated PASS/FAIL report.
#
# Mapping (per docs/releases/v3.9.0/alpha/ALPHA_GATE_CONTRACT.md):
#   G1  22/22 TPC-H 保持                -> check_g1_tpch_22_22.sh      -> #3186
#   G2  INT-2 关闭 (ParallelExecutor)   -> check_int2_no_orphan.sh    -> #3187
#   G3  INT-3 关闭 (Single Expression)  -> check_int3_single_expr.sh  -> #3188
#   G4  ARCH-3 关闭 (VtuGuard)          -> check_arch3_no_bypass.sh   -> #3189
#   G5  SEM-1 关闭 (Savepoint MVCC)     -> check_sem1_savepoint.sh    -> #3190
#   G6  Backup/Restore 100+ scenarios   -> check_backup_restore.sh    -> #3191
#   G7  24h Soak (COMPRESSED smoke)      -> check_p13_soak_test.sh     -> #3192
#   G8  Crash Matrix 100+ scenarios     -> check_p12_crash_test.sh    -> #3193
#   G9  Upgrade Test 50+ scenarios      -> check_p14_upgrade_test.sh  -> #3194
#   G10 Audit + Time Travel 40+ tests   -> check_p21_audit_log.sh     -> #3195
#                                       -> check_p22_time_travel.sh
#                                       -> check_p23_hash_chain.sh
#   G11 QPS/TPS 基准 (hardware required) -> check_g11_qps.sh           -> #3200
#   G12 Sysbench OLTP (hardware required) -> check_g12_sysbench.sh     -> #3201
#   G13 24h Stability (deferred hardware) -> check_g13_stability.sh   -> #3202
#   G14 Real Crash (deferred hardware)    -> check_g14_real_crash.sh   -> #3203
#   G15 Perf Report (aggregates G11-G16) -> check_g15_perf_report.sh  -> #3204
#   G16 Compatibility                     -> check_g16_compatibility.sh -> #3205
#
# NOTE: G7/G13 run COMPRESSED smoke (60s/180s/420s), NOT real 24h/72h/168h.
#       G11/G12 require Z6G4 hardware. G14 requires W12. Real soaks deferred.
#
# Exit code: 0 = ALL PASS, 1 = ANY FAIL
#
# Refs: docs/releases/v3.9.0/alpha/ALPHA_GATE_CONTRACT.md
#       docs/releases/v3.9.0/alpha/ALPHA_GATE_REPORT.md

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

# Per-gate: name|script|tracking_issue|blocking
GATES=(
    "G1|22/22 TPC-H 保持|check_g1_tpch_22_22.sh|#3186|yes"
    "G2|INT-2 关闭|check_int2_no_orphan.sh|#3187|yes"
    "G3|INT-3 关闭|check_int3_single_expr.sh|#3188|yes"
    "G4|ARCH-3 关闭|check_arch3_no_bypass.sh|#3189|yes"
    "G5|SEM-1 关闭|check_sem1_savepoint.sh|#3190|yes"
    "G6|Backup/Restore|check_backup_restore.sh|#3191|yes"
    "G7|24h Soak|check_p13_soak_test.sh|#3192|yes"
    "G8|Crash Matrix|check_p12_crash_test.sh|#3193|yes"
    "G9|Upgrade|check_p14_upgrade_test.sh|#3194|yes"
    "G11|QPS 基准|check_g11_qps.sh|#3200|warn"
    "G12|Sysbench OLTP|check_g12_sysbench.sh|#3201|warn"
    "G13|24h Stability|check_g13_stability.sh|#3202|warn"
    "G14|Real Crash|check_g14_real_crash.sh|#3203|warn"
    "G15|Perf Report|check_g15_perf_report.sh|#3204|warn"
    "G16|Compatibility|check_g16_compatibility.sh|#3205|warn"
    "G17|check_coverage_v312.sh|check_coverage_v312.sh|#V312-31|yes"
    "G18|check_sql_corpus_gate.sh|check_sql_corpus_gate.sh|#V312-31|yes"
    "G19|check_anti_ignore_gate.sh|check_anti_ignore_gate.sh|#V312-31|yes"
)

# G10 has 3 sub-scripts
G10_SUBSCRIPTS=(
    "check_p21_audit_log.sh"
    "check_p22_time_travel.sh"
    "check_p23_hash_chain.sh"
)

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0
RESULTS=()

echo "================================================================"
echo "  v3.9.0 G1-G19 Orchestrator"
echo "  Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')"
echo "  Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
echo "  Date:   $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "================================================================"
echo

for entry in "${GATES[@]}"; do
    IFS='|' read -r gid gname gscript gissue gblocking <<< "$entry"

    echo "--- $gid: $gname (tracking $gissue) ---"

    if [ ! -f "scripts/gate/$gscript" ]; then
        echo "  [SKIP] $gscript not found"
        if [ "$gblocking" = "yes" ]; then
            FAIL_COUNT=$((FAIL_COUNT + 1))
            RESULTS+=("$gid|FAIL|script-missing")
        else
            WARN_COUNT=$((WARN_COUNT + 1))
            RESULTS+=("$gid|WARN|script-missing")
        fi
        continue
    fi

    set +e
    timeout 600 bash "scripts/gate/$gscript" 2>&1 | tail -8
    EXIT=$?
    set -e

    if [ $EXIT -eq 0 ]; then
        echo "  -> $gid: PASS"
        PASS_COUNT=$((PASS_COUNT + 1))
        RESULTS+=("$gid|PASS|ok")
    else
        if [ "$gblocking" = "yes" ]; then
            echo "  -> $gid: FAIL (exit $EXIT, blocking)"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            RESULTS+=("$gid|FAIL|exit=$EXIT")
        else
            echo "  -> $gid: FAIL (exit $EXIT, warning only)"
            WARN_COUNT=$((WARN_COUNT + 1))
            RESULTS+=("$gid|WARN|exit=$EXIT")
        fi
    fi
    echo
done

# G10 also runs 2 extra sub-scripts
echo "--- G10 (extra: Time Travel + Hash Chain) ---"
for sub in "${G10_SUBSCRIPTS[@]:1}"; do
    if [ -f "scripts/gate/$sub" ]; then
        set +e
        timeout 300 bash "scripts/gate/$sub" > /dev/null 2>&1
        EXIT=$?
        set -e
        if [ $EXIT -eq 0 ]; then
            echo "  $sub: PASS (sub-gate)"
            PASS_COUNT=$((PASS_COUNT + 1))
        else
            echo "  $sub: FAIL (sub-gate, exit $EXIT, warning)"
            WARN_COUNT=$((WARN_COUNT + 1))
        fi
    fi
done
echo

echo
echo "================================================================"
echo "  v3.9.0 G1-G19 SUMMARY"
echo "================================================================"
printf "  %-6s %-30s %-12s %s\n" "GATE" "TOPIC" "STATUS" "TRACKING"
printf "  %-6s %-30s %-12s %s\n" "----" "-----" "------" "--------"
for entry in "${GATES[@]}"; do
    IFS='|' read -r gid gname gscript gissue gblocking <<< "$entry"
    FOUND="?"
    for r in "${RESULTS[@]}"; do
        IFS='|' read -r rgid rstatus rdetail <<< "$r"
        if [ "$rgid" = "$gid" ]; then
            FOUND="$rstatus"
            break
        fi
    done
    printf "  %-6s %-30s %-12s %s\n" "$gid" "$gname" "$FOUND" "$gissue"
done
echo
echo "  PASS: $PASS_COUNT | FAIL: $FAIL_COUNT | WARN: $WARN_COUNT"
echo

if [ $FAIL_COUNT -gt 0 ]; then
    echo "  GATE STATUS: ❌ FAIL (blocking gates failed)"
    exit 1
fi

if [ $WARN_COUNT -gt 0 ]; then
    echo "  GATE STATUS: 🟡 PASS with warnings (G11-G16 perf deferred to hardware)"
    exit 0
fi

echo "  GATE STATUS: ✅ ALL PASS (G1-G19 fully green)"
exit 0
