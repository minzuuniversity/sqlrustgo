#!/usr/bin/env bash
# check_4019_sysbench_baseline.sh — V312-26 Issue #4019 Sysbench OLTP Baseline Gate
#
# Verifies the Sysbench OLTP baseline capture infrastructure for #4019:
# 1. scripts/sysbench/run_baseline.sh exists and has valid bash syntax
# 2. The script has the two required baseline functions (cpu_baseline,
#    oltp_read_only_baseline) and the prepare_db helper
# 3. The script uses per-step timeout to bound wire-protocol hangs
# 4. Evidence directory exists with at least one captured run
# 5. The captured run has a real cpu baseline (events/sec line in sysbench_cpu.log)
#    OR a documented honesty step (SKIPPED log + .rc_prepare_db failure markers)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# IMPORTANT (STRICT PROOF MODE): this gate does NOT claim oltp_read_only
# passed. It only verifies the infrastructure and recorded result codes. The
# 4019_evidence.md is the authoritative record of what was actually captured.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== V312-26 #4019 Sysbench OLTP Baseline Gate ==="
echo ""

PASS=0
FAIL=0

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  [PASS] $name"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] $name"
        FAIL=$((FAIL+1))
    fi
}

# 1. Runner script exists
echo "--- Runner Script ---"
if [ -f "scripts/sysbench/run_baseline.sh" ]; then
    echo "  [PASS] scripts/sysbench/run_baseline.sh exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/sysbench/run_baseline.sh missing"
    FAIL=$((FAIL+1))
fi

check "run_baseline.sh syntax valid" "bash -n scripts/sysbench/run_baseline.sh"
check "run_baseline.sh has cpu_baseline function" "grep -q 'cpu_baseline()' scripts/sysbench/run_baseline.sh"
check "run_baseline.sh has prepare_db function" "grep -q 'prepare_db()' scripts/sysbench/run_baseline.sh"
check "run_baseline.sh has oltp_read_only_baseline function" "grep -q 'oltp_read_only_baseline()' scripts/sysbench/run_baseline.sh"
check "run_baseline.sh has per-step timeout (wire-protocol hardening)" "grep -q 'timeout.*QUERY_TIMEOUT_SEC' scripts/sysbench/run_baseline.sh"
check "run_baseline.sh writes metadata.json" "grep -q 'write_metadata()' scripts/sysbench/run_baseline.sh"
check "run_baseline.sh records .rc_cpu_baseline" "grep -q '.rc_cpu_baseline' scripts/sysbench/run_baseline.sh"
check "run_baseline.sh records .rc_prepare_db_create_db" "grep -q '.rc_prepare_db_create_db' scripts/sysbench/run_baseline.sh"
check "run_baseline.sh handles oltp_read_only skip gracefully" "grep -q 'SKIPPED' scripts/sysbench/run_baseline.sh"

# 2. Evidence directory
echo ""
echo "--- Evidence Directory ---"
EVIDENCE_DIR="docs/releases/v3.12.0/evidence/issue-4019"
if [ -d "$EVIDENCE_DIR" ]; then
    echo "  [PASS] $EVIDENCE_DIR exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] $EVIDENCE_DIR missing"
    FAIL=$((FAIL+1))
fi

# 3. At least one captured run
LATEST_RUN=$(ls -1dt "$EVIDENCE_DIR"/*/ 2>/dev/null | head -1 || true)
if [ -n "$LATEST_RUN" ]; then
    echo "  [PASS] latest run dir: $LATEST_RUN"
    PASS=$((PASS+1))
    if [ -f "$LATEST_RUN/metadata.json" ]; then
        echo "  [PASS] metadata.json present"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] metadata.json missing in $LATEST_RUN"
        FAIL=$((FAIL+1))
    fi
    if [ -f "$LATEST_RUN/summary.txt" ]; then
        echo "  [PASS] summary.txt present"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] summary.txt missing in $LATEST_RUN"
        FAIL=$((FAIL+1))
    fi
    if [ -f "$LATEST_RUN/sysbench_cpu.log" ]; then
        echo "  [PASS] sysbench_cpu.log present"
        PASS=$((PASS+1))
        if grep -q "events per second:" "$LATEST_RUN/sysbench_cpu.log" 2>/dev/null; then
            local_eps=$(grep "events per second:" "$LATEST_RUN/sysbench_cpu.log" | tail -1 | awk '{print $NF}')
            echo "  [PASS] cpu baseline captured: ${local_eps:-N/A} events/sec"
            PASS=$((PASS+1))
        else
            echo "  [FAIL] sysbench_cpu.log has no events-per-second line"
            FAIL=$((FAIL+1))
        fi
    else
        echo "  [FAIL] sysbench_cpu.log missing in $LATEST_RUN"
        FAIL=$((FAIL+1))
    fi
    # .rc files are mandatory for honest fail-explicit mode
    if [ -f "$LATEST_RUN/.rc_cpu_baseline" ]; then
        echo "  [PASS] .rc_cpu_baseline recorded ($(cat "$LATEST_RUN/.rc_cpu_baseline" 2>/dev/null))"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] .rc_cpu_baseline missing (fail-explicit mode requires step rc)"
        FAIL=$((FAIL+1))
    fi
    if [ -f "$LATEST_RUN/.rc_prepare_db" ] || [ -f "$LATEST_RUN/.rc_prepare_db_create_db" ]; then
        echo "  [PASS] prepare_db rc recorded (honest step tracking)"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] prepare_db rc missing — gate cannot verify what was attempted"
        FAIL=$((FAIL+1))
    fi
else
    echo "  [FAIL] no captured run in $EVIDENCE_DIR"
    FAIL=$((FAIL+1))
fi

# 4. Evidence doc (the authoritative record)
echo ""
echo "--- Evidence Document ---"
EVIDENCE_DOC="docs/releases/v3.12.0/evidence/issue-4019/4019_evidence.md"
if [ -f "$EVIDENCE_DOC" ]; then
    echo "  [PASS] $EVIDENCE_DOC exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] $EVIDENCE_DOC missing — STRICT PROOF MODE requires the doc"
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== #4019 Sysbench Baseline Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ #4019 gate PASSED (infrastructure verified — see 4019_evidence.md for actual results)"
    exit 0
else
    echo "❌ #4019 gate FAILED ($FAIL blocker(s))"
    exit 1
fi
