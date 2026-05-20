#!/bin/bash
#
# v3.3.0 RC Gate 检查脚本
# 用途: RC 阶段门禁检查
# 执行: bash scripts/gate/check_rc_v330.sh
#

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== v3.3.0 RC Gate ==="
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

echo "=== 代码质量检查 (R1-R4) ==="
run_check "R1 Build" "cargo build --release --workspace 2>/dev/null"
run_check "R2 Test" "cargo test --all-features -- --test-threads=4 2>/dev/null"
run_check "R3 Clippy" "cargo clippy --all-features -- -D warnings 2>/dev/null"
run_check "R4 Format" "cargo fmt --all -- --check 2>/dev/null"

echo ""
echo "=== 覆盖率检查 (R5) ==="
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
echo -n "[R5 Coverage L1≥85%] ... "
if check_coverage; then
    PASS=$((PASS+1))
else
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== 安全与兼容性检查 (R6-R8) ==="
run_check "R6 Security" "cargo audit 2>/dev/null"
run_check "R7 SQL Compat" "bash scripts/gate/check_sql_compat.sh 2>/dev/null"
run_z6g4 "R8 TPC-H SF=1" "需要在 Z6G4 服务器执行"

echo ""
echo "=== RC 稳定性测试 (R-S1~S4) ==="
run_z6g4 "R-S1 16h Write" "需要在 Z6G4 服务器执行"
run_z6g4 "R-S2 16h Read/Write" "需要在 Z6G4 服务器执行"
run_z6g4 "R-S3 24h Write" "需要在 Z6G4 服务器执行"
run_z6g4 "R-S4 24h Read/Write" "需要在 Z6G4 服务器执行"

echo ""
echo "=== 结果汇总 ==="
echo "  PASS : $PASS"
echo "  FAIL : $FAIL"
echo "  SKIP : $SKIP"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✅ RC Gate: PASS ($PASS+$SKIP/$((PASS+FAIL+SKIP)))"
    exit 0
else
    echo "❌ RC Gate: FAIL ($FAIL 项未通过)"
    exit 1
fi
