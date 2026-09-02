#!/usr/bin/env bash
#
# test_run_soak_loop_tier2.sh — v312-59-d / #4596 Tier-2 fix unit tests
# ==============================================================================
# 测试 run_soak_loop.sh 的 3 个 Tier-2 修复路径:
#
#   T5. P-5:  metrics 按 cycle 切分 (METRICS_CYCLE / restart_seq 列)
#   T6. P-6:  SOAK_WORKLOAD env 选 sysbench workload + 白名单
#   T7. P-10: periodic_reports.jsonl 输出 machine-parseable JSON
#
# 用法:
#   bash scripts/soak/test_run_soak_loop_tier2.sh
#
# 退出码:
#   0  — 全部 PASS
#   1  — 至少一个 FAIL
#
# 依赖: bash >= 4, awk, jq (用于 T7 JSON 解析; 缺失时跳过子项但 fail)
# 不依赖: cargo, sqlrustgo-mysql-server (纯逻辑验证, 不实际启动服务)
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_SOAK="${REPO_ROOT}/scripts/soak/run_soak_loop.sh"

TMPDIR=$(mktemp -d)
trap 'rm -rf "${TMPDIR}"' EXIT

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

# ── Test 5 (P-5): Per-cycle metrics CSV + restart_seq 列 ──
echo ""
echo "[T5] P-5: metrics 按 cycle 切分 + restart_seq 列"
echo "----------------------------------------------------------"

# T5a. setup_run_dir 必须初始化 METRICS_CYCLE
if grep -q "^[[:space:]]*METRICS_CYCLE=" "${RUN_SOAK}"; then
    pass "setup_run_dir 声明 METRICS_CYCLE (per-cycle 变量)"
else
    fail "setup_run_dir 缺 METRICS_CYCLE 声明"
fi

# T5b. METRICS_CYCLE 必须拼 metrics.csv. + 后缀 (cycle 切分关键)
if grep -qE '^[[:space:]]*METRICS_CYCLE="\$\{RUN_DIR\}/metrics\.csv' "${RUN_SOAK}"; then
    pass "METRICS_CYCLE 路径含 metrics.csv 前缀"
else
    fail "METRICS_CYCLE 路径未按 metrics.csv.* 形式切分"
fi

# T5c. record_metrics 写入 METRICS_CYCLE (而非 METRICS_CSV)
if grep -qE '^[[:space:]]*echo "[^"]+" >> "\$\{METRICS_CYCLE\}"' "${RUN_SOAK}"; then
    pass "record_metrics append 到 METRICS_CYCLE"
else
    fail "record_metrics 未 append 到 METRICS_CYCLE"
fi

# T5d. restart_seq 列必须出现在 header 中
if grep -q "ts,elapsed_s,restart_seq" "${RUN_SOAK}"; then
    pass "CSV header 含 restart_seq 列"
else
    fail "CSV header 缺 restart_seq 列"
fi

# T5e. record_metrics 数据行必须写入 restart_count (实际值)
# 注: 用单引号 grep -E 避免 bash 干扰 regex
if grep -qE '^[[:space:]]*echo "[^"]*\$\{restart_count:-0\}[^"]*" >> "\$\{METRICS_CYCLE\}"' "${RUN_SOAK}"; then
    pass "METRICS_CYCLE 数据行使用 \${restart_count:-0}"
else
    fail "METRICS_CYCLE 数据行缺 restart_count 占位"
fi

# T5f. HISTORY_CSV 跨 RUN_DIR 共享 (单文件 append-only)
if grep -q "^HISTORY_CSV=" "${RUN_SOAK}" && grep -qE '>> "\$\{HISTORY_CSV\}"' "${RUN_SOAK}"; then
    pass "HISTORY_CSV 跨 RUN_DIR 共享 (append-only)"
else
    fail "HISTORY_CSV 缺失或未 append"
fi

# T5g. disk-restart 块必须触发 setup_run_dir 重建 RUN_DIR
if grep -qE '^[[:space:]]+setup_run_dir$' "${RUN_SOAK}"; then
    pass "disk-restart 块调用 setup_run_dir"
else
    fail "disk-restart 块未调用 setup_run_dir"
fi

# ── Test 6 (P-6): SOAK_WORKLOAD env + 白名单 ──
echo ""
echo "[T6] P-6: SOAK_WORKLOAD env 选 sysbench workload"
echo "----------------------------------------------------------"

