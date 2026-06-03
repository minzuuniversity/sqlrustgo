#!/usr/bin/env bash
# =============================================================================
# check_arch_sem_debt.sh — v3.8.0 D8-Architecture-Semantic-Debt Gate
# =============================================================================
# Implements 5-Principle P5 enforcement for ARCH-1~3 + SEM-1~4:
#   "未通过的必须有记录和后续改进"
#
# These 7 architecture/semantic debt items lack tracking:
#   ARCH-1: execution_engine.rs 6829 lines (since v3.0.0)
#   ARCH-2: dual path mysql-server vs bench-cli (since v2.6.0)
#   ARCH-3: VTU partial integration (since v3.5.0)
#   SEM-1: ROLLBACK MVCC stub (since v3.0.0)
#   SEM-2: SHOW TABLES partial (since v3.7.0)
#   SEM-3: ALTER TABLE incomplete (since v3.0.0)
#   SEM-4: Coverage measurement difference (since v3.0.0)
#
# This gate forces 0 OPEN or has a documented v3.9.0+ plan.
#
# Exit codes:
#   0  = 0 OPEN, all CLOSED
#   1  = ANY OPEN without plan (blocker)
#   2  = OPEN with plan (DRIFT, acceptable for v3.8.0)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

PLAN_DOC="$REPO_ROOT/docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md"
# Fallback: docs were reorganized in PR #2933 into archived/ subdir.
if [ ! -f "$PLAN_DOC" ]; then
    PLAN_DOC="$REPO_ROOT/docs/releases/v3.8.0/ARCH_SEM_DEBT_REMEDIATION_PLAN.md"
fi
CV_DEBT_DOC="$REPO_ROOT/docs/releases/v3.8.0/COMPREHENSIVE_FEATURE_TRACKING.md"

echo "=== D8: Architecture/Semantic Debt Gate (5-Principle P5) ==="
echo

# Validate input docs
if [ ! -f "$PLAN_DOC" ]; then
    echo "❌ $PLAN_DOC not found"
    exit 1
fi

# Status hard-coded from v3.8.0 audit (COMPREHENSIVE_FEATURE_TRACKING.md §4.2)
# All 7 items are OPEN per 2026-06-03 audit.
# When any item is closed, update both this gate and the audit doc.
declare -A DEBT_STATUS=(
    ["ARCH-1"]="OPEN"
    ["ARCH-2"]="OPEN"
    ["ARCH-3"]="OPEN"
    ["SEM-1"]="OPEN"
    ["SEM-2"]="OPEN"
    ["SEM-3"]="OPEN"
    ["SEM-4"]="OPEN"
)

OPEN_COUNT=0
CLOSED_COUNT=0
OPEN_WITH_PLAN=0
OPEN_WITHOUT_PLAN=0
FAILED_ITEMS=()

# 7 debt items to track
ITEMS=("ARCH-1" "ARCH-2" "ARCH-3" "SEM-1" "SEM-2" "SEM-3" "SEM-4")

for debt_id in "${ITEMS[@]}"; do
    status="${DEBT_STATUS[$debt_id]}"
    echo -n "  [$debt_id] status=$status"

    # Check for plan
    if grep -q "$debt_id" "$PLAN_DOC" 2>/dev/null; then
        has_plan=true
    else
        has_plan=false
    fi

    case "$status" in
        CLOSED)
            echo " ✅"
            CLOSED_COUNT=$((CLOSED_COUNT + 1))
            ;;
        OPEN)
            OPEN_COUNT=$((OPEN_COUNT + 1))
            if [ "$has_plan" = true ]; then
                echo " (has v3.9.0+ plan) ⚠️"
                OPEN_WITH_PLAN=$((OPEN_WITH_PLAN + 1))
            else
                echo " ❌"
                OPEN_WITHOUT_PLAN=$((OPEN_WITHOUT_PLAN + 1))
                FAILED_ITEMS+=("$debt_id (OPEN without v3.9.0+ plan)")
            fi
            ;;
        *)
            echo " (unknown status: $status) ❌"
            OPEN_WITHOUT_PLAN=$((OPEN_WITHOUT_PLAN + 1))
            FAILED_ITEMS+=("$debt_id (unknown status)")
            ;;
    esac
done

echo
echo "=== D8 Arch/Sem Debt Summary ==="
echo "  CLOSED:               $CLOSED_COUNT"
echo "  OPEN:                 $OPEN_COUNT"
echo "  OPEN w/ plan:         $OPEN_WITH_PLAN"
echo "  OPEN w/o plan:        $OPEN_WITHOUT_PLAN"
echo

# Decision logic
if [ $OPEN_WITHOUT_PLAN -gt 0 ]; then
    echo "=== Failed Items ==="
    for item in "${FAILED_ITEMS[@]}"; do
        echo "  - $item"
    done
    echo
    echo "❌ D8 Arch/Sem Debt: FAILED ($OPEN_WITHOUT_PLAN OPEN without v3.9.0+ plan)"
    echo "   5-Principle P5 violated: OPEN debt must be either:"
    echo "     1. Closed (resolved and validated)"
    echo "     2. Documented in $PLAN_DOC with v3.9.0+ plan"
    exit 1
fi

if [ $OPEN_WITH_PLAN -gt 0 ]; then
    echo "⚠️  D8 Arch/Sem Debt: PASS-WITH-DRIFT ($OPEN_WITH_PLAN OPEN with plan)"
    echo "   These items have documented v3.9.0+ plans but are not yet implemented."
    exit 2
fi

echo "✅ D8 Arch/Sem Debt: PASS (0 OPEN, all CLOSED)"
exit 0
