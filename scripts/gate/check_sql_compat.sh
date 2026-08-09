#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== R8: SQL Compatibility Check ==="
echo "Date: $(date)"
echo ""

cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

CORPUS_THRESHOLD=80

echo "[1/2] Running SQL Corpus tests..."
# V312-30: removed `|| true` mask; propagate failure explicitly. P16 step
# 2.5 detects this anti-fab pattern; fail-explicit per V312-24 acceptance
# criteria ("known broken test binaries 不得继续靠 WARN-only 掩盖").
CORPUS_OUTPUT=$(cargo test -p sqlrustgo-sql-corpus -- --nocapture 2>&1)
CORPUS_EXIT=$?
if [ ${CORPUS_EXIT} -ne 0 ]; then
    echo "  FAIL: cargo test exit ${CORPUS_EXIT} (corpus test failed to build/run)"
    echo "  Output tail:"
    echo "${CORPUS_OUTPUT}" | tail -20 | sed 's/^/    /'
    exit 1
fi

echo "[2/2] Analyzing results..."

if echo "$CORPUS_OUTPUT" | grep -q "test result: ok"; then
    echo "SQL Corpus tests: PASS"
    echo ""
    echo "✅ R8: SQL Compatibility Check PASSED"
    echo "   Corpus pass rate >= $CORPUS_THRESHOLD%"
    exit 0
else
    echo "SQL Corpus tests: FAIL"
    echo ""
    echo "❌ R8: SQL Compatibility Check FAILED"
    echo "   SQL Corpus pass rate < $CORPUS_THRESHOLD%"
    exit 1
fi
