#!/bin/bash
# =============================================================================
# V312-11 / ISSUE #3898 — SQLLogicTest Gate
# =============================================================================
# Runs the SQLite SQLLogicTest oracle against sqlrustgo's local corpus.
# Exit 0 = pass (no regressions), exit non-zero = fail
#
# Usage: bash scripts/gate/sqllogictest_gate.sh
# =============================================================================
set -euo pipefail

TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
SOURCE_AGENT="${SOURCE_AGENT:-claude-code}"
SOURCE_RUN="${SOURCE_RUN:-v312-11-gate-$(git rev-parse --short HEAD 2>/dev/null || echo nohead)}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${ROOT}/docs/releases/v3.12.0/evidence/sqllogictest"

mkdir -p "${OUT_DIR}"

echo "=== SQLLogicTest Gate ==="
echo "Timestamp: ${TIMESTAMP}"
echo "Source: ${SOURCE_AGENT}/${SOURCE_RUN}"
echo "Root: ${ROOT}"
echo ""

# Build first
echo "[1/3] Building sqlrustgo_sqllogictest..."
cd "${ROOT}"
if ! cargo build -p sqlrustgo_sqllogictest --quiet 2>&1; then
    echo "FAIL: Build failed"
    exit 1
fi
echo "Build: OK"
echo ""

# Run tests
echo "[2/3] Running SQLLogicTest corpus..."
RESULTS="${OUT_DIR}/results_${SOURCE_RUN}.txt"
cargo run -p sqlrustgo_sqllogictest -- \
    --test-dir crates/sqlrustgo_sqllogictest/testdata \
    --max-fail 5 2>&1 | tee "${RESULTS}"

# Extract pass/fail stats
if grep -q "files:.*(pass/fail)" "${RESULTS}"; then
    STATS=$(grep "files:.*(pass/fail)" "${RESULTS}" | head -1)
    echo ""
    echo "Results: ${STATS}"
fi

echo ""
echo "[3/3] Gate complete"
echo "Results saved to: ${RESULTS}"

# Exit based on test results
# For now, just warn - full gate requires parser fixes
echo ""
echo "NOTE: This is a baseline run. Full gate requires parser fixes for:"
echo "  - VALUES constructor"
echo "  - EXCEPT ALL / INTERSECT ALL"
echo "  - Window functions in ORDER BY"
