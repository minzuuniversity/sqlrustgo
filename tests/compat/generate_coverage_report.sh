#!/usr/bin/env bash
set -uo pipefail

COVERAGE_FILE="$(dirname "$0")/behavioral_coverage.txt"

if [ ! -f "$COVERAGE_FILE" ]; then
    echo "ERROR: Coverage file not found at $COVERAGE_FILE"
    exit 2
fi

DATA=$(tail -n +2 "$COVERAGE_FILE" | grep -v "^$" | grep -v "^feature_name:")
TOTAL=$(echo "$DATA" | wc -l | xargs)
COVERED=$(echo "$DATA" | grep "COVERED" | wc -l | xargs)
PARTIAL=$(echo "$DATA" | grep "PARTIAL" | wc -l | xargs)
NOT_IMPL=$(echo "$DATA" | grep "NOT_IMPLEMENTED" | wc -l | xargs)
NEEDS_VER=$(echo "$DATA" | grep "NEEDS_VERIFICATION" | wc -l | xargs)

if [ "$TOTAL" -gt 0 ]; then
    COVERAGE_PCT=$((COVERED * 100 / TOTAL))
else
    COVERAGE_PCT=0
fi

echo "=== Behavioral Coverage Report ==="
echo "Total features: $TOTAL"
echo "Covered: $COVERED ($COVERAGE_PCT%)"
echo "Partial: $PARTIAL"
echo "Not implemented: $NOT_IMPL"
echo "Needs verification: $NEEDS_VER"
echo ""
echo "=== Not Implemented ==="
echo "$DATA" | grep "NOT_IMPLEMENTED" | cut -d: -f1
echo ""
echo "=== Needs Verification (first 20) ==="
echo "$DATA" | grep "NEEDS_VERIFICATION" | cut -d: -f1 | head -20
echo "..."
