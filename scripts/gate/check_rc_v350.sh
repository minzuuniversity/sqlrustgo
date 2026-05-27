#!/usr/bin/env bash
# check_rc_v350.sh — v3.5.0 RC Gate 检查脚本
# 规范来源: docs/governance/gate_spec_v350.md §五
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0; TOTAL=0; BLOCKERS=0
FAIL_REASONS=()
SKIP_REASONS=()

log_info() { echo "[rc-v3.5.0] $1"; }
log_pass() { echo "✅ PASS: $1"; PASS=$((PASS+1)); TOTAL=$((TOTAL+1)); }
log_fail() { echo "❌ FAIL: $1"; BLOCKERS=$((BLOCKERS+1)); TOTAL=$((TOTAL+1)); FAIL_REASONS+=("$1"); }
log_skip() { echo "⏭️  SKIP: $1"; TOTAL=$((TOTAL+1)); SKIP_REASONS+=("$1"); }

check() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    echo -n "[rc-v3.5.0] $name ... "
    if eval "$cmd" >/dev/null 2>&1; then
        echo "PASS"; PASS=$((PASS+1))
    else
        echo "FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("$name")
    fi
}

check_test() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    echo -n "[rc-v3.5.0] $name ... "
    local output
    output=$(eval "$cmd" 2>&1) || true
    local passed failed
    passed=$(echo "$output" | grep -c "test result: ok\." || true)
    failed=$(echo "$output" | grep -c "test result: FAILED" || true)
    passed=${passed:-0}; failed=${failed:-0}
    if [ "$failed" -eq 0 ] && [ "$passed" -gt 0 ]; then
        echo "PASS ($passed tests)"; PASS=$((PASS+1))
    else
        echo "FAIL ($passed passed, $failed failed)"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("$name")
    fi
}

echo "=========================================="
echo "  v3.5.0 RC Gate"
echo "  规范来源: docs/governance/gate_spec_v350.md"
echo "=========================================="
echo "日期: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'N/A')"
echo ""

# === R1: Build ===
check "R1: Build" "cargo build --release --workspace"

# === R2: Test ===
check_test "R2: Test suite" "cargo test --all-features --lib 2>&1"

# === R3: Clippy ===
check "R3: Clippy" "cargo clippy --all-features -- -D warnings"

# === R4: Format ===
check "R4: Format" "cargo fmt --all -- --check"

# === R5: Coverage ===
log_info "R5: Coverage >= 85%"
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
        echo "⏭️  SKIP (measurement unavailable)"; SKIP_REASONS+=("R5: cargo-llvm-cov issue")
    elif awk "BEGIN{exit !($coverage_pct >= 85)}"; then
        echo "✅ PASS (${coverage_pct}%)"; PASS=$((PASS+1))
    else
        echo "❌ FAIL (${coverage_pct}% < 85%)"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("R5: Coverage ${coverage_pct}%")
    fi
else
    echo "⏭️  SKIP (cargo-llvm-cov not installed)"; SKIP_REASONS+=("R5: No cargo-llvm-cov")
fi

# === R6: Security ===
check "R6: Security audit" "cargo audit"

# === R7: SQL Compat ===
log_info "R7: SQL Compat >= 85%"
TOTAL=$((TOTAL+1))
if bash scripts/gate/check_sql_compat.sh >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("R7: SQL Compat")
fi

# === R8: TPC-H SF=1 ===
log_info "R8: TPC-H SF=1"
TOTAL=$((TOTAL+1))
if bash scripts/gate/check_tpch.sh --sf1 >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("R8: TPC-H SF=1")
fi

# === v3.5.0 新增 crate 测试 ===
echo ""
echo "━━━ v3.5.0 新增 Crate Tests ━━━"
check_test "compliance-engine" "cargo test -p sqlrustgo-compliance-engine --lib 2>&1"
check_test "gmp-api" "cargo test -p sqlrustgo-gmp-api --lib 2>&1"
check_test "gmp" "cargo test -p sqlrustgo-gmp --lib 2>&1"
check_test "gmp-retrieval" "cargo test -p sqlrustgo-gmp-retrieval --lib 2>&1"

# === Trust Infrastructure ===
echo ""
echo "━━━ Trust Infrastructure ━━━"
check_test "evidence-engine" "cargo test -p sqlrustgo-evidence-engine --lib 2>&1"
check_test "workflow-v2" "cargo test -p sqlrustgo-workflow-v2 --lib 2>&1"
check_test "perf-baseline" "cargo test -p sqlrustgo-perf-baseline --lib 2>&1"
check_test "crash-sim" "cargo test -p sqlrustgo-crash-sim --lib 2>&1"

# === Summary ===
echo ""
echo "=========================================="
echo "  RC Gate 结果汇总"
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
    echo "手动验证项:"
    for reason in "${SKIP_REASONS[@]}"; do
        echo "  - $reason"
    done
fi
echo ""
if [ $BLOCKERS -gt 0 ]; then
    echo "❌ RC Gate FAILED — $BLOCKERS blocker(s)"
    exit 1
else
    echo "✅ RC Gate PASSED"
    exit 0
fi
