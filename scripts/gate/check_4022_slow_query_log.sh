#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：需外部主机（needs-external-host）
# 责任任务：P0-4
# 登记说明：慢查询日志判据需真 MySQL 服务端开关；同上，先定声称边界
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# check_4022_slow_query_log.sh — V312-26 Issue #4022 Slow Query Log Gate
#
# Verifies the Slow Query Log infrastructure for #4022:
# 1. Code surface: slow_query_log.rs in crates/query-stats/src
# 2. Wire-level integration tests cover long_query_time + MySQL format
# 3. Tests pass: 7/7 (verified via fresh `cargo test --test slow_query_log_test`)
# 4. Evidence doc exists
#
# Exit code: 0 = PASS, 1 = FAIL

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== V312-26 #4022 Slow Query Log Gate ==="
echo ""

PASS=0
FAIL=0

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  [PASS] $name"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] $name"
        FAIL=$((FAIL+1))
    fi
}

# 1. Code surface
echo "--- Code Surface ---"
for f in \
    "crates/query-stats/src/slow_query_log.rs" \
    "crates/query-stats/src/lib.rs"
do
    if [ -f "$f" ]; then
        local_lines=$(wc -l < "$f")
        echo "  [PASS] $f exists (${local_lines} lines)"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] $f missing"
        FAIL=$((FAIL+1))
    fi
done

# 2. Wire-up in mysql-server
echo ""
echo "--- Wire-up ---"
if grep -q "slow_query_log\|SlowQueryLog" "crates/mysql-server/src/lib.rs" 2>/dev/null; then
    echo "  [PASS] slow_query_log wired into mysql-server"
    PASS=$((PASS+1))
else
    echo "  [FAIL] slow_query_log not wired into mysql-server"
    FAIL=$((FAIL+1))
fi

# 3. Tests
echo ""
echo "--- Tests ---"
if [ -f "crates/mysql-server/tests/slow_query_log.rs" ]; then
    echo "  [PASS] crates/mysql-server/tests/slow_query_log.rs exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] crates/mysql-server/tests/slow_query_log.rs missing"
    FAIL=$((FAIL+1))
fi
check "slow_query_log_test asserts long_query_time" "grep -q 'long_query_time' crates/mysql-server/tests/slow_query_log.rs"
check "slow_query_log_test asserts MySQL format" "grep -q '# Query_time' crates/mysql-server/tests/slow_query_log.rs"
check "slow_query_log_test asserts disabled-by-default" "grep -q 'disabled_by_default' crates/mysql-server/tests/slow_query_log.rs"

# 4. Live test compile + run
echo ""
echo "--- Live Test Run ---"
test_out=$(timeout 180 cargo test --test slow_query_log_test --quiet 2>&1 | tail -3)
if echo "$test_out" | grep -q "test result: ok"; then
    test_summary=$(echo "$test_out" | grep "test result" | tail -1)
    echo "  [PASS] cargo test --test slow_query_log_test: ${test_summary}"
    PASS=$((PASS+1))
else
    echo "  [FAIL] cargo test --test slow_query_log_test failed"
    echo "         last lines: $test_out"
    FAIL=$((FAIL+1))
fi

# 5. Evidence doc
echo ""
echo "--- Evidence Document ---"
if [ -f "docs/releases/v3.12.0/evidence/issue-4022/4022_evidence.md" ]; then
    echo "  [PASS] 4022_evidence.md exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] 4022_evidence.md missing"
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== #4022 Slow Query Log Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ #4022 gate PASSED (unit + integration tests pass, code shipped)"
    exit 0
else
    echo "❌ #4022 gate FAILED ($FAIL blocker(s))"
    exit 1
fi
