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
FAILED_CRATES=()

for crate in "${L1_8_CRATES[@]}"; do
    echo "Measuring coverage for $crate ..."
    # `--no-fail-fast` so one failing test doesn't abort the whole measurement.
    # Exit status is still checked below — a crate that will not build or whose
    # tests will not run is a measurement failure, not a zero-coverage crate.
    set +e
    OUTPUT=$(cargo llvm-cov test -p "$crate" --no-fail-fast 2>&1)
    RC=$?
    set -e
    # `cargo llvm-cov` prints a coverage table whose TOTAL row carries FOUR
    # percentages (regions / functions / lines / branches). The previous
    # `grep -oE '[0-9]+\.[0-9]+%' | tail -1` took the last one — the BRANCHES
    # figure — and labelled it line coverage.
    #
    # The header is `Filename Regions Missed Regions Cover Functions ...` and
    # the data row is `TOTAL 1278 76 94.05% ...`: the "TOTAL" label occupies
    # the Filename column, so data cells sit several columns left of their
    # header. Rather than hardcode that shift (it varies with the llvm-cov
    # version), locate the header's Lines-group Cover cell and walk back to
    # the nearest cell that actually parses as a percentage. If none does,
    # report no coverage rather than a wrong one.
    COVERAGE=$(echo "$OUTPUT" | python3 -c '
import re, sys

hdr = tot = None
for line in sys.stdin.read().splitlines():
    f = line.split()
    if not f:
        continue
    if hdr is None and f[0] == "Filename" and "Lines" in f:
        hdr = f
    elif f[0] == "TOTAL":
        tot = f

if hdr is None or tot is None:
    sys.exit(0)

try:
    # Lines group is [Lines, Missed, Lines, Cover]; its Cover is the first
    # "Cover" at or after the FIRST "Lines" token.
    cov_col = hdr.index("Cover", hdr.index("Lines"))
except ValueError:
    sys.exit(0)

# Walk back over the leading label shift, taking the rightmost cell that is
# actually a percentage. This tolerates a "TOTAL" prefix of any width and any
# number of columns the tool inserts, without ever returning a non-percentage.
for shift in range(0, 6):
    idx = cov_col - shift
    if 0 <= idx < len(tot):
        m = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)%", tot[idx])
        if m:
            print(m.group(1))
            break
')
    if [ $RC -ne 0 ] || [ -z "$COVERAGE" ]; then
        # #4944 P1-01: the previous shape did `continue` here, which dropped the
        # crate from both the numerator and the denominator. A crate that fails
        # to measure therefore *raises* the reported average — the gate could
        # pass with fewer crates measured than the L1_8 set requires. Record the
        # failure and fail the gate at the end instead of averaging over a
        # silently reduced sample.
        echo "  FAIL: could not measure $crate (exit=$RC${COVERAGE:+, got $COVERAGE%})"
        FAILED_CRATES+=("$crate")
        continue
    fi
    echo "  $crate: ${COVERAGE}%"
    TOTAL=$(echo "$TOTAL + $COVERAGE" | bc)
    COUNT=$((COUNT + 1))
done

if [ ${#FAILED_CRATES[@]} -gt 0 ]; then
    echo "FAIL: ${#FAILED_CRATES[@]}/${#L1_8_CRATES[@]} crate(s) produced no coverage data: ${FAILED_CRATES[*]}" >&2
    echo "      The L1_8 average is only meaningful over the full crate set —" >&2
    echo "      a reduced sample silently inflates it." >&2
    exit 1
fi

if [ "$COUNT" -ne "${#L1_8_CRATES[@]}" ]; then
    echo "FAIL: measured $COUNT of ${#L1_8_CRATES[@]} L1_8 crates" >&2
    exit 1
fi

if [ "$COUNT" -eq 0 ]; then
    # Unreachable: the FAILED_CRATES / count-mismatch checks above already exit
    # unless all 8 crates measured. Kept as a guard against a future edit that
    # reorders the checks, since a division by COUNT=0 would otherwise be
    # silently wrong rather than loud.
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
