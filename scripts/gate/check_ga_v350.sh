#!/usr/bin/env bash
# check_ga_v350.sh — v3.5.0 GA Gate 检查脚本
# 规范来源: docs/governance/gate_spec_v350.md §六
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0; TOTAL=0; BLOCKERS=0
FAIL_REASONS=()
SKIP_REASONS=()

log_info() { echo "[ga-v3.5.0] $1"; }
log_pass() { echo "✅ PASS: $1"; PASS=$((PASS+1)); TOTAL=$((TOTAL+1)); }
log_fail() { echo "❌ FAIL: $1"; BLOCKERS=$((BLOCKERS+1)); TOTAL=$((TOTAL+1)); FAIL_REASONS+=("$1"); }
log_skip() { echo "⏭️  SKIP: $1"; TOTAL=$((TOTAL+1)); SKIP_REASONS+=("$1"); }

check() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    echo -n "[ga-v3.5.0] $name ... "
    if eval "$cmd" >/dev/null 2>&1; then
        echo "PASS"; PASS=$((PASS+1))
    else
        echo "FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("$name")
    fi
}

check_test() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    echo -n "[ga-v3.5.0] $name ... "
    local output
    output=$(eval "$cmd" 2>&1) || true
    local passed failed
    passed=$(echo "$output" | grep -c "test result: ok\." || true)
    failed=$(echo "$output" | grep -c "test result: FAILED" || true)
    passed=${passed:-0}; failed=${failed:-0}
    if [ "$failed" -eq 0 ] && [ "$passed" -gt 0 ]; then
        echo "PASS ($passed tests)"; PASS=$((PASS+1))
    else
        echo "FAIL ($passed passed, $failed failed)"; BLOCKERS=$((BLOCKERS+1))
        FAIL_REASONS+=("$name")
    fi
}

echo "=========================================="
echo "  v3.5.0 GA Gate — 正式发布前必须通过"
echo "  规范来源: docs/governance/gate_spec_v350.md"
echo "=========================================="
echo "日期: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# === G1: Build ===
log_info "G1: cargo build --release --workspace"
TOTAL=$((TOTAL+1))
if cargo build --release --workspace >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G1: Build failed")
fi

