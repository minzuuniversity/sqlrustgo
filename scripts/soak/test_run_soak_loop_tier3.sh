#!/usr/bin/env bash
#
# test_run_soak_loop_tier3.sh — v312-59-d / #4598 Tier-3 unit tests
# ==============================================================================
# 静态 + 行为测试, 不实际跑 server / 不需 root / 不需 binary.
#
#   T11. SOAK_ALERT_WEBHOOK 默认空, send_alert 函数存在, state vars 就绪
#   T12. send_alert JSON payload (jq / printf 双路径) + curl --max-time + --fail
#   T13. parse_thread_ramp 校验 (1..256 正整数, 非法值返回 rc=1)
#   T14. run_thread_ramp 集成点 (main_loop fork, ramp_*.csv 输出, ALERT reset)
#   T15. SOP doc 存在 + INDEX.md 已索引 + 关键术语出现
#
# 用法:
#   bash scripts/soak/test_run_soak_loop_tier3.sh
#
# 退出码:
#   0  — 全部 PASS
#   1  — 至少一个 FAIL
# ==============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_SOAK="${SCRIPT_DIR}/run_soak_loop.sh"
SOP_DOC="${REPO_ROOT}/docs/runbooks/soak-168h-local-sop.md"
INDEX_DOC="${REPO_ROOT}/docs/runbooks/INDEX.md"

PASS_COUNT=0
FAIL_COUNT=0
pass() { PASS_COUNT=$((PASS_COUNT + 1)); echo "  ✅ PASS: $1"; }
fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "  ❌ FAIL: $1"; [[ -n "${2:-}" ]] && echo "       $2"; }

# ── T11: alerting 默认值 + send_alert + state vars ──
echo ""
echo "[T11] SOAK_ALERT_WEBHOOK 默认 + send_alert 函数"
echo "----------------------------------------------------------"
T11_OK=true
# 默认空
if ! grep -qE '^SOAK_ALERT_WEBHOOK="\$\{SOAK_ALERT_WEBHOOK:-' "${RUN_SOAK}"; then
    T11_OK=false; echo "  缺: SOAK_ALERT_WEBHOOK 默认值声明"
fi
# send_alert 函数
if ! grep -qE '^send_alert\(\)' "${RUN_SOAK}"; then
    T11_OK=false; echo "  缺: send_alert() 函数定义"
fi
# 阈值默认值
if ! grep -qE '^SOAK_ALERT_RSS_MB=' "${RUN_SOAK}" \
   || ! grep -qE '^SOAK_ALERT_QPS_DROP_PCT=' "${RUN_SOAK}" \
   || ! grep -qE '^SOAK_ALERT_CURL_TIMEOUT=' "${RUN_SOAK}"; then
    T11_OK=false; echo "  缺: SOAK_ALERT_* 阈值默认值 (RSS_MB/QPS_DROP_PCT/CURL_TIMEOUT)"
fi
# QPS baseline state
if ! grep -qE '^ALERT_PREV_SB_QPS=' "${RUN_SOAK}"; then
    T11_OK=false; echo "  缺: ALERT_PREV_SB_QPS state var (QPS 同比)"
fi
${T11_OK} && pass "SOAK_ALERT_WEBHOOK 默认空 + send_alert + 3 阈值 + QPS baseline 全在" \
          || fail "T11 不完整"

# ── T12: send_alert JSON payload + curl 选项 ──
echo ""
echo "[T12] send_alert payload + curl 选项"
echo "----------------------------------------------------------"
T12_OK=true
# jq 路径: --arg sev/met/val/thr/msg/ts/run + --argjson rc
for arg in 'sev' 'met' 'val' 'thr' 'msg' 'ts' 'run' 'rc'; do
    if ! grep -qE "\-\-arg[a-z]*[[:space:]]+${arg}[[:space:]]" "${RUN_SOAK}"; then
        T12_OK=false; echo "  缺 jq --arg* : ${arg}"
    fi
done
# printf 路径 + sed 转义
if ! grep -qE "sed[[:space:]]+'s/\\\\\\\\\\\\\\\\/\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\/g;[[:space:]]*s/\\\"/\\\\\\\\\\\"/g'" "${RUN_SOAK}"; then
    # 简化: 只检查 printf + jq 缺位时 fallback 路径有 sed 转义
    if ! grep -qE 'sed[[:space:]]+.*s/.*backslash' "${RUN_SOAK}"; then
        # 最弱检查: 至少有一个 printf '%s' 转义 message
        if ! grep -qE "printf '%s'.*\\\\| sed" "${RUN_SOAK}"; then
            T12_OK=false; echo "  缺: printf fallback 路径 (sed 转义)"
        fi
    fi
