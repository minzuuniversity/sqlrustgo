#!/usr/bin/env bash

# check_cross_version_debt.sh — Cross-Version Debt Gate Check (EXTENDED)
#
# This script checks cross-version debt status for v3.8.0 GA gate.
# Validates:
#   - INT-1~INT-4 (Integration Debt, from CROSS-VERSION-DEBT.md)
#   - F-01~F-36 (Feature Debt, from v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md)
#   - I-01~I-12 (Integration Debt v3.0.0 era, from same source)
#   - T-01~T-20 (Test Debt, from same source)
# Total: 4 + 36 + 12 + 20 = 72 cross-version debt items tracked
#
# Source: docs/releases/v3.8.0/INT5_PLUS_DEBT_INVENTORY.md
#
# Usage: ./scripts/gate/check_cross_version_debt.sh
#        ./scripts/gate/check_cross_version_debt.sh --strict (fail on any ACTIVE/OPEN)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
CROSS_VERSION_DEBT_DOC="$REPO_DIR/docs/releases/v3.8.0/CROSS-VERSION-DEBT.md"
INT5_INVENTORY_DOC="$REPO_DIR/docs/releases/v3.8.0/INT5_PLUS_DEBT_INVENTORY.md"

STRICT_MODE=false
if [ "${1:-}" = "--strict" ]; then
    STRICT_MODE=true
fi

echo "=== Running Cross-Version Debt Gate Check (EXTENDED) ==="
cd "$REPO_DIR"
pwd

# Check that required docs exist
MISSING_DOCS=()
if [ ! -f "$CROSS_VERSION_DEBT_DOC" ]; then
    MISSING_DOCS+=("$CROSS_VERSION_DEBT_DOC")
fi
if [ ! -f "$INT5_INVENTORY_DOC" ]; then
    MISSING_DOCS+=("$INT5_INVENTORY_DOC")
