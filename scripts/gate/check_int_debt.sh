#!/usr/bin/env bash
# =============================================================================
# check_int_debt.sh — v3.8.0 D7-Cross-Version-INT-Debt Gate
# =============================================================================
# Implements 5-Principle P5 enforcement for INT-1~INT-4:
#   "未通过的必须有记录和后续改进"
#
# These 4 ACTIVE cross-version debt items have been ignored for 5-7 versions:
#   INT-1: DML 不经过 WAL/TransactionManager (v1.2.0+, 7 versions)
#   INT-2: ParallelVolcanoExecutor 孤岛 (v2.6.0+, 5 versions)
#   INT-3: expr crate 功能孤岛 (v3.0.0+, 3 versions)
#   INT-4: mysql-server 未与主 server 集成 (v2.6.0+, 5 versions)
#
# This gate FORCES 0 ACTIVE or has a documented v3.9.0+ plan.
#
# Exit codes:
#   0  = 0 ACTIVE, all CLOSED or DEFERRED with v3.9.0+ plans
#   1  = ANY ACTIVE without plan (blocker)
#   2  = DRIFT (deferred but no v3.9.0+ plan linked)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

CROSS_VERSION_DEBT_DOC="$REPO_ROOT/docs/releases/v3.8.0/CROSS-VERSION-DEBT.md"
DEBT_PLAN_DOC="$REPO_ROOT/docs/releases/v3.8.0/INT_DEBT_REMEDIATION_PLAN.md"

echo "=== D7: Cross-Version INT Debt Gate (5-Principle P5) ==="
echo

# Validate input docs
if [ ! -f "$CROSS_VERSION_DEBT_DOC" ]; then
    echo "❌ $CROSS_VERSION_DEBT_DOC not found"
    exit 1
fi

ACTIVE_COUNT=0
CLOSED_COUNT=0
DEFERRED_WITH_PLAN=0
DEFERRED_WITHOUT_PLAN=0
FAILED_ITEMS=()

for int_id in INT-1 INT-2 INT-3 INT-4; do
    # Find status in CROSS-VERSION-DEBT.md
    status_line=$(grep -E "^\| $int_id \|" "$CROSS_VERSION_DEBT_DOC" | head -1)
    if [ -z "$status_line" ]; then
        echo "  [$int_id] NOT FOUND in $CROSS_VERSION_DEBT_DOC"
        FAILED_ITEMS+=("$int_id (not in CV debt doc)")
        continue
    fi

    # Extract status (last field before trailing empty)
    status=$(echo "$status_line" | awk -F'|' '{for(i=NF;i>=1;i--) if(length($i)>0) {print $i; exit}}' | xargs)
    echo -n "  [$int_id] status=$status"

    case "$status" in
        CLOSED)
            echo " ✅"
            CLOSED_COUNT=$((CLOSED_COUNT + 1))
            ;;
        ACTIVE)
            # Check for remediation plan
            if grep -q "$int_id" "$DEBT_PLAN_DOC" 2>/dev/null; then
                echo " (has v3.9.0+ plan) ⚠️"
                DEFERRED_WITH_PLAN=$((DEFERRED_WITH_PLAN + 1))
            else
                echo " ❌"
                ACTIVE_COUNT=$((ACTIVE_COUNT + 1))
                FAILED_ITEMS+=("$int_id (ACTIVE without v3.9.0+ plan)")
            fi
            ;;
        DEFERRED)
            if grep -q "$int_id" "$DEBT_PLAN_DOC" 2>/dev/null; then
                echo " (deferred with plan) ⚠️"
                DEFERRED_WITH_PLAN=$((DEFERRED_WITH_PLAN + 1))
            else
                echo " ❌"
                DEFERRED_WITHOUT_PLAN=$((DEFERRED_WITHOUT_PLAN + 1))
                FAILED_ITEMS+=("$int_id (DEFERRED without plan)")
            fi
            ;;
        *)
            echo " (unknown status: $status)"
            FAILED_ITEMS+=("$int_id (unknown status)")
            ;;
    esac
done

echo
echo "=== D7 INT Debt Summary ==="
echo "  CLOSED:               $CLOSED_COUNT"
echo "  ACTIVE:               $ACTIVE_COUNT"
echo "  DEFERRED w/ plan:     $DEFERRED_WITH_PLAN"
echo "  DEFERRED w/o plan:    $DEFERRED_WITHOUT_PLAN"
echo

# Decision logic
if [ $ACTIVE_COUNT -gt 0 ] || [ $DEFERRED_WITHOUT_PLAN -gt 0 ]; then
    echo "=== Failed Items ==="
    for item in "${FAILED_ITEMS[@]}"; do
        echo "  - $item"
    done
    echo
    echo "❌ D7 INT Debt: FAILED ($ACTIVE_COUNT ACTIVE, $DEFERRED_WITHOUT_PLAN DEFERRED w/o plan)"
    echo "   5-Principle P5 violated: ACTIVE debt must be either:"
    echo "     1. Closed (implemented and tested)"
    echo "     2. Deferred with v3.9.0+ plan in $DEBT_PLAN_DOC"
    exit 1
fi

if [ $DEFERRED_WITH_PLAN -gt 0 ]; then
    echo "⚠️  D7 INT Debt: PASS-WITH-DRIFT ($DEFERRED_WITH_PLAN deferred with plan)"
    echo "   These items have documented v3.9.0+ plans but are not yet implemented."
    exit 2
fi

echo "✅ D7 INT Debt: PASS (0 ACTIVE, 0 DEFERRED, all CLOSED)"
exit 0
