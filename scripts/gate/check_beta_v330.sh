#!/usr/bin/env bash
# v3.3.0 Beta Gate (Z6G4 Server)
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PROJECT_ROOT="$(pwd)"
PASS=0; FAIL=0; SKIP=0

check() {
    local name="$1"; shift
    local cmd=("$@")
    echo -n "[beta-v3.3.0] $name ... "
    if "${cmd[@]}" >/dev/null 2>&1; then
        echo "✅ PASS"; PASS=$((PASS+1))
    else
        echo "❌ FAIL"; FAIL=$((FAIL+1))
    fi
}

check_coverage() {
    local name="$1"; local min="$2"; local crate="$3"
    echo -n "[beta-v3.3.0] $name ... "
    local cov=$(cargo llvm-cov -p "sqlrustgo-$crate" --all-features --lib 2>&1 | grep "^TOTAL" | awk '{print $4}' | tr -d '%' || echo "0")
    local result=$(echo "$cov >= $min" | bc -l 2>/dev/null || echo "0")
    if [ "$result" = "1" ]; then
        echo "✅ PASS (${cov}%)"; PASS=$((PASS+1))
    else
        echo "❌ FAIL (${cov}% < ${min}%)"; FAIL=$((FAIL+1))
    fi
}

echo "=== v3.3.0 Beta Gate ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Branch: beta/v3.3.0"
echo "Commit: $(git rev-parse --short HEAD)"
echo ""

echo "=== 代码质量 ==="
check "B1: cargo build --all-features" cargo build --all-features --release
check "B3: cargo clippy" cargo clippy --all-features -- -D warnings
check "B4: cargo fmt" cargo fmt --all -- --check

echo ""
echo "=== Beta Gate (Z6G4) ==="
check "B6: MySQL protocol handshake" bash scripts/gate/check_mysql_handshake.sh

echo ""
echo "=== 覆盖率检查 (Beta >= 65%) ==="
check_coverage "B5: parser coverage >= 75%" 75 "parser"
check_coverage "B5: executor coverage >= 70%" 70 "executor"
check_coverage "B5: planner coverage >= 65%" 65 "planner"
check_coverage "B5: optimizer coverage >= 55%" 55 "optimizer"
check_coverage "B5: storage coverage >= 70%" 70 "storage"
check_coverage "B5: transaction coverage >= 70%" 70 "transaction"

echo ""
echo "=== TPC-H 检查 ==="
check "B7: TPC-H SF=1 22/22" bash scripts/gate/check_tpch.sh

echo ""
echo "=== 结果汇总 ==="
echo "  PASS : $PASS"
echo "  FAIL : $FAIL"
echo "  SKIP : $SKIP"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✅ Beta Gate: PASS ($PASS/$((PASS+FAIL+SKIP)))"
    exit 0
else
    echo "❌ Beta Gate: FAIL ($FAIL 项未通过)"
    exit 1
fi
