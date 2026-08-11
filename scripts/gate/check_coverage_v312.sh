#!/bin/bash
# V312-35: per-crate coverage measurement (L1_8)
# Threshold: L1_8 average line coverage >= 80%
# Exit 0 = PASS, Exit 1 = FAIL
set -e

# L1_8 = 8 core crates per GATE_CONDITIONS.md
L1_8_CRATES=(
    "sqlrustgo-parser"
    "sqlrustgo-planner"
    "sqlrustgo-executor"
    "sqlrustgo-transaction"
    "sqlrustgo-storage"
    "sqlrustgo-catalog"
    "sqlrustgo-optimizer"
    "sqlrustgo-types"
)

THRESHOLD=80.0
TOTAL=0
COUNT=0

for crate in "${L1_8_CRATES[@]}"; do
    echo "Measuring coverage for $crate ..."
    # Run cargo llvm-cov with --no-fail-fast so a failing test doesn't block coverage
    OUTPUT=$(cargo llvm-cov test -p "$crate" --no-fail-fast 2>&1 || true)
    # Extract "X.YY%" pattern from "Coverage ... XX.YY%"
    COVERAGE=$(echo "$OUTPUT" | grep -oE '[0-9]+\.[0-9]+%' | tail -1 | tr -d '%')
    if [ -z "$COVERAGE" ]; then
        echo "  WARN: could not extract coverage for $crate, skipping"
        continue
    fi
    echo "  $crate: ${COVERAGE}%"
    TOTAL=$(echo "$TOTAL + $COVERAGE" | bc)
    COUNT=$((COUNT + 1))
done

if [ "$COUNT" -eq 0 ]; then
    echo "FAIL: no crates produced coverage data" >&2
    exit 1
fi

AVG=$(echo "scale=2; $TOTAL / $COUNT" | bc)
echo "L1_8 average coverage: ${AVG}% (threshold: ${THRESHOLD}%)"

if (( $(echo "$AVG < $THRESHOLD" | bc -l) )); then
    echo "FAIL: L1_8 avg ${AVG}% < ${THRESHOLD}%" >&2
    exit 1
fi

exit 0