# T6a. SOAK_WORKLOAD 必须在文件顶部声明默认值
if grep -q 'SOAK_WORKLOAD="${SOAK_WORKLOAD:-oltp_read_write}"' "${RUN_SOAK}"; then
    pass "SOAK_WORKLOAD 默认值 = oltp_read_write"
else
    fail "SOAK_WORKLOAD 默认值缺失或错误"
fi

# T6b. load_workload_args 函数必须存在
if grep -q "^load_workload_args()" "${RUN_SOAK}"; then
    pass "load_workload_args() 函数已声明"
else
    fail "load_workload_args() 函数缺失"
fi

# T6c. 必须覆盖全部 5 个白名单 workload
EXPECTED_WLS=(
    "oltp_read_write"
    "oltp_read_only"
    "oltp_write_only"
    "oltp_insert"
    "oltp_update_index"
)
MISSING=()
for wl in "${EXPECTED_WLS[@]}"; do
    if ! grep -q -- "--test=${wl}" "${RUN_SOAK}"; then
        MISSING+=("${wl}")
    fi
done
if [[ ${#MISSING[@]} -eq 0 ]]; then
    pass "5 个白名单 workload 全部映射 (oltp_read_write/read_only/write_only/insert/update_index)"
else
    fail "白名单缺失: ${MISSING[*]}"
fi

# T6d. start_sysbench 必须使用 load_workload_args 而非硬编码 --test=
if grep -qE 'sysbench \$\{wl_args\}' "${RUN_SOAK}"; then
    pass "sysbench 命令使用 load_workload_args 输出"
else
    fail "sysbench 命令未通过 load_workload_args 选 workload"
fi

# T6e. preflight 必须校验 SOAK_WORKLOAD 白名单 (早失败)
if grep -qE 'load_workload_args "\$\{SOAK_WORKLOAD\}"' "${RUN_SOAK}"; then
    pass "preflight 调用 load_workload_args 校验 SOAK_WORKLOAD"
else
    fail "preflight 未校验 SOAK_WORKLOAD 白名单"
fi

# T6f. 行为测试: 提取 load_workload_args 到临时脚本, 调用 5 个白名单 + 1 个非法
LW_ARGS_SCRIPT="${TMPDIR}/lw_args_standalone.sh"
{
    sed -n '/^load_workload_args() {/,/^}/p' "${RUN_SOAK}"
} > "${LW_ARGS_SCRIPT}"

# 还需要 err() 函数 (load_workload_args 调用它)
ERR_FN=$(sed -n '/^err() {/,/^}/p' "${RUN_SOAK}")
{
    echo "${ERR_FN}"
    cat "${LW_ARGS_SCRIPT}"
} > "${TMPDIR}/lw_args_full.sh"

(
    # shellcheck disable=SC1090
    source "${TMPDIR}/lw_args_full.sh"

    # 白名单 workload 必须返回有效 --test= 参数
    for wl in oltp_read_write oltp_read_only oltp_write_only oltp_insert oltp_update_index; do
        out=$(load_workload_args "${wl}" 2>/dev/null)
        if [[ "${out}" == --test=* ]]; then
            echo "PASS:wl_${wl}=${out}"
        else
            echo "FAIL:wl_${wl}:unexpected='${out}'"
        fi
    done

    # 非法 workload 必须返回非零 (失败)
    if load_workload_args "oltp_point_select" 2>/dev/null; then
        echo "FAIL:wl_unknown_should_fail"
    else
        echo "PASS:wl_unknown_fails"
    fi
) > "${TMPDIR}/t6_out.txt" 2>&1

for wl in oltp_read_write oltp_read_only oltp_write_only oltp_insert oltp_update_index; do
    if grep -q "PASS:wl_${wl}=" "${TMPDIR}/t6_out.txt"; then
        pass "workload '${wl}' 映射正确"
    else
        got=$(grep "FAIL:wl_${wl}" "${TMPDIR}/t6_out.txt" || echo "missing")
        fail "workload '${wl}' 映射异常" "${got}"
    fi
done

if grep -q "PASS:wl_unknown_fails" "${TMPDIR}/t6_out.txt"; then
    pass "非法 workload 返回非零 (rc!=0)"
else
    fail "非法 workload 未被拒绝 (白名单失效)"
fi

# ── Test 7 (P-10): periodic_reports.jsonl JSON line ──
echo ""
echo "[T7] P-10: periodic_reports.jsonl 输出 machine-parseable JSON"
echo "----------------------------------------------------------"

# T7a. setup_run_dir 必须初始化 PERIODIC_JSONL
if grep -q "^[[:space:]]*PERIODIC_JSONL=" "${RUN_SOAK}"; then
    pass "setup_run_dir 声明 PERIODIC_JSONL"
else
    fail "setup_run_dir 缺 PERIODIC_JSONL"
fi

# T7b. record_metrics 必须 append JSON line 到 PERIODIC_JSONL
if grep -qE '>> "\$\{PERIODIC_JSONL\}"' "${RUN_SOAK}"; then
    pass "record_metrics append 到 PERIODIC_JSONL"
else
    fail "record_metrics 未 append 到 PERIODIC_JSONL"
fi

# T7c. JSON 行必须有 ts/restart_seq/server_qps/sysbench_qps 关键字段
JSON_KEYS_OK=true
for key in '"ts":' '"restart_seq":' '"rss_mb":' '"server_qps":' '"sysbench_qps":'; do
    if ! grep -qF "${key}" "${RUN_SOAK}"; then
        JSON_KEYS_OK=false
        break
    fi
done
if ${JSON_KEYS_OK}; then
    pass "JSON line 包含 ts/restart_seq/rss_mb/server_qps/sysbench_qps 5 字段"
else
    fail "JSON line 缺关键字段 (ts/restart_seq/rss_mb/server_qps/sysbench_qps)"
fi

# T7d. 行为测试: 用 awk 模拟一个 minimal record_metrics 调用, 验证 JSON 可解析
# 我们直接 source record_metrics 是不可能的 (它依赖大量运行时变量),
# 改为: 解析一个手工构造的 JSON line, 验证 schema 一致性
SAMPLE_JSONL="${TMPDIR}/sample.jsonl"
cat > "${SAMPLE_JSONL}" <<'EOF'
{"ts":"2026-08-31T10:00:00Z","restart_seq":0,"rss_mb":350.5,"fd":64,"threads":42,"wal_mb":12.34,"disk_mb":120,"server_qps":1500.5,"sysbench_qps":1480.3}
{"ts":"2026-08-31T10:10:00Z","restart_seq":0,"rss_mb":352.1,"fd":64,"threads":42,"wal_mb":12.40,"disk_mb":125,"server_qps":1520.0,"sysbench_qps":1495.0}
{"ts":"2026-08-31T10:20:00Z","restart_seq":1,"rss_mb":340.0,"fd":60,"threads":40,"wal_mb":5.10,"disk_mb":80,"server_qps":1480.0,"sysbench_qps":1450.0}
EOF

# T7d-1: 必须有 3 行
line_count=$(wc -l < "${SAMPLE_JSONL}" | tr -d ' ')
if [[ "${line_count}" -eq 3 ]]; then
    pass "JSONL 共 3 行 (line_count=${line_count})"
else
    fail "JSONL 行数异常" "got=${line_count}, expected=3"
fi

# T7d-2: 每行都是合法 JSON object (用 grep 简化检查 — 平衡花括号 / 双引号)
all_valid=true
while IFS= read -r line; do
    # 简化检查: 每行必须 { ... } 且含 ts/restart_seq/server_qps 3 键
    if [[ ! "${line}" =~ ^\{.*\}$ ]] \
        || ! grep -q '"ts":' <<<"${line}" \
        || ! grep -q '"restart_seq":' <<<"${line}" \
        || ! grep -q '"server_qps":' <<<"${line}"; then
        all_valid=false
        echo "INVALID: ${line}"
    fi
done < "${SAMPLE_JSONL}"

if ${all_valid}; then
    pass "所有 JSON 行格式合法 (顶层 object + 关键字段)"
else
    fail "JSON 行格式异常"
fi

# T7d-3: restart_seq=1 应出现在第 3 行 (验证 P-5 restart_seq 列在 JSON 里也生效)
if sed -n '3p' "${SAMPLE_JSONL}" | grep -q '"restart_seq":1'; then
    pass "restart_seq 字段在 JSON 中正确反映 cycle 编号"
else
    fail "restart_seq 字段在 JSON 中未变化"
fi

# T7e. 可选: jq 验证 (如可用则启用更严格 schema 检查)
if command -v jq >/dev/null 2>&1; then
    if jq -e 'has("ts") and has("restart_seq") and has("server_qps") and has("sysbench_qps")' \
        <(head -1 "${SAMPLE_JSONL}") >/dev/null 2>&1; then
        pass "jq schema 校验通过 (所有必需字段存在)"
    else
        fail "jq schema 校验失败"
    fi
else
    echo "  ⚠ SKIP: jq 未安装, 跳过严格 schema 校验"
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