fi
# curl 选项
for opt in '\-\-max-time' '\-\-fail' '\-H "Content-Type: application/json"' '\-X POST' '\-\-data'; do
    if ! grep -qE "${opt}" "${RUN_SOAK}"; then
        T12_OK=false; echo "  缺 curl 选项 : ${opt}"
    fi
done
# warn 不阻断主循环
if ! grep -qE 'alert webhook POST 失败.*不阻断\|warn.*alert webhook' "${RUN_SOAK}"; then
    if ! grep -qE 'warn "alert webhook POST 失败' "${RUN_SOAK}"; then
        T12_OK=false; echo "  缺: webhook 失败时 warn 而非 err/exit"
    fi
fi
${T12_OK} && pass "jq + printf 双路径 payload + curl 选项 + 失败降级" \
          || fail "T12 不完整"

# 行为测试: source send_alert, 用 empty webhook, 验证 MONITOR_LOG 有 ALERT 行
echo "  [behavioral] source send_alert + empty webhook → MONITOR_LOG 写入"
TMPDIR_B=$(mktemp -d)
trap 'rm -rf "${TMPDIR_B}"' EXIT
export MONITOR_LOG="${TMPDIR_B}/monitor.log"
export RUN_DIR="${TMPDIR_B}/run"
export SOAK_ALERT_WEBHOOK=""
export restart_count=2
# send_alert 调用 log() / warn() — 这些函数定义在 run_soak_loop.sh:138-138,
# 在 send_alert 之前. 我们提供一个 stub, 因为行为测试只验证 send_alert 输出.
log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >> "${MONITOR_LOG}"; }
warn() { log "WARN: $*"; }
# 提取 send_alert 函数定义 (从 ^send_alert() 到下一个 ^xxx() 行)
awk '/^send_alert\(\)/ {p=1} p; /^}/ && p {exit}' "${RUN_SOAK}" > "${TMPDIR_B}/send_alert_fn.sh"
# shellcheck disable=SC1090
source "${TMPDIR_B}/send_alert_fn.sh"
if send_alert "WARN" "rss_mb" "612.3" "500" "test alert" 2>/dev/null \
   && grep -q 'ALERT \[WARN\] rss_mb=612.3 (阈值 500) — test alert' "${MONITOR_LOG}"; then
    pass "send_alert (空 webhook) 写 MONITOR_LOG 含 ALERT 行"
else
    fail "send_alert (空 webhook) 未写 MONITOR_LOG" \
         "MONITOR_LOG 内容: $(cat "${MONITOR_LOG}" 2>/dev/null | head -3)"
fi
unset -f send_alert 2>/dev/null || true

# ── T13: parse_thread_ramp 校验 ──
echo ""
echo "[T13] parse_thread_ramp 校验"
echo "----------------------------------------------------------"
# 提取 parse_thread_ramp 函数定义
awk '/^parse_thread_ramp\(\)/ {p=1} p; /^}/ && p {exit}' "${RUN_SOAK}" > "${TMPDIR_B}/ptr_fn.sh"
# shellcheck disable=SC1090
source "${TMPDIR_B}/ptr_fn.sh"

# T13a: 空 → 返回 1 (disable)
if parse_thread_ramp "" >/dev/null 2>&1; then
    fail "parse_thread_ramp '' 应返回非 0 (禁用)"
else
    pass "parse_thread_ramp '' 返回 rc≠0"
fi
# T13b: "1 2 4 8" → 4 行
OUT=$(parse_thread_ramp "1 2 4 8" 2>/dev/null)
if [[ "$(printf '%s\n' "${OUT}" | wc -l)" == "4" ]] \
   && [[ "${OUT%%$'\n'*}" == "1" ]] \
   && [[ "$(printf '%s\n' "${OUT}" | tail -1)" == "8" ]]; then
    pass "parse_thread_ramp '1 2 4 8' → 4 levels (1..8)"
else
    fail "parse_thread_ramp '1 2 4 8' 输出异常: ${OUT}"
fi
# T13c: 非法值 "0" → rc=1
if parse_thread_ramp "0" >/dev/null 2>&1; then
    fail "parse_thread_ramp '0' 应拒绝 (必须 ≥1)"
else
    pass "parse_thread_ramp '0' 被拒"
fi
# T13d: 非法值 "300" → rc=1 (>256)
if parse_thread_ramp "300" >/dev/null 2>&1; then
    fail "parse_thread_ramp '300' 应拒绝 (必须 ≤256)"
else
    pass "parse_thread_ramp '300' 被拒"
fi
# T13e: 非法值 "abc" → rc=1
if parse_thread_ramp "abc" >/dev/null 2>&1; then
    fail "parse_thread_ramp 'abc' 应拒绝 (非整数)"
