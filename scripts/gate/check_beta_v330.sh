#!/bin/bash
#
# v3.3.0 Beta Gate 检查脚本
# 用途: Beta 阶段门禁检查
# 执行: bash scripts/gate/check_beta_v330.sh
#

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== v3.3.0 Beta Gate ==="
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

echo "=== 代码质量检查 (B1-B4) ==="
run_check "B1 Build" "cargo build --release --workspace 2>/dev/null"
run_check "B2 Test" "cargo test --all-features -- --test-threads=4 2>/dev/null"
run_check "B3 Clippy" "cargo clippy --all-features -- -D warnings 2>/dev/null"
run_check "B4 Format" "cargo fmt --all -- --check 2>/dev/null"

echo ""
echo "=== 覆盖率检查 (B5) ==="
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
    if [ -n "$total" ] && [ "$total" -ge 80 ]; then
        echo "✅ PASS (Coverage: ${total}%)"
        return 0
    else
        echo "❌ FAIL (Coverage: ${total}% < 80%)"
        return 1
    fi
}
echo -n "[B5 Coverage L1≥80%] ... "
if check_coverage; then
    PASS=$((PASS+1))
else
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== 安全与文档检查 (B6-B7) ==="
run_check "B6 Security" "cargo audit 2>/dev/null"
run_check "B7 Doc Links" "bash scripts/gate/check_docs_links.sh 2>/dev/null"

echo ""
echo "=== Trust Infrastructure 检查 (B-TI1~TI6) ==="
run_check "B-TI1 perf-baseline tests" "cargo test -p perf-baseline 2>/dev/null"
run_check "B-TI2 crash-sim tests" "cargo test -p crash-sim 2>/dev/null"
run_check "B-TI3 wal-verification tests" "cargo test -p wal-verification 2>/dev/null"
run_check "B-TI4 compliance-engine tests" "cargo test -p compliance-engine 2>/dev/null"
run_check "B-TI5 evidence-engine tests" "cargo test -p evidence-engine 2>/dev/null"
run_check "B-TI6 provenance-graph tests" "cargo test -p provenance-graph 2>/dev/null"

echo ""
echo "=== Z6G4 专属检查 (B8) ==="
run_z6g4 "B8 TPC-H SF=0.1" "需要在 Z6G4 服务器执行"

echo ""
echo "=== 结果汇总 ==="
echo "  PASS : $PASS"
echo "  FAIL : $FAIL"
echo "  SKIP : $SKIP"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✅ Beta Gate: PASS ($PASS+$SKIP/$((PASS+FAIL+SKIP)))"
    exit 0
else
    echo "❌ Beta Gate: FAIL ($FAIL 项未通过)"
    exit 1
fi