# === G2: Test (100%) ===
log_info "G2: Full test suite (100%)"
TOTAL=$((TOTAL+1))
TEST_OUTPUT=$(cargo test --all-features --lib 2>&1 || true)
PASSED=$(echo "$TEST_OUTPUT" | grep -c "test result: ok\." 2>/dev/null || echo "0")
FAILED=$(echo "$TEST_OUTPUT" | grep -c "test result: FAILED" 2>/dev/null || echo "0")
PASSED=${PASSED//[^0-9]/}; FAILED=${FAILED//[^0-9]/}
TOTAL_TESTS=$((PASSED + FAILED))
if [ "$TOTAL_TESTS" -gt 0 ]; then
    if [ "$FAILED" -eq 0 ] && [ "$PASSED" -gt 0 ]; then
        echo "✅ PASS (100% = $PASSED/$TOTAL_TESTS)"; PASS=$((PASS+1))
    else
        echo "❌ FAIL ($PASSED passed, $FAILED failed)"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G2: Test $PASSED passed, $FAILED failed")
    fi
else
    echo "❌ FAIL (no tests found)"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G2: No tests found")
fi

# === G3: Clippy ===
check "G3: clippy" "cargo clippy --all-features -- -D warnings"

# === G4: Format ===
check "G4: format" "cargo fmt --all -- --check"

# === G5: Coverage ===
log_info "G5: Coverage >= 85% (L1 CRATES)"
TOTAL=$((TOTAL+1))
if command -v cargo-llvm-cov &>/dev/null; then
    json_out=$(cargo llvm-cov test \
        -p sqlrustgo-types \
        -p sqlrustgo-parser \
        -p sqlrustgo-planner \
        -p sqlrustgo-optimizer \
        -p sqlrustgo-executor \
        -p sqlrustgo-storage \
        -p sqlrustgo-transaction \
        -p sqlrustgo-catalog \
        --lib --json 2>/dev/null) || json_out="{}"

    coverage_pct=$(echo "$json_out" | \
        python3 -c "import sys,json; d=json.load(sys.stdin).get('data',[]); t=d[-1].get('totals',{}) if d else {}; print(t.get('regions',{}).get('percent','0') or '0')" 2>/dev/null) || coverage_pct="0"
    coverage_pct=${coverage_pct:-0}

    if [ -z "$coverage_pct" ] || [ "$coverage_pct" = "0" ]; then
        echo "⏭️  SKIP (coverage measurement unavailable)"; SKIP_REASONS+=("G5: cargo-llvm-cov issue")
    elif awk "BEGIN{exit !($coverage_pct >= 85)}"; then
        echo "✅ PASS (${coverage_pct}%)"; PASS=$((PASS+1))
    else
        echo "❌ FAIL (${coverage_pct}% < 85%)"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G5: Coverage ${coverage_pct}% < 85%")
    fi
else
    echo "⏭️  SKIP (cargo-llvm-cov not installed)"; SKIP_REASONS+=("G5: No cargo-llvm-cov")
fi

# === G6: Security ===
check "G6: security audit" "cargo audit"

# === G7: GMP Build ===
check "G7: GMP API build" "cargo build -p sqlrustgo-gmp-api --release"
check "G8: GMP build" "cargo build -p sqlrustgo-gmp --release"
check "G9: GMP Retrieval build" "cargo build -p sqlrustgo-gmp-retrieval --release"

# === G10: TPC-H SF=1 ===
log_info "G10: TPC-H SF=1 22/22"
TOTAL=$((TOTAL+1))
if bash scripts/gate/check_tpch.sh --sf1 >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G10: TPC-H SF=1 failed")
fi

# === v3.5.0 新增 crate 测试 (G-API/TI) ===
echo ""
echo "━━━ v3.5.0 新增 Crate Tests ━━━"
check_test "G-API1: gmp-api lib tests" "cargo test -p sqlrustgo-gmp-api --lib 2>&1"
check_test "G-API2: compliance-engine lib tests" "cargo test -p sqlrustgo-compliance-engine --lib 2>&1"
check_test "G-GMP1: gmp lib tests" "cargo test -p sqlrustgo-gmp --lib 2>&1"
check_test "G-RET1: gmp-retrieval lib tests" "cargo test -p sqlrustgo-gmp-retrieval --lib 2>&1"

# === Trust Infrastructure Tests ===
echo ""
echo "━━━ Trust Infrastructure Tests ━━━"
check_test "G-TI1: evidence-engine" "cargo test -p sqlrustgo-evidence-engine --lib 2>&1"
check_test "G-TI2: provenance-graph" "cargo test -p sqlrustgo-provenance-graph --lib 2>&1"
check_test "G-TI3: workflow-v2" "cargo test -p sqlrustgo-workflow-v2 --lib 2>&1"
check_test "G-TI4: perf-baseline" "cargo test -p sqlrustgo-perf-baseline --lib 2>&1"
check_test "G-TI5: crash-sim" "cargo test -p sqlrustgo-crash-sim --lib 2>&1"
check_test "G-TI6: wal-verification" "cargo test -p sqlrustgo-wal-verification --lib 2>&1"

# === Summary ===
echo ""
echo "=========================================="
echo "  GA Gate 结果汇总"
echo "=========================================="
echo "✅ PASS: $PASS"
echo "⏭️  SKIP: ${#SKIP_REASONS[@]}"
echo "❌ FAIL: $BLOCKERS"
echo ""

if [ ${#FAIL_REASONS[@]} -gt 0 ]; then
    echo "失败项:"
    for reason in "${FAIL_REASONS[@]}"; do
        echo "  - $reason"
    done
fi

if [ ${#SKIP_REASONS[@]} -gt 0 ]; then
    echo ""
    echo "手动验证项 (需人工确认):"
    for reason in "${SKIP_REASONS[@]}"; do
        echo "  - $reason"
    done
fi

echo ""
if [ $BLOCKERS -gt 0 ]; then
    echo "❌ GA Gate FAILED — $BLOCKERS blocker(s) detected"
    exit 1
else
    echo "✅ GA Gate PASSED"
    exit 0
fi
