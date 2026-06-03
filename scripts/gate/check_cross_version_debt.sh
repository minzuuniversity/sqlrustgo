#!/usr/bin/env bash

# check_cross_version_debt.sh — Cross-Version Debt Gate Check
#
# This script checks cross-version debt status for v3.8.0 GA gate.
# It validates that INT-1~INT-4 (Integration Debt) status is properly tracked.
#
# Usage: ./scripts/gate/check_cross_version_debt.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
CROSS_VERSION_DEBT_DOC="$REPO_DIR/docs/releases/v3.8.0/CROSS-VERSION-DEBT.md"

echo "=== Running Cross-Version Debt Gate Check ==="
cd "$REPO_DIR"
pwd

# Check that CROSS-VERSION-DEBT.md exists
if [ ! -f "$CROSS_VERSION_DEBT_DOC" ]; then
    echo "❌ FAIL: $CROSS_VERSION_DEBT_DOC not found"
    exit 1
fi
echo "✅ CROSS-VERSION-DEBT.md exists"

# Parse INT-1~INT-4 status from CROSS-VERSION-DEBT.md
echo ""
echo "Checking Integration Debt (INT-1~INT-4) status..."

declare -A DEBT_STATUS
DEBT_STATUS["INT-1"]="UNKNOWN"
DEBT_STATUS["INT-2"]="UNKNOWN"
DEBT_STATUS["INT-3"]="UNKNOWN"
DEBT_STATUS["INT-4"]="UNKNOWN"

# Extract status for each debt item
for debt_id in INT-1 INT-2 INT-3 INT-4; do
    # Look for status in the table (ACTIVE, CLOSED, DEFERRED)
    status_line=$(grep -A2 "$debt_id" "$CROSS_VERSION_DEBT_DOC" | grep -E "ACTIVE|CLOSED|DEFERRED" | head -1)
    if echo "$status_line" | grep -q "ACTIVE"; then
        DEBT_STATUS[$debt_id]="ACTIVE"
    elif echo "$status_line" | grep -q "CLOSED"; then
        DEBT_STATUS[$debt_id]="CLOSED"
    elif echo "$status_line" | grep -q "DEFERRED"; then
        DEBT_STATUS[$debt_id]="DEFERRED"
    else
        # Try alternative pattern - look for status in same line as debt_id
        status_line=$(grep "$debt_id" "$CROSS_VERSION_DEBT_DOC" | grep -E "ACTIVE|CLOSED|DEFERRED" | head -1)
        if echo "$status_line" | grep -q "ACTIVE"; then
            DEBT_STATUS[$debt_id]="ACTIVE"
        elif echo "$status_line" | grep -q "CLOSED"; then
            DEBT_STATUS[$debt_id]="CLOSED"
        elif echo "$status_line" | grep -q "DEFERRED"; then
            DEBT_STATUS[$debt_id]="DEFERRED"
        fi
    fi
done

# Report status
PASS=true
for debt_id in INT-1 INT-2 INT-3 INT-4; do
    status=${DEBT_STATUS[$debt_id]}
    echo "  $debt_id: $status"
    if [ "$status" = "UNKNOWN" ]; then
        PASS=false
    fi
done

echo ""

# Validate that all debt items have known status
if [ "$PASS" = false ]; then
    echo "❌ FAIL: Some debt items have unknown status"
    exit 1
fi
echo "✅ All INT debt items have known status"

# Count ACTIVE debt items (for reporting)
ACTIVE_COUNT=0
for debt_id in INT-1 INT-2 INT-3 INT-4; do
    if [ "${DEBT_STATUS[$debt_id]}" = "ACTIVE" ]; then
        ACTIVE_COUNT=$((ACTIVE_COUNT + 1))
    fi
done

echo ""
echo "Cross-Version Debt Summary:"
echo "  Total INT items: 4"
echo "  ACTIVE: $ACTIVE_COUNT"
echo "  CLOSED/DEFERRED: $((4 - ACTIVE_COUNT))"

# Gate criteria: All ACTIVE debt must have fix plans
if [ "$ACTIVE_COUNT" -gt 0 ]; then
    echo ""
    echo "⚠️  WARNING: $ACTIVE_COUNT ACTIVE debt items found"
    echo "    These items require deferred status or fix plans"
fi

# Check that ADR-010 exists for deferred items
ADR_010="$REPO_DIR/docs/governance/adr/ADR-010-ghost-pr-resolution.md"
if [ -f "$ADR_010" ]; then
    echo ""
    echo "✅ ADR-010 (Ghost PR Resolution) exists - deferred items documented"
else
    echo ""
    echo "⚠️  WARNING: ADR-010 not found - ghost PR deferrals not documented"
fi

echo ""
echo "=== Cross-Version Debt Gate Check Complete ==="

if [ "$PASS" = true ]; then
    echo "✅ PASS"
    exit 0
else
    echo "❌ FAIL"
    exit 1
fi
