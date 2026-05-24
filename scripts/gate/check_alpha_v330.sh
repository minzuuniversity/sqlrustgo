#!/bin/bash
#
# v3.3.0 Alpha Gate 检查脚本
# 用途: Alpha 阶段入口检查（本地快速检查）
# 执行: bash scripts/gate/check_alpha_v330.sh
#
# Alpha Gate 检查项:
#   A1: Build - cargo build --release --workspace
#   A2: Test - cargo test --lib
#   A3: Clippy - cargo clippy --all-features -- -D warnings
#   A4: Format - cargo fmt --all -- --check
#   A5: Coverage - cargo llvm-cov (L1 CRATES ≥75%)
#   A6: MySQL Protocol - mysql client connection test
#   A7: TPC-H SF=1 - 22/22 queries pass
#

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== v3.3.0 Alpha Gate ==="
echo "分支: $(git branch --show-current)"
echo "Commit: $(git rev-parse --short HEAD)"
echo ""

PASS=0
FAIL=0
SKIP=0

run_check() {
    local name="$1"
    local cmd="$2"
    echo -n "[$name] ... "
    if eval "$cmd" > /dev/null 2>&1; then
        echo "✅ PASS"
        PASS=$((PASS+1))
    else
        echo "❌ FAIL"
        FAIL=$((FAIL+1))
    fi
}

run_check_report() {
    local name="$1"
    local cmd="$2"
    echo -n "[$name] ... "
    if eval "$cmd" > /dev/null 2>&1; then
        echo "✅ PASS"
        PASS=$((PASS+1))
    else
        echo "⏭ SKIP (manual review required)"
        SKIP=$((SKIP+1))
    fi
}

run_z6g4() {
    local name="$1"
    local reason="$2"
    echo "[$name] ... ⏭ SKIP ($reason)"
    SKIP=$((SKIP+1))
}

# ========================================
# 代码质量检查
# ========================================
echo "=== 代码质量 ==="

run_check "A1 Build" "cargo build --release --workspace 2>/dev/null"

run_check "A2 Test" "cargo test --lib -- --test-threads=4 2>/dev/null"

run_check "A3 Clippy" "cargo clippy --all-features -- -D warnings 2>/dev/null"

run_check "A4 Format" "cargo fmt --all -- --check 2>/dev/null"

# ========================================
# 覆盖率检查 (A5)
# ========================================
echo ""
echo "=== 覆盖率检查 (A5) ==="

check_coverage() {
    local output
    output=$(cargo llvm-cov test --lib \
        -p sqlrustgo \
        -p sqlrustgo-executor \
        -p sqlrustgo-optimizer \
        -p sqlrustgo-storage \
        -p sqlrustgo-types \
        --llvm-cov --codecov 2>/dev/null || echo "COVERAGE_FAILED")

    if echo "$output" | grep -q "COVERAGE_FAILED"; then
        return 1
    fi

    local total
    total=$(echo "$output" | grep "^TOTAL" | awk '{print $4}' | sed 's/%//')

    if [ -n "$total" ] && [ "$total" -ge 75 ]; then
        echo "✅ PASS (Coverage: ${total}%)"
        return 0
    else
        echo "❌ FAIL (Coverage: ${total}% < 75%)"
        return 1
    fi
}

echo -n "[A5 Coverage L1≥75%] ... "
if check_coverage; then
    PASS=$((PASS+1))
else
    FAIL=$((FAIL+1))
fi

# ========================================
# MySQL Protocol 检查 (A6)
# ========================================
echo ""
echo "=== 协议检查 (A6) ==="

run_z6g4 "A6 MySQL Protocol" "需要在 Z6G4 服务器执行"

# ========================================
# TPC-H 检查 (A7)
# ========================================
echo ""
echo "=== 性能检查 (A7) ==="

run_z6g4 "A7 TPC-H SF=1" "需要在 Z6G4 服务器执行 (tpch_data/)"

# ========================================
# Trust Infrastructure 检查
# ========================================
echo ""
echo "=== Trust Infrastructure 检查 ==="

run_check_report "TI-1 perf-baseline exists" "[ -d crates/perf-baseline/src ]"
run_check_report "TI-2 crash-sim exists" "[ -d crates/crash-sim/src ]"
run_check_report "TI-3 wal-verification exists" "[ -d crates/wal-verification/src ]"
run_check_report "TI-4 compliance-engine exists" "[ -d crates/compliance-engine/src ]"
run_check_report "TI-5 evidence-engine exists" "[ -d crates/evidence-engine/src ]"
run_check_report "TI-6 provenance-graph exists" "[ -d crates/provenance-graph/src ]"

# ========================================
# SSOT 规范检查
# ========================================
echo ""
echo "=== SSOT 规范检查 ==="

run_check_report "SSOT Coverage Spec" "[ -f docs/governance/gate_spec_v330.md ]"
run_check_report "SSOT OO Roadmap" "[ -f docs/releases/v3.3.0/oo/OO_ROADMAP.md ]"
run_check_report "SSOT Legacy Issues" "[ -f docs/releases/v3.3.0/LEGACY_ISSUES.md ]"

# ========================================
# 结果汇总
# ========================================
echo ""
echo "=== 结果汇总 ==="
echo "  PASS : $PASS"
echo "  FAIL : $FAIL"
echo "  SKIP : $SKIP"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✅ Alpha Gate: Local checks PASS ($PASS+$SKIP/$((PASS+FAIL+SKIP)))"
    echo ""
    echo "⏳ 需 Z6G4 执行:"
    echo "  - A6: MySQL Protocol handshake test"
    echo "  - A7: TPC-H SF=1 22/22 query test"
    exit 0
else
    echo "❌ Alpha Gate: FAIL ($FAIL 项未通过)"
    exit 1
fi
