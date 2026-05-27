#!/usr/bin/env bash
# check_beta_v350.sh — v3.5.0 Beta Gate 检查脚本
# 规范来源: docs/governance/gate_spec_v350.md §四
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0; TOTAL=0; SKIP=0; BLOCKERS=0

check() {
    local name="$1"; shift
    local cmd=("$@")
    TOTAL=$((TOTAL+1))
    echo -n "[beta-v3.5.0] $name ... "
    if "${cmd[@]}" >/dev/null 2>&1; then
        echo "PASS"; PASS=$((PASS+1))
    else
        echo "FAIL"; BLOCKERS=$((BLOCKERS+1))
    fi
}

check_test() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    echo -n "[beta-v3.5.0] $name ... "
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

echo "=== v3.5.0 Beta Gate ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'N/A')"
echo ""

# === 2.1 代码质量 ===
echo "━━━ 2.1 代码质量 ━━━"
check "B1: cargo build --release --workspace" cargo build --release --workspace
check_test "B2: cargo test --lib" "cargo test --lib -- --test-threads=4"
check "B3: cargo clippy --all-features" cargo clippy --all-features -- -D warnings
check "B4: cargo fmt --all -- --check" cargo fmt --all -- --check

# === 2.2 v3.5.0 新增 crate 测试 ===
echo ""
echo "━━━ 2.2 v3.5.0 新增 Crate 测试 ━━━"
check_test "B-TI4: compliance-engine --lib" "cargo test -p sqlrustgo-compliance-engine --lib"
check_test "B-TI7: gmp-api --lib" "cargo test -p sqlrustgo-gmp-api --lib"
check_test "B-TI8: gmp --lib" "cargo test -p sqlrustgo-gmp --lib"
check_test "B-TI9: gmp-retrieval --lib" "cargo test -p sqlrustgo-gmp-retrieval --lib"

# === 2.3 Trust Infrastructure (existing) ===
echo ""
echo "━━━ 2.3 Trust Infrastructure ━━━"
check_test "B-TI1: evidence-engine --lib" "cargo test -p sqlrustgo-evidence-engine --lib"
check_test "B-TI2: provenance-graph --lib" "cargo test -p sqlrustgo-provenance-graph --lib"
check_test "B-TI3: workflow-v2 --lib" "cargo test -p sqlrustgo-workflow-v2 --lib"
check_test "B-TI5: perf-baseline --lib" "cargo test -p sqlrustgo-perf-baseline --lib"
check_test "B-TI6: crash-sim --lib" "cargo test -p sqlrustgo-crash-sim --lib"

echo ""
echo "=== v3.5.0 Beta Gate Results ==="
echo "PASS=$PASS / $TOTAL, SKIP=$SKIP, BLOCKERS=$BLOCKERS"
echo ""

if [ $BLOCKERS -gt 0 ]; then
    echo "RESULT: FAILED"
    exit 1
else
    echo "RESULT: PASSED"
    exit 0
fi
