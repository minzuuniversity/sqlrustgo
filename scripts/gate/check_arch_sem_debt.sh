#!/usr/bin/env bash
# =============================================================================
# check_arch_sem_debt.sh — v3.8.0 D8-Architecture-Semantic-Debt Gate
# =============================================================================
# Implements 5-Principle P5 enforcement for ARCH-1~3 + SEM-1~4:
#   "未通过的必须有记录和后续改进"
#
# v3.8.0+ UPGRADE: Reads status from `debt-registry.yaml` SSOT
# (was hard-coded "OPEN" for all 7 items in v3.7.0 — see PR-3097 §2.6
# audit finding #3105: 3 项 ARCH/SEM 状态 STALE).
#
# Per ADR-011 (7-state machine), the 7 states are:
#   OPEN / IN_PROGRESS / BLOCKED / VERIFIED / CLOSED / SUPERSEDED / REJECTED
#
# Exit codes:
#   0  = all CLOSED
#   1  = any OPEN w/o plan (blocker)
#   2  = some IN_PROGRESS / BLOCKED w/ plan (DRIFT, acceptable for v3.8.0)
#   3  = any other unknown state (UNHANDLED)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# SSOT (Single Source of Truth) — per ADR-011 + PR-3149
REGISTRY="$REPO_ROOT/docs/governance/debt/debt-registry.yaml"

# Fallback plan doc for items not yet in registry
PLAN_DOC="$REPO_ROOT/docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md"
if [ ! -f "$PLAN_DOC" ]; then
    PLAN_DOC="$REPO_ROOT/docs/releases/v3.8.0/ARCH_SEM_DEBT_REMEDIATION_PLAN.md"
fi

echo "=== D8: Architecture/Semantic Debt Gate (5-Principle P5) ==="
echo "  SSOT: $REGISTRY"
echo

# Validate registry
if [ ! -f "$REGISTRY" ]; then
    echo "❌ $REGISTRY not found"
    exit 1
fi

# Read all 7 ARCH + SEM items from registry (SSOT).
# Output: one line per item: "<id>\t<state>\t<progress>"
ITEMS_DATA=$(python3 -c "
import yaml, sys
d = yaml.safe_load(open('$REGISTRY'))
items = []
for category in ('arch', 'sem'):
    for entry in d.get(category, []):
        items.append((entry['id'], entry.get('state', 'UNKNOWN'), entry.get('progress', '')))
for i, s, p in items:
    print(f'{i}\t{s}\t{p}')
")

CLOSED_COUNT=0
IN_PROGRESS_COUNT=0
OPEN_W_PLAN=0
OPEN_WO_PLAN=0
UNKNOWN_COUNT=0
FAILED_ITEMS=()

while IFS=$'\t' read -r debt_id status progress; do
    [ -z "$debt_id" ] && continue
    echo -n "  [$debt_id] state=$status"
    [ -n "$progress" ] && echo -n " ($progress)" || true

    case "$status" in
        CLOSED)
            echo " ✅"
            CLOSED_COUNT=$((CLOSED_COUNT + 1))
            ;;
        IN_PROGRESS|BLOCKED|VERIFIED|SUPERSEDED|REJECTED)
            # Per ADR-011: in-progress / blocked with target_release is acceptable (DRIFT).
            # require target_release field (parsed from registry).
            has_plan=$(REG="$REGISTRY" ID="$debt_id" python3 -c "
import yaml, os
d = yaml.safe_load(open(os.environ['REG']))
for cat in ('arch', 'sem'):
    for e in d.get(cat, []):
        if e['id'] == os.environ['ID']:
            r = 'true' if e.get('target_release') else 'false'
            print(r)
            break
    else:
        continue
    break
else:
    print('false')")
            if [ "$has_plan" = "true" ]; then
                echo " (has v3.9.0+ plan) ⚠️"
                IN_PROGRESS_COUNT=$((IN_PROGRESS_COUNT + 1))
                OPEN_W_PLAN=$((OPEN_W_PLAN + 1))
            else
                echo " ❌ (no v3.9.0+ plan)"
                OPEN_WO_PLAN=$((OPEN_WO_PLAN + 1))
                FAILED_ITEMS+=("$debt_id ($status without target_release)")
            fi
            ;;
        OPEN)
            OPEN_WO_PLAN=$((OPEN_WO_PLAN + 1))
            echo " (no plan) ❌"
            FAILED_ITEMS+=("$debt_id (OPEN without plan)")
            ;;
        *)
            UNKNOWN_COUNT=$((UNKNOWN_COUNT + 1))
            echo " ❌ (unknown state)"
            FAILED_ITEMS+=("$debt_id (unknown state: $status)")
            ;;
    esac
done <<< "$ITEMS_DATA"

echo
echo "=== D8 Arch/Sem Debt Summary ==="
echo "  CLOSED:               $CLOSED_COUNT"
echo "  IN_PROGRESS/BLOCKED:  $IN_PROGRESS_COUNT (with plan)"
echo "  OPEN w/o plan:        $OPEN_WO_PLAN"
echo "  UNKNOWN:              $UNKNOWN_COUNT"
echo

# Decision logic
if [ $OPEN_WO_PLAN -gt 0 ] || [ $UNKNOWN_COUNT -gt 0 ]; then
    echo "=== Failed Items ==="
    for item in "${FAILED_ITEMS[@]}"; do
        echo "  - $item"
    done
    echo
    echo "❌ D8 Arch/Sem Debt: FAILED"
    echo "   5-Principle P5 violated: OPEN debt must be either:"
    echo "     1. Closed (resolved and validated)"
    echo "     2. IN_PROGRESS/BLOCKED with target_release in $REGISTRY"
    exit 1
fi

if [ $IN_PROGRESS_COUNT -gt 0 ]; then
    echo "⚠️  D8 Arch/Sem Debt: PASS-WITH-DRIFT ($IN_PROGRESS_COUNT in-progress / blocked with plan)"
    echo "   These items have documented v3.9.0+ plans in $REGISTRY"
    echo "   but are not yet fully implemented."
    exit 2
fi

echo "✅ D8 Arch/Sem Debt: PASS (all $CLOSED_COUNT items CLOSED)"
exit 0