fi
if [ ${#MISSING_DOCS[@]} -gt 0 ]; then
    echo "❌ FAIL: Required docs missing:"
    for d in "${MISSING_DOCS[@]}"; do
        echo "    - $d"
    done
    exit 1
fi
echo "✅ Both CROSS-VERSION-DEBT.md and INT5_PLUS_DEBT_INVENTORY.md exist"

declare -A DEBT_STATUS
TOTAL=0
PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

# ============================================================
# Part 1: INT-1~INT-4 (from CROSS-VERSION-DEBT.md)
# ============================================================
echo ""
echo "=== Part 1: Integration Debt (INT-1~INT-4) ==="

for debt_id in INT-1 INT-2 INT-3 INT-4; do
    TOTAL=$((TOTAL + 1))
    status_line=$(grep -A2 "$debt_id" "$CROSS_VERSION_DEBT_DOC" | grep -E "ACTIVE|CLOSED|DEFERRED" | head -1)
    if echo "$status_line" | grep -q "ACTIVE"; then
        DEBT_STATUS[$debt_id]="ACTIVE"
    elif echo "$status_line" | grep -q "CLOSED"; then
        DEBT_STATUS[$debt_id]="CLOSED"
    elif echo "$status_line" | grep -q "DEFERRED"; then
        DEBT_STATUS[$debt_id]="DEFERRED"
    else
        status_line=$(grep "$debt_id" "$CROSS_VERSION_DEBT_DOC" | grep -E "ACTIVE|CLOSED|DEFERRED" | head -1)
        if echo "$status_line" | grep -q "ACTIVE"; then
            DEBT_STATUS[$debt_id]="ACTIVE"
        elif echo "$status_line" | grep -q "CLOSED"; then
            DEBT_STATUS[$debt_id]="CLOSED"
        elif echo "$status_line" | grep -q "DEFERRED"; then
            DEBT_STATUS[$debt_id]="DEFERRED"
        else
            DEBT_STATUS[$debt_id]="UNKNOWN"
        fi
    fi
    status=${DEBT_STATUS[$debt_id]}
    echo "  $debt_id: $status"
    if [ "$status" = "UNKNOWN" ]; then
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
done

# ============================================================
# Part 2: F-01~F-36 (Feature Debt, from INT5_PLUS_DEBT_INVENTORY.md)
# ============================================================
echo ""
echo "=== Part 2: Feature Debt (F-01~F-36) ==="

# Check inventory table for status
for f_id in $(seq -f "F-%02g" 1 36); do
    TOTAL=$((TOTAL + 1))
    # Look for the F-xx row and check status column
    status_line=$(grep -E "^\| (${f_id}) \||^${f_id} \||\| ${f_id} \|" "$INT5_INVENTORY_DOC" | head -1)
    # Extract status indicator (✅ = closed, ⚠️ = partial, ❌ = open)
    if echo "$status_line" | grep -q "✅"; then
        DEBT_STATUS[$f_id]="CLOSED"
    elif echo "$status_line" | grep -q "⚠️"; then
        DEBT_STATUS[$f_id]="PARTIAL"
    elif echo "$status_line" | grep -q "❌"; then
        DEBT_STATUS[$f_id]="OPEN"
    elif echo "$status_line" | grep -q "DEFERRED"; then
        DEBT_STATUS[$f_id]="DEFERRED"
    else
        DEBT_STATUS[$f_id]="UNKNOWN"
    fi
    status=${DEBT_STATUS[$f_id]}
    if [ "$status" = "UNKNOWN" ]; then
        echo "  ⚠️  $f_id: $status (could not extract from inventory)"
        WARN_COUNT=$((WARN_COUNT + 1))
    else
        echo "  $f_id: $status"
    fi
done

# ============================================================
# Part 3: I-01~I-12 (Integration v3.0.0 era)
# ============================================================
echo ""
echo "=== Part 3: Integration Debt v3.0.0 (I-01~I-12) ==="

for i in $(seq -f "I-%02g" 1 12); do
    TOTAL=$((TOTAL + 1))
    status_line=$(grep -E "^\| (${i}) \||^${i} \||\| ${i} \|" "$INT5_INVENTORY_DOC" | head -1)
    if echo "$status_line" | grep -q "✅"; then
        DEBT_STATUS[$i]="CLOSED"
    elif echo "$status_line" | grep -q "⚠️"; then
        DEBT_STATUS[$i]="PARTIAL"
    elif echo "$status_line" | grep -q "❌"; then
        DEBT_STATUS[$i]="OPEN"
    else
        DEBT_STATUS[$i]="UNKNOWN"
    fi
    status=${DEBT_STATUS[$i]}
    if [ "$status" = "UNKNOWN" ]; then
        echo "  ⚠️  $i: $status"
        WARN_COUNT=$((WARN_COUNT + 1))
    else
        echo "  $i: $status"
    fi
done

# ============================================================
# Part 4: T-01~T-20 (Test Debt)
# ============================================================
echo ""
echo "=== Part 4: Test Debt (T-01~T-20) ==="

for t in $(seq -f "T-%02g" 1 20); do
    TOTAL=$((TOTAL + 1))
    status_line=$(grep -E "^\| (${t}) \||^${t} \||\| ${t} \|" "$INT5_INVENTORY_DOC" | head -1)
    if echo "$status_line" | grep -q "✅"; then
        DEBT_STATUS[$t]="CLOSED"
    elif echo "$status_line" | grep -q "⚠️"; then
        DEBT_STATUS[$t]="PARTIAL"
    elif echo "$status_line" | grep -q "❌"; then
        DEBT_STATUS[$t]="OPEN"
    else
        DEBT_STATUS[$t]="UNKNOWN"
    fi
    status=${DEBT_STATUS[$t]}
    if [ "$status" = "UNKNOWN" ]; then
        echo "  ⚠️  $t: $status"
        WARN_COUNT=$((WARN_COUNT + 1))
    else
        echo "  $t: $status"
    fi
done

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== Cross-Version Debt Summary ==="

ACTIVE_COUNT=0
CLOSED_COUNT=0
DEFERRED_COUNT=0
PARTIAL_COUNT=0
OPEN_COUNT=0
UNKNOWN_COUNT=0

for id in "${!DEBT_STATUS[@]}"; do
    status=${DEBT_STATUS[$id]}
    case "$status" in
        ACTIVE)   ACTIVE_COUNT=$((ACTIVE_COUNT + 1)) ;;
        CLOSED)   CLOSED_COUNT=$((CLOSED_COUNT + 1)) ;;
        DEFERRED) DEFERRED_COUNT=$((DEFERRED_COUNT + 1)) ;;
        PARTIAL)  PARTIAL_COUNT=$((PARTIAL_COUNT + 1)) ;;
        OPEN)     OPEN_COUNT=$((OPEN_COUNT + 1)) ;;
        UNKNOWN)  UNKNOWN_COUNT=$((UNKNOWN_COUNT + 1)) ;;
    esac
