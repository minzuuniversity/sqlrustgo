#!/bin/bash
# V312-36: SQL Corpus 80% pass rate gate (G18)
# Threshold: pass_rate >= 80.0%
# Exit 0 = PASS, Exit 1 = FAIL
set -e

REPORT=$(mktemp -t corpus-gate.XXXXXX)
trap 'rm -f "$REPORT"' EXIT

echo "Running corpus_test::test_sql_corpus_all ..."
# Run release-mode corpus test; allow cargo to fail (the panic below threshold is the failure signal we capture)
cargo test --release -p sqlrustgo-sql-corpus --test corpus_test \
    test_sql_corpus_all -- --nocapture > "$REPORT" 2>&1 || true

# Extract pass rate from "Final Summary: ... XX.X% pass rate"
PASS_RATE=$(grep -oE '[0-9]+\.[0-9]+% pass rate' "$REPORT" | head -1 | grep -oE '[0-9]+\.[0-9]+')

if [ -z "$PASS_RATE" ]; then
    echo "FAIL: could not extract pass rate from corpus output" >&2
    echo "----- corpus test output (tail 30) -----" >&2
    tail -30 "$REPORT" >&2
    exit 1
fi

echo "SQL Corpus pass rate: ${PASS_RATE}% (threshold: 80.0%)"

if (( $(echo "$PASS_RATE < 80.0" | bc -l) )); then
    echo "FAIL: pass rate ${PASS_RATE}% < 80.0% threshold" >&2
    exit 1
fi

exit 0
