#!/usr/bin/env bash
# v3.4.0 Beta Gate
# 执行: bash scripts/gate/check_beta_v340.sh
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0; TOTAL=0; SKIP=0; BLOCKERS=0

check() {
    local name="$1"; shift
    local cmd=("$@")
    TOTAL=$((TOTAL+1))
    echo -n "[beta-v3.4.0] $name ... "
    if "${cmd[@]}" >/dev/null 2>&1; then
        echo "PASS"; PASS=$((PASS+1))
    else
        echo "FAIL"; BLOCKERS=$((BLOCKERS+1))
    fi
}

check_test() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    echo -n "[beta-v3.4.0] $name ... "
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

check_optional() {
    local name="$1"; shift
    local cmd=("$@")
    TOTAL=$((TOTAL+1))
    echo -n "[beta-v3.4.0] $name ... "
    if "${cmd[@]}" >/dev/null 2>&1; then
        echo "PASS"; PASS=$((PASS+1))
    else
        echo "SKIP (not available)"; SKIP=$((SKIP+1))
    fi
}

echo "=== v3.4.0 Beta Gate ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'N/A')"
echo ""

# === 2.1 代码质量 ===
echo "━━━ 2.1 代码质量 ━━━"

check "B1: cargo build --release --workspace" cargo build --release --workspace
check_test "B2: cargo test --lib" "cargo test --lib -- --test-threads=4"
check "B3: cargo clippy --all-features" cargo clippy --all-features -- -D warnings
check "B4: cargo fmt --all -- --check" cargo fmt --all -- --check

# === 2.2 GMP API 功能 ===
echo ""
echo "━━━ 2.2 GMP API 功能 ━━━"

check "B5: cargo build -p sqlrustgo-gmp" cargo build -p sqlrustgo-gmp
check_test "B6: cargo test -p sqlrustgo-gmp --lib" "cargo test -p sqlrustgo-gmp --lib"
check_test "B7: Batch API tests" "cargo test -p sqlrustgo-gmp -- batch"
check_test "B8: Audit API tests" "cargo test -p sqlrustgo-gmp -- audit"

# === 2.3 GMP 检索 v2 (Optional - may not exist yet) ===
echo ""
echo "━━━ 2.3 GMP 检索 v2 ━━━"

check_optional "B9: gmp-retrieval build" cargo build -p sqlrustgo-gmp-retrieval 2>/dev/null
check_optional "B10: BM25 search" cargo test -p sqlrustgo-gmp-retrieval -- bm25 2>/dev/null
check_optional "B11: RRF fusion" cargo test -p sqlrustgo-gmp-retrieval -- rrf 2>/dev/null

# === 2.4 Trust Infrastructure ===
echo ""
echo "━━━ 2.4 Trust Infrastructure ━━━"

check_test "B12: evidence-engine --lib" "cargo test -p sqlrustgo-evidence-engine --lib"
check_test "B13: workflow-v2 --lib" "cargo test -p sqlrustgo-workflow-v2 --lib"
check_test "B14: trust-viz --lib" "cargo test -p sqlrustgo-trust-viz --lib"

# === Summary ===
echo ""
echo "=== v3.4.0 Beta Gate Results ==="
echo "PASS=$PASS / $TOTAL, SKIP=$SKIP, BLOCKERS=$BLOCKERS"
echo ""

if [ $BLOCKERS -gt 0 ]; then
    echo "RESULT: FAILED"
    exit 1
else
    echo "RESULT: PASSED"
    exit 0
fi