done

echo "  Total debt items tracked: $TOTAL"
echo "  ✅ CLOSED:    $CLOSED_COUNT"
echo "  ⚠️  PARTIAL:   $PARTIAL_COUNT"
echo "  ❌ OPEN:      $OPEN_COUNT"
echo "  ⏸️  DEFERRED:  $DEFERRED_COUNT"
echo "  🔄 ACTIVE:    $ACTIVE_COUNT"
echo "  ❓ UNKNOWN:   $UNKNOWN_COUNT"

# ============================================================
# Gate criteria
# ============================================================
echo ""
echo "=== Gate Criteria Check ==="

# CRITICAL: UNKNOWN items are FAILS
if [ "$UNKNOWN_COUNT" -gt 0 ]; then
    echo "❌ FAIL: $UNKNOWN_COUNT debt items have UNKNOWN status"
    echo "    These need to be tracked in INT5_PLUS_DEBT_INVENTORY.md"
    exit 1
fi

# In strict mode, ACTIVE/OPEN fail
if [ "$STRICT_MODE" = true ]; then
    if [ "$ACTIVE_COUNT" -gt 0 ] || [ "$OPEN_COUNT" -gt 0 ]; then
        echo "❌ STRICT FAIL: $ACTIVE_COUNT ACTIVE + $OPEN_COUNT OPEN items found"
        echo "    --strict mode requires 100% CLOSED or DEFERRED"
        exit 1
    fi
fi

# Default mode: WARN on ACTIVE/OPEN, FAIL on UNKNOWN
if [ "$ACTIVE_COUNT" -gt 0 ]; then
    echo "⚠️  WARNING: $ACTIVE_COUNT ACTIVE debt items found"
    echo "    These should be moved to CLOSED or DEFERRED"
fi

if [ "$OPEN_COUNT" -gt 0 ]; then
    echo "⚠️  WARNING: $OPEN_COUNT OPEN debt items found"
    echo "    See INT5_PLUS_DEBT_INVENTORY.md for fix plans"
fi

# ADR-010 check
ADR_010="$REPO_DIR/docs/governance/adr/ADR-010-ghost-pr-resolution.md"
if [ -f "$ADR_010" ]; then
    echo "✅ ADR-010 (Ghost PR Resolution) exists - deferred items documented"
else
    echo "⚠️  WARNING: ADR-010 not found - ghost PR deferrals not documented"
fi

echo ""
echo "=== Cross-Version Debt Gate Check Complete ==="

if [ "$UNKNOWN_COUNT" -gt 0 ]; then
    echo "❌ FAIL (UNKNOWN items)"
    exit 1
elif [ "$STRICT_MODE" = true ] && { [ "$ACTIVE_COUNT" -gt 0 ] || [ "$OPEN_COUNT" -gt 0 ]; }; then
    echo "❌ STRICT FAIL"
    exit 1
else
    echo "✅ PASS"
    exit 0
fi
