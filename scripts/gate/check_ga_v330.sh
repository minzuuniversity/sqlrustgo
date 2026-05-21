#!/bin/bash
#
# v3.3.0 GA Gate 检查脚本
# 用途: GA 阶段门禁检查
# 执行: bash scripts/gate/check_ga_v330.sh
#

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== v3.3.0 GA Gate ==="
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

run_z6g4() {
    local name="$1"
    local reason="$2"
    echo "[$name] ... ⏭ SKIP ($reason)"
    SKIP=$((SKIP+1))
}

echo "=== 代码质量检查 (G1-G4) ==="
run_check "G1 Build" "cargo build --release --workspace 2>/dev/null"
run_check "G2 Test" "cargo test --all-features -- --test-threads=4 2>/dev/null"
run_check "G3 Clippy" "cargo clippy --all-features -- -D warnings 2>/dev/null"
run_check "G4 Format" "cargo fmt --all -- --check 2>/dev/null"

echo ""
echo "=== 覆盖率检查 (G5) ==="
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
    if [ -n "$total" ] && [ "$total" -ge 85 ]; then
        echo "✅ PASS (Coverage: ${total}%)"
        return 0
    else
        echo "❌ FAIL (Coverage: ${total}% < 85%)"
        return 1
    fi
}
echo -n "[G5 Coverage L1≥85%] ... "
if check_coverage; then
    PASS=$((PASS+1))
else
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== 安全与兼容性检查 (G6-G8) ==="
run_check "G6 Security" "cargo audit 2>/dev/null"
run_check "G7 SQL Compat" "bash scripts/gate/check_sql_compat.sh 2>/dev/null"
run_z6g4 "G8 TPC-H SF=1" "需要在 Z6G4 服务器执行"

echo ""
echo "=== GA 性能基准 (G-P1~P3) ==="
run_z6g4 "G-P1 Point Select QPS" "需要在 Z6G4 服务器执行"
run_z6g4 "G-P2 Update QPS" "需要在 Z6G4 服务器执行"
run_z6g4 "G-P3 Delete QPS" "需要在 Z6G4 服务器执行"

echo ""
echo "=== GA 稳定性测试 (G-S1~S4) ==="
run_z6g4 "G-S1 48h Write" "需要在 Z6G4 服务器执行"
run_z6g4 "G-S2 48h Read/Write" "需要在 Z6G4 服务器执行"
run_z6g4 "G-S3 72h Write" "需要在 Z6G4 服务器执行"
run_z6g4 "G-S4 72h Read/Write" "需要在 Z6G4 服务器执行"

echo ""
echo "=== Trust Infrastructure 验证 (G-TI1~TI6) ==="
run_check "G-TI1 Perf Governance" "cargo test -p perf-baseline 2>/dev/null"
run_check "G-TI2 Crash Simulation" "cargo test -p crash-sim 2>/dev/null"
run_check "G-TI3 WAL Verification" "cargo test -p wal-verification 2>/dev/null"
run_check "G-TI4 Compliance Engine" "cargo test -p compliance-engine 2>/dev/null"
run_check "G-TI5 Evidence Engine" "cargo test -p evidence-engine 2>/dev/null"
run_check "G-TI6 Provenance Graph" "cargo test -p provenance-graph 2>/dev/null"

echo ""
echo "=== 结果汇总 ==="
echo "  PASS : $PASS"
echo "  FAIL : $FAIL"
echo "  SKIP : $SKIP"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✅ GA Gate: PASS ($PASS+$SKIP/$((PASS+FAIL+SKIP)))"
    exit 0
else
    echo "❌ GA Gate: FAIL ($FAIL 项未通过)"
    exit 1
fi
