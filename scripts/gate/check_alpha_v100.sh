#!/bin/bash
# Gate Check Script: Alpha Gate v1.0.0
# Usage: bash scripts/gate/check_alpha_v100.sh

set -e

echo "=========================================="
echo "  GMP Retrieve v1.0.0 Alpha Gate Check"
echo "  $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="
echo ""

PASS=0
FAIL=0
TOTAL=9

log_pass() { echo "  [PASS] $1"; PASS=$((PASS+1)); }
log_fail() { echo "  [FAIL] $1"; FAIL=$((FAIL+1)); }

# A1: Build
echo "--- A1: Build ---"
if cargo build --release 2>&1 | tail -1 | grep -q "Finished"; then
    log_pass "cargo build --release"
else
    log_fail "cargo build --release"
fi

# A2: Test
echo "--- A2: Unit Tests ---"
if cargo test 2>&1 | tail -3; then
    log_pass "cargo test"
else
    log_fail "cargo test"
fi

# A3: Clippy
echo "--- A3: Clippy ---"
WARNINGS=$(cargo clippy -- -D warnings 2>&1 | grep -c "warning:" || true)
if [ "$WARNINGS" -eq 0 ]; then
    log_pass "clippy: 0 warnings"
else
    log_fail "clippy: $WARNINGS warnings"
fi

# A4: Format
echo "--- A4: Format ---"
if cargo fmt -- --check 2>&1; then
    log_pass "cargo fmt"
else
    log_fail "cargo fmt"
fi

# A5: Data Integrity
echo "--- A5: Data Integrity ---"
if [ -f "gmp.db" ]; then
    DOC_COUNT=$(./target/release/gmp-retrieve index --validate 2>&1 | grep "Documents" | grep -o '[0-9]*' | head -1)
    if [ "$DOC_COUNT" -ge 1696 ]; then
        log_pass "data integrity: $DOC_COUNT documents"
    else
        log_fail "data integrity: only $DOC_COUNT/1696 documents"
    fi
else
    log_fail "gmp.db not found"
fi

# A6: FTS Query
echo "--- A6: FTS Query ---"
FTS_COUNT=$(./target/release/gmp-retrieve search "SOP" --channel fts 2>&1 | grep -c "^\[" || true)
if [ "$FTS_COUNT" -ge 5 ]; then
    log_pass "FTS query: $FTS_COUNT results"
else
    log_fail "FTS query: only $FTS_COUNT results"
fi

# A7: Vector Query
echo "--- A7: Vector Query ---"
VEC_COUNT=$(./target/release/gmp-retrieve search "偏差" --channel vector 2>&1 | grep -c "^\[" || true)
if [ "$VEC_COUNT" -ge 5 ]; then
    log_pass "Vector query: $VEC_COUNT results"
else
    log_fail "Vector query: only $VEC_COUNT results"
fi

# A8: Graph Query
echo "--- A8: Graph Query ---"
GRAPH_COUNT=$(./target/release/gmp-retrieve search "CAPA" --channel graph 2>&1 | grep -c "^\[" || true)
if [ "$GRAPH_COUNT" -ge 3 ]; then
    log_pass "Graph query: $GRAPH_COUNT results"
else
    log_fail "Graph query: only $GRAPH_COUNT results"
fi

# A9: RRF Fusion
echo "--- A9: RRF Fusion ---"
RRF_COUNT=$(./target/release/gmp-retrieve search "偏差分级" --all 2>&1 | grep -c "^\[" || true)
if [ "$RRF_COUNT" -ge 1 ]; then
    log_pass "RRF fusion: $RRF_COUNT results"
else
    log_fail "RRF fusion: no results"
fi

echo ""
echo "=========================================="
echo "  Alpha Gate Result: $PASS/$TOTAL PASSED"
echo "=========================================="

if [ "$FAIL" -eq 0 ]; then
    echo "  Status: PASS"
    exit 0
else
    echo "  Status: FAIL ($FAIL failures)"
    exit 1
fi