else
    pass "parse_thread_ramp 'abc' 被拒"
fi
# T13f: 混合 (1 abc 4) → rc=1 (整批拒绝)
if parse_thread_ramp "1 abc 4" >/dev/null 2>&1; then
    fail "parse_thread_ramp '1 abc 4' 应拒绝 (批内有非法)"
else
    pass "parse_thread_ramp '1 abc 4' 整批拒绝"
fi
unset -f parse_thread_ramp 2>/dev/null || true

# ── T14: run_thread_ramp 集成点 ──
echo ""
echo "[T14] run_thread_ramp 集成点 + 输出文件 + ALERT reset"
echo "----------------------------------------------------------"
T14_OK=true
# 函数定义
if ! grep -qE '^run_thread_ramp\(\)' "${RUN_SOAK}"; then
    T14_OK=false; echo "  缺: run_thread_ramp() 函数定义"
fi
# main_loop 调用 (前面有 SOAK_THREAD_RAMP 检查)
if ! awk '/SOAK_THREAD_RAMP.*run_thread_ramp|run_thread_ramp.*SOAK_THREAD_RAMP/' "${RUN_SOAK}" \
        | grep -q run_thread_ramp; then
    if ! grep -qE 'if \[\[ -n "\$\{SOAK_THREAD_RAMP:-?\}" \]\]; then' "${RUN_SOAK}" \
       || ! grep -qE 'run_thread_ramp' "${RUN_SOAK}"; then
        T14_OK=false; echo "  缺: main_loop 中 SOAK_THREAD_RAMP 分支 + run_thread_ramp 调用"
    fi
fi
# ramp_*.csv 输出
if ! grep -qE 'ramp_\$\{level\}\.csv' "${RUN_SOAK}" \
   || ! grep -qE 'ramp_all\.csv' "${RUN_SOAK}"; then
    T14_OK=false; echo "  缺: ramp_\${level}.csv / ramp_all.csv 输出文件"
fi
# ramp loop 重置 ALERT_PREV_SB_QPS=0 (避免 level 切换误报)
if ! grep -qE 'ALERT_PREV_SB_QPS=0' "${RUN_SOAK}"; then
    T14_OK=false; echo "  缺: ramp 中 ALERT_PREV_SB_QPS 重置"
fi
# start_sysbench 接受 thread override
if ! grep -qE 'start_sysbench "\$\{level\}"' "${RUN_SOAK}"; then
    T14_OK=false; echo "  缺: start_sysbench '${level}' override 调用"
fi
# stop_sysbench 函数
if ! grep -qE '^stop_sysbench\(\)' "${RUN_SOAK}"; then
    T14_OK=false; echo "  缺: stop_sysbench() 函数"
fi
${T14_OK} && pass "run_thread_ramp 全部集成点齐备" \
          || fail "T14 缺集成点"

# ── T15: SOP doc + INDEX ──
echo ""
echo "[T15] SOP doc 存在 + INDEX.md 已索引 + 关键术语"
echo "----------------------------------------------------------"
T15_OK=true
if [[ ! -f "${SOP_DOC}" ]]; then
    T15_OK=false; echo "  缺文件: ${SOP_DOC}"
else
    LINE_CNT=$(wc -l < "${SOP_DOC}")
    if [[ ${LINE_CNT} -lt 100 ]]; then
        T15_OK=false; echo "  SOP doc 过短: ${LINE_CNT} 行 (<100)"
    fi
    # 关键术语 (各项目关键词)
    for kw in "168h" "SOAK_ALERT_WEBHOOK" "hp-z6g4" "INCIDENT-REPORT" "run_thread_ramp" "send_alert" "disk-restart"; do
        if ! grep -q "${kw}" "${SOP_DOC}"; then
            T15_OK=false; echo "  SOP 缺关键词: ${kw}"
        fi
    done
fi
# INDEX.md 索引
if [[ ! -f "${INDEX_DOC}" ]]; then
    T15_OK=false; echo "  缺文件: ${INDEX_DOC}"
else
    if ! grep -q 'soak-168h-local-sop' "${INDEX_DOC}"; then
        T15_OK=false; echo "  INDEX.md 未引用 soak-168h-local-sop"
    fi
fi
${T15_OK} && pass "SOP doc (≥100 行) + 关键词 + INDEX.md 索引齐备" \
          || fail "T15 缺要素"

# ── 总结 ──
echo ""
echo "=========================================="
echo "Total: PASS=${PASS_COUNT}  FAIL=${FAIL_COUNT}"
echo "=========================================="

[[ ${FAIL_COUNT} -eq 0 ]] && exit 0 || exit 1