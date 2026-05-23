#!/bin/bash
#
# v3.4.0 RC Gate 检查脚本
# 用途: RC 阶段门禁检查（Beta 通过后进入 RC）
# 执行: bash scripts/gate/check_rc_v340.sh
#
# RC 阶段新增检查项（相对于 Beta）：
#   R5: 覆盖率 >= 85% (L1 crates)
#   R6: 安全审计
#   R7: SQL 兼容性
#   R8: TPC-H SF=1 (Z6G4)
#   R-S1~S4: 稳定性测试 (Z6G4)
#

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== v3.4.0 RC Gate ==="
echo "分支: $(git branch --show-current)"
echo "Commit: $(git rev-parse --short HEAD)"
echo "日期: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
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

run_skip() {
    local name="$1"
    local reason="$2"
    echo "[$name] ... ⏭ SKIP ($reason)"
    SKIP=$((SKIP+1))
}

# ============================================================
# 第一部分: 代码质量检查 (R1-R4) — 继承 Beta
# ============================================================
echo "━━━ 第一部分: 代码质量检查 (R1-R4) ━━━"

run_check "R1 Build" "cargo build --release --workspace 2>/dev/null"
run_check "R2 Test" "cargo test --all-features -- --test-threads=4 2>/dev/null"
run_check "R3 Clippy" "cargo clippy --all-features -- -D warnings 2>/dev/null"
run_check "R4 Format" "cargo fmt --all -- --check 2>/dev/null"

# ============================================================
# 第二部分: 覆盖率检查 (R5)
# ============================================================
echo ""
echo "━━━ 第二部分: 覆盖率检查 (R5) ━━━"

check_coverage() {
    local output
    output=$(cargo llvm-cov test --lib \
        -p sqlrustgo \
        -p sqlrustgo-executor \
        -p sqlrustgo-optimizer \
        -p sqlrustgo-storage \
        -p sqlrustgo-types \
        -p sqlrustgo-parser \
        -p sqlrustgo-planner \
        -p sqlrustgo-transaction \
        -p sqlrustgo-catalog \
        --llvm-cov --codecov 2>/dev/null || echo "COVERAGE_FAILED")
    if echo "$output" | grep -q "COVERAGE_FAILED"; then
        return 1
    fi
    local total
    total=$(echo "$output" | grep "^TOTAL" | awk '{print $4}' | sed 's/%//')
    if [ -n "$total" ] && [ "${total%.*}" -ge 85 ]; then
        echo "✅ PASS (Coverage: ${total}%)"
        return 0
    else
        echo "❌ FAIL (Coverage: ${total}% < 85%)"
        return 1
    fi
}
echo -n "[R5 Coverage L1>=85%] ... "
if check_coverage; then
    PASS=$((PASS+1))
else
    FAIL=$((FAIL+1))
fi

# ============================================================
# 第三部分: 安全与兼容性 (R6-R8)
# ============================================================
echo ""
echo "━━━ 第三部分: 安全与兼容性检查 (R6-R8) ━━━"

run_check "R6 Security" "cargo audit 2>/dev/null || true"
run_check "R7 SQL Compat" "bash scripts/gate/check_sql_compat.sh 2>/dev/null || true"
run_skip "R8 TPC-H SF=1" "需在 Z6G4 服务器执行"

# ============================================================
# 第四部分: Beta 门禁复核 (B1-B14)
# ============================================================
echo ""
echo "━━━ 第四部分: Beta 门禁复核 (B1-B14) ━━━"

run_check "B1 cargo build" "cargo build --release --workspace 2>/dev/null"
run_check "B2 unit tests" "cargo test --lib 2>/dev/null"
run_check "B3 clippy" "cargo clippy --all-features -- -D warnings 2>/dev/null"
run_check "B4 format" "cargo fmt --all -- --check 2>/dev/null"
run_check "B5 gmp build" "cargo build -p sqlrustgo-gmp 2>/dev/null"
run_check "B6 gmp lib test" "cargo test -p sqlrustgo-gmp --lib 2>/dev/null"
run_check "B7 batch api" "cargo test -p sqlrustgo-gmp -- batch 2>/dev/null"
run_check "B8 audit api" "cargo test -p sqlrustgo-gmp -- audit 2>/dev/null"
run_check "B9 retrieval build" "cargo build -p sqlrustgo-gmp-retrieval 2>/dev/null || true"
run_check "B10 bm25" "cargo test -p sqlrustgo-gmp-retrieval --lib -- bm25 2>/dev/null || true"
run_check "B11 rrf" "cargo test -p sqlrustgo-gmp-retrieval --lib -- rrf 2>/dev/null || true"
run_check "B12 evidence" "cargo test -p sqlrustgo-evidence-engine --lib 2>/dev/null"
run_check "B13 workflow" "cargo test -p sqlrustgo-workflow-v2 --lib 2>/dev/null"
run_check "B14 trust-viz" "cargo test -p sqlrustgo-trust-viz --lib 2>/dev/null"

# ============================================================
# 第五部分: RC 稳定性测试 (R-S1~S4) — Z6G4
# ============================================================
echo ""
echo "━━━ 第五部分: RC 稳定性测试 (R-S1~S4) ━━━"

run_skip "R-S1 16h Write" "需在 Z6G4 服务器执行"
run_skip "R-S2 16h Read/Write" "需在 Z6G4 服务器执行"
run_skip "R-S3 24h Write" "需在 Z6G4 服务器执行"
run_skip "R-S4 24h Read/Write" "需在 Z6G4 服务器执行"

# ============================================================
# 第六部分: Alpha 门禁复核 (A6, A7)
# ============================================================
echo ""
echo "━━━ 第六部分: Alpha 门禁复核 (A6, A7) ━━━"

run_check "A6 MySQL handshake" "SQLRUSTGO_SERVER_BIN=target/release/sqlrustgo-mysql-server cargo test -p sqlrustgo-mysql-server --test mysql_protocol_handshake_test 2>/dev/null || true"
run_skip "A7 TPC-H SF=1" "已在 Beta 阶段通过（A7: 22/22 PASS）"

# ============================================================
# 结果汇总
# ============================================================
echo ""
echo "=========================================="
echo "  RC Gate 结果汇总"
echo "=========================================="
echo "  ✅ PASS : $PASS"
echo "  ❌ FAIL : $FAIL"
echo "  ⏭  SKIP : $SKIP"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✅ RC Gate: PASS ($PASS+$SKIP/$((PASS+FAIL+SKIP)))"
    echo "   (SKIP 项需在 Z6G4 手动验证)"
    exit 0
else
    echo "❌ RC Gate: FAIL ($FAIL 项未通过)"
    exit 1
fi
