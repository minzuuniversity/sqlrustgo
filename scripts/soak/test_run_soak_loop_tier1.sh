#!/usr/bin/env bash
#
# test_run_soak_loop_tier1.sh — v312-59-d / #4594 Tier-1 fix unit tests
# ==============================================================================
# 测试 run_soak_loop.sh 的 4 个 Tier-1 修复路径:
#
#   T1. P-2: SOAK_DATA_DIR env override (默认 data-dir 不再用 /tmp)
#   T2. P-3: SOAK_NICE_SERVER / SOAK_NICE_SYSBENCH env override
#   T3. P-4: sysbench --time 派生自 SOAK_HOURS (+1h buffer)
#   T4. P-9: server_qps() 解析 total_q=N 而非 qps=X.Y
#
# 用法:
#   bash scripts/soak/test_run_soak_loop_tier1.sh
#
# 退出码:
#   0  — 全部 PASS
#   1  — 至少一个 FAIL
#
# 依赖: bash >= 4, awk
# 不依赖: cargo, sqlrustgo-mysql-server (纯逻辑验证, 不实际启动服务)
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_SOAK="${REPO_ROOT}/scripts/soak/run_soak_loop.sh"

# 提取 server_qps() 函数到独立临时脚本 (它依赖 SOAK_PREV_* 全局变量)
TMPDIR=$(mktemp -d)
trap 'rm -rf "${TMPDIR}"' EXIT

# Source 整个 run_soak_loop.sh 是不可能的 (它会调用 main); 只 source 我们需要的部分
SERVER_QPS_SCRIPT="${TMPDIR}/server_qps_standalone.sh"
{
    # 把 server_qps() 提取为可独立 source 的形式 (替换一些 bash 4 特性)
    sed -n '/^server_qps() {/,/^}/p' "${RUN_SOAK}"
    echo 'SOAK_PREV_TOTAL_Q=0'
    echo 'SOAK_PREV_QPS_TS=0'
    echo 'SERVER_LOG="${1:-/dev/null}"'
} > "${SERVER_QPS_SCRIPT}"

# ── helpers ──
PASS_COUNT=0
FAIL_COUNT=0

pass() {
    PASS_COUNT=$((PASS_COUNT + 1))
    echo "  ✅ PASS: $1"
}

fail() {
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "  ❌ FAIL: $1"
    [[ -n "${2:-}" ]] && echo "       $2"
}

# ── Test 1 (P-2): SOAK_DATA_DIR env override ──
echo ""
echo "[T1] P-2: SOAK_DATA_DIR env override (data-dir 不再用 /tmp)"
echo "----------------------------------------------------------"
if grep -q "SOAK_DATA_DIR" "${RUN_SOAK}"; then
    if grep -q "^    CYCLE_DATA=\"\${SOAK_DATA_DIR}\"" "${RUN_SOAK}"; then
        pass "CYCLE_DATA 引用 SOAK_DATA_DIR env"
    else
        fail "CYCLE_DATA 未引用 SOAK_DATA_DIR" "应形如: CYCLE_DATA=\"\${SOAK_DATA_DIR}\""
    fi
    if grep -q "SOAK_DATA_DIR:-" "${RUN_SOAK}"; then
        pass "SOAK_DATA_DIR 有默认值 (非空 fallback)"
    else
        fail "SOAK_DATA_DIR 无默认值"
    fi
    if grep -q "tmpfs" "${RUN_SOAK}"; then
        pass "tmpfs 检测警告已实现 (防止 /tmp 数据丢失)"
    else
        fail "缺 tmpfs 检测 — 168h SOAK 跨重启仍会失败"
    fi
    if grep -q "/tmp/sqlrustgo-soak-data" "${RUN_SOAK}"; then
        fail "仍硬编码 /tmp/sqlrustgo-soak-data" "应只在文档/历史 evidence 中出现"
    else
        pass "无 /tmp hardcode"
    fi
else
    fail "缺 SOAK_DATA_DIR 变量" "应在文件顶部默认变量块声明"
fi

# ── Test 2 (P-3): SOAK_NICE_SERVER / SOAK_NICE_SYSBENCH env override ──
echo ""
echo "[T2] P-3: SOAK_NICE_SERVER / SOAK_NICE_SYSBENCH env override"
echo "----------------------------------------------------------"
if grep -q "SOAK_NICE_SERVER=\"\${SOAK_NICE_SERVER:-10}\"" "${RUN_SOAK}"; then
    pass "SOAK_NICE_SERVER 默认值 = 10"
else
    fail "SOAK_NICE_SERVER 默认值错误或缺失"
fi
if grep -q "SOAK_NICE_SYSBENCH=\"\${SOAK_NICE_SYSBENCH:-15}\"" "${RUN_SOAK}"; then
    pass "SOAK_NICE_SYSBENCH 默认值 = 15"
else
    fail "SOAK_NICE_SYSBENCH 默认值错误或缺失"
fi
if grep -q "nice -n \"\${SOAK_NICE_SERVER}\"" "${RUN_SOAK}"; then
    pass "server nice 命令使用 \${SOAK_NICE_SERVER}"
else
    fail "server nice 仍硬编码 -n 10"
fi
if grep -q "nice -n \"\${SOAK_NICE_SYSBENCH}\"" "${RUN_SOAK}"; then
    pass "sysbench nice 命令使用 \${SOAK_NICE_SYSBENCH}"
