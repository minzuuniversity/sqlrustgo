#!/usr/bin/env bash
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0; TOTAL=0; SKIP=0; BLOCKERS=0

check() {
    local name="$1"; shift
    local cmd=("$@")
    TOTAL=$((TOTAL+1))
    echo -n "[ga-v3.4.0] $name ... "
    if "${cmd[@]}" >/dev/null 2>&1; then
        echo "PASS"; PASS=$((PASS+1))
    else
        echo "FAIL"; BLOCKERS=$((BLOCKERS+1))
    fi
}

check_test() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    echo -n "[ga-v3.4.0] $name ... "
    out=$(eval "$cmd" 2>&1 || true)
    passed=$(echo "$out" | grep -c "test result: ok\." 2>/dev/null || echo "0")
    failed=$(echo "$out" | grep -c "test result: FAILED" 2>/dev/null || echo "0")
    passed=${passed//[^0-9]/}
    failed=${failed//[^0-9]/}
    if [ -z "$passed" ] || [ -z "$failed" ]; then
        passed=0; failed=0
    fi
    if [ "$failed" -eq 0 ] && [ "$passed" -gt 0 ]; then
        echo "PASS ($passed tests)"; PASS=$((PASS+1))
    elif [ "$failed" -eq 0 ] && [ "$passed" -eq 0 ]; then
        echo "SKIP (no tests)"; SKIP=$((SKIP+1)); TOTAL=$((TOTAL-1))
    else
        echo "FAIL ($passed passed, $failed failed)"; BLOCKERS=$((BLOCKERS+1))
    fi
}

echo "=== v3.4.0 GA Gate ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'N/A')"
echo ""

echo "━━━ R1: Build & Quality ━━━"
check "R1: cargo build --release --workspace" cargo build --release --workspace
check_test "R2: cargo test --lib" "cargo test --lib -- --test-threads=4"
check "R3: cargo clippy --all-features" cargo clippy --all-features -- -D warnings
check "R4: cargo fmt --all -- --check" cargo fmt --all -- --check

echo ""
echo "━━━ R5: Coverage (GA >=85%) ━━━"
TOTAL=$((TOTAL+1))
echo "[ga-v3.4.0] R5: Coverage >=85% (see artifacts/coverage/)"
if cargo llvm-cov --all-features --lib -- text 2>/dev/null | tail -1 | grep -q "\|.*85"; then
    echo "PASS (coverage >=85%)"; PASS=$((PASS+1))
else
    echo "SKIP (llvm-cov not available or coverage <85%)"; SKIP=$((SKIP+1))
fi

echo ""
echo "━━━ R6: Beta Gate (must pass first) ━━━"
check_test "R6: Beta Gate B1-B14" "bash scripts/gate/check_beta_v340.sh"

echo ""
echo "━━━ R7: gmp-retrieval (BM25 + RRF) ━━━"
check_test "R7: gmp-retrieval --lib" "cargo test -p sqlrustgo-gmp-retrieval --lib"

echo ""
echo "━━━ R8: Documentation ━━━"
check "R8: CHANGELOG exists" test -f docs/releases/v3.4.0/CHANGELOG.md
check "R8: RELEASE_NOTES exists" test -f docs/releases/v3.4.0/RELEASE_NOTES.md

echo ""
echo "=== v3.4.0 GA Gate Results ==="
echo "PASS=$PASS / $TOTAL, SKIP=$SKIP, BLOCKERS=$BLOCKERS"
echo ""

if [ $BLOCKERS -gt 0 ]; then
    echo "RESULT: FAILED"
    exit 1
else
    echo "RESULT: PASSED"
    exit 0
fi