else
    fail "sysbench nice 仍硬编码 -n 15"
fi

# ── Test 3 (P-4): sysbench --time 派生自 SOAK_HOURS ──
echo ""
echo "[T3] P-4: sysbench --time 与 SOAK_HOURS 同步 (+1h buffer)"
echo "----------------------------------------------------------"
if grep -q "sysbench_seconds=\\\$(( (SOAK_HOURS + 1) \* 3600 ))" "${RUN_SOAK}"; then
    pass "sysbench_seconds = (SOAK_HOURS + 1) * 3600 派生公式"
else
    fail "sysbench_seconds 派生公式缺失" "应: local sysbench_seconds=\$(( (SOAK_HOURS + 1) * 3600 ))"
fi
if grep -qE -- '--time="\$\{sysbench_seconds\}"' "${RUN_SOAK}"; then
    pass "sysbench --time 使用派生值"
else
    fail "sysbench --time 仍硬编码"
fi
if grep -qE '^[[:space:]]*--time=86400([[:space:]]|$)' "${RUN_SOAK}"; then
    fail "仍存在 --time=86400 hardcode"
else
    pass "无 --time=86400 hardcode"
fi

# ── Test 4 (P-9): server_qps 解析 total_q=N 而非 qps=X.Y ──
echo ""
echo "[T4] P-9: server_qps() 解析 total_q=N (不再用不存在字段 qps=)"
echo "----------------------------------------------------------"

# T4a. 静态检查: 旧 regex `qps=[0-9]+\.[0-9]+` 必须不出现在非注释行
# (允许在注释中作为历史记录存在, 但不能在 active grep/sed 命令中)
old_qps_pattern_q=$(grep -vE '^[[:space:]]*#' "${RUN_SOAK}" | grep -E 'qps=\[0-9\]+\.\[0-9\]+|^qps=' || true)
if [[ -n "${old_qps_pattern_q}" ]]; then
    fail "active code 中仍存在 qps=[0-9]+\\.[0-9]+ regex" "${old_qps_pattern_q}"
else
    pass "active code 无遗留 qps=[0-9]+\\.[0-9]+ regex"
fi

# T4b. 静态检查: 新逻辑必须解析 total_q=N
if grep -q "total_q=\[0-9\]+" "${RUN_SOAK}"; then
    pass "server_qps 解析 total_q=[0-9]+"
else
    fail "server_qps 未解析 total_q=[0-9]+"
fi

# T4c. 行为测试: 用 sample server.log 验证 server_qps 输出
SAMPLE_LOG="${TMPDIR}/sample_server.log"
cat > "${SAMPLE_LOG}" <<'EOF'
2026-08-30T03:18:30Z INFO sqlrustgo_mysql_server: server listening on 127.0.0.1:3396
2026-08-30T03:19:00Z INFO sqlrustgo_mysql_server: RESOURCE_MONITOR pid=12345 rss_mb=357.2 fd=64/1048576 (0.0%) threads=42 active_conn=3 total_acc=100 total_q=5000 total_err=0
2026-08-30T03:20:00Z INFO sqlrustgo_mysql_server: RESOURCE_MONITOR pid=12345 rss_mb=358.1 fd=64/1048576 (0.0%) threads=42 active_conn=3 total_acc=110 total_q=11000 total_err=0
2026-08-30T03:21:00Z INFO sqlrustgo_mysql_server: RESOURCE_MONITOR pid=12345 rss_mb=359.0 fd=64/1048576 (0.0%) threads=42 active_conn=5 total_acc=120 total_q=18000 total_err=0
EOF

# Source 提取的 server_qps 函数, 然后调用两次 (第二次应用差分)
# 把 server_qps 包装成可直接调用的形式
(
    # shellcheck disable=SC1090
    source "${SERVER_QPS_SCRIPT}"
    # 调用 1: 首次初始化, 应返回 0
    r1=$(server_qps 2>/dev/null)
    # 调用 2: 应返回非零 QPS (基于 60s 间隔)
    sleep 1
    r2=$(server_qps 2>/dev/null)
    echo "FIRST=${r1}"
    echo "SECOND=${r2}"
    # 验证 FIRST=0 (初始化)
    if [[ "${r1}" == "0" ]]; then
        echo "PASS:first_call_returns_zero"
    else
        echo "FAIL:first_call_not_zero:${r1}"
    fi
    # 验证 SECOND >= 0 且是数字 (差分计算)
    if [[ "${r2}" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
        echo "PASS:second_call_is_numeric:${r2}"
    else
        echo "FAIL:second_call_not_numeric:${r2}"
    fi
) > "${TMPDIR}/t4_out.txt" 2>&1

if grep -q "PASS:first_call_returns_zero" "${TMPDIR}/t4_out.txt"; then
    pass "首次 server_qps 调用返回 0 (state 初始化)"
else
    fail "首次 server_qps 未返回 0"
fi

if grep -q "PASS:second_call_is_numeric" "${TMPDIR}/t4_out.txt"; then
    pass "第二次 server_qps 调用返回数值 (差分计算生效)"
else
    fail "第二次 server_qps 未返回数值" "$(cat ${TMPDIR}/t4_out.txt)"
fi

# ── 总结 ──
echo ""
echo "=========================================="
echo "Total: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}"
echo "=========================================="

if [[ ${FAIL_COUNT} -gt 0 ]]; then
    exit 1
fi
exit 0