#!/usr/bin/env bash
#
# run_soak_loop.sh — SQLRustGo 综合 SOAK 测试
# ==============================================================================
# 功能:
#   1. 启动 sqlrustgo-mysql-server (nice -n 10, 日志循环 <1G)
#   2. 启动 sysbench oltp_read_write (nice -n 15, 连续写循环)
#   3. 持续监控 RSS / WAL / FD / 线程 / QPS / 磁盘用量
#   4. 每 10 分钟报告一次完整指标
#   5. 磁盘总量接近 1G 时自动循环 (重启 server + 清理数据目录)
#   6. 所有进程低优先级, 避免系统失去响应
#
# 用法:
#   bash scripts/soak/run_soak_loop.sh
#
# 环境变量:
#   SOAK_HOURS         测试运行时长 (默认 24)
#   SOAK_PORT          服务器端口 (默认 3396)
#   SOAK_TABLE_SIZE    sysbench 表行数 (默认 10000)
#   SOAK_SERVER_THR    服务器线程数 (默认 4)
#   SOAK_SB_THR        sysbench 线程数 (默认 8)
#   SOAK_RESULTS_DIR   结果输出目录
#   SOAK_NO_DETACH     设为 1 关闭自动 detach (默认: stdin 非 TTY 时自动 detach)
#   SOAK_DETACHED      内部标记, 不要手动设置
#   SOAK_DATA_DIR      server data-dir 路径 (默认 ${HOME}/sqlrustgo-soak-data-${SOAK_PORT})
#                       — v312-59-d / #4594 P-2: 防止 168h SOAK 跨重启丢失数据
#   SOAK_NICE_SERVER   server nice level (默认 10) — v312-59-d / #4594 P-3
#   SOAK_NICE_SYSBENCH sysbench nice level (默认 15) — v312-59-d / #4594 P-3
#   SOAK_WORKLOAD      sysbench workload (oltp_read_write|oltp_read_only|oltp_write_only
#                       |oltp_insert|oltp_update_index; 默认 oltp_read_write) — v312-59-d / #4596 P-6
#   SOAK_ALERT_WEBHOOK webhook URL, 阈值触发 POST JSON (空=禁用) — v312-59-d / #4598 P-11
#   SOAK_ALERT_RSS_MB  RSS 阈值 (MB), 默认 500
#   SOAK_ALERT_QPS_DROP_PCT QPS 同比下降阈值 (%), 默认 50
#   SOAK_ALERT_CURL_TIMEOUT webhook curl --max-time (秒), 默认 5
#   SOAK_THREAD_RAMP    线程斜坡模式, 空格分隔列表 e.g. "1 2 4 8 16 32" (空=禁用) — v312-59-d / #4598 P-8
#   SOAK_RAMP_DURATION_MIN 每个 thread level 跑多少分钟 (默认 5)
#
# 退出码:
#   0  — 正常完成
#   1  — 前置条件失败
#   2  — 服务器异常退出
# ==============================================================================
# 不使用 set -e, 监控脚本需容错
shopt -s nullglob
SOAK_HOURS="${SOAK_HOURS:-24}"
SOAK_PORT="${SOAK_PORT:-3396}"
SOAK_TABLE_SIZE="${SOAK_TABLE_SIZE:-10000}"
SOAK_SERVER_THR="${SOAK_SERVER_THR:-4}"
SOAK_SB_THR="${SOAK_SB_THR:-8}"
SOAK_RESULTS_DIR="${SOAK_RESULTS_DIR:-${HOME}/sqlrustgo-soak-results}"
# v312-59-d / #4594 P-2: 默认 data-dir 移到 ${HOME}, 不再用 /tmp (tmpfs 重启即失)
SOAK_DATA_DIR="${SOAK_DATA_DIR:-${HOME}/sqlrustgo-soak-data-${SOAK_PORT}}"
# v312-59-d / #4594 P-3: nice levels 可经 env 覆盖 (跨主机比较时控制干扰变量)
SOAK_NICE_SERVER="${SOAK_NICE_SERVER:-10}"
SOAK_NICE_SYSBENCH="${SOAK_NICE_SYSBENCH:-15}"
# v312-59-d / #4596 P-6: SOAK_WORKLOAD env 选 sysbench workload, 默认 read_write
# 5 个支持的 workload: oltp_read_write|oltp_read_only|oltp_write_only|oltp_insert|oltp_update_index
SOAK_WORKLOAD="${SOAK_WORKLOAD:-oltp_read_write}"
# v312-59-d / #4598 P-11: 阈值触发 webhook (空=禁用). QPS 同比下降需要 baseline, 见
# record_metrics() 中 ALERT_PREV_SB_QPS state. curl --max-time 默认 5s, 避免 SOAK 阻塞.
SOAK_ALERT_WEBHOOK="${SOAK_ALERT_WEBHOOK:-}"
SOAK_ALERT_RSS_MB="${SOAK_ALERT_RSS_MB:-500}"
SOAK_ALERT_QPS_DROP_PCT="${SOAK_ALERT_QPS_DROP_PCT:-50}"
SOAK_ALERT_CURL_TIMEOUT="${SOAK_ALERT_CURL_TIMEOUT:-5}"
# v312-59-d / #4598 P-8: 线程斜坡模式 — 用同一 workload 在 1/2/4/.../N 线程下各跑一段时间,
# 找出 sysbench QPS 的饱和点. 例如 "1 2 4 8 16 32" 6 个 level × 5min = 30min 总时长.
SOAK_THREAD_RAMP="${SOAK_THREAD_RAMP:-}"
SOAK_RAMP_DURATION_MIN="${SOAK_RAMP_DURATION_MIN:-5}"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY="${PROJECT_ROOT}/target/release/sqlrustgo-mysql-server"

TIMESTAMP=$(date '+%Y%m%d_%H%M%S')
RUN_DIR="${SOAK_RESULTS_DIR}/soak_${TIMESTAMP}"
CYCLE_DATA=""
LOG_DIR=""
SERVER_LOG=""
SYSBENCH_LOG=""
METRICS_CSV=""
HISTORY_CSV="${SOAK_RESULTS_DIR}/soak_history.csv"
MONITOR_LOG=""
PERIODIC_LOG=""

DISK_LIMIT_BYTES=$((800 * 1024 * 1024))  # 800 MB — 触发循环
DISK_WARN_BYTES=$((600 * 1024 * 1024))   # 600 MB — 警告

# ── 工具函数 ──

setup_run_dir() {
    local ts
    ts=$(date '+%Y%m%d_%H%M%S')
    RUN_DIR="${SOAK_RESULTS_DIR}/soak_${ts}"
    mkdir -p "${RUN_DIR}"
    LOG_DIR="/tmp/sqlrustgo-soak-logs-${SOAK_PORT}"
    mkdir -p "${LOG_DIR}"
    SERVER_LOG="${RUN_DIR}/server.log"
    SYSBENCH_LOG="${RUN_DIR}/sysbench.log"
    METRICS_CSV="${RUN_DIR}/metrics.csv"
    MONITOR_LOG="${RUN_DIR}/monitor.log"
    PERIODIC_LOG="${RUN_DIR}/periodic_reports.log"
    PERIODIC_JSONL="${RUN_DIR}/periodic_reports.jsonl"
    # v312-59-d / #4596 P-5: 跨 disk-restart 切分 metrics 按 cycle,
    # 把 restart_seq 拼到 metrics.csv 文件名, 旧文件保留 (便于拼接)
    METRICS_CYCLE="${RUN_DIR}/metrics.csv.0"
    echo "${RUN_DIR}" > "${SOAK_RESULTS_DIR}/latest_run.txt"
    echo "data: ${CYCLE_DATA}" >> "${SOAK_RESULTS_DIR}/latest_run.txt"
    echo "log:  ${LOG_DIR}" >> "${SOAK_RESULTS_DIR}/latest_run.txt"
}

# MONITOR_LOG 可能在 setup_run_dir() 之前被调用 (preflight),
# 所以提供 fallback 路径.
MONITOR_LOG="${MONITOR_LOG:-${SOAK_RESULTS_DIR}/preflight.log}"

# ── Self-detach (防止父 shell 被回收时内核 SIGKILL 整个进程组) ──
# 当脚本以非交互方式启动 (stdin 非 TTY, 如 AI agent / nohup / cron / systemd)
# 时, 自动用 setsid+nohup 重新 exec, 脱离父进程组 + 父会话, 这样父 shell 被
# reap 不会带走 sqlrustgo + sysbench + 本脚本。
# 交互式终端 (stdin=TTY) 保持原行为, 可 Ctrl+C 优雅中断。
# 参见 docs/releases/v3.12.0/evidence/issue-4560/INCIDENT-REPORT-2026-08-30.md §3, §6
if [[ -z "${SOAK_DETACHED:-}" && -z "${SOAK_NO_DETACH:-}" && ! -t 0 ]]; then
    mkdir -p "${SOAK_RESULTS_DIR}"
    LAUNCHER_TS=$(date '+%Y%m%d_%H%M%S')
    LAUNCHER_LOG="${SOAK_RESULTS_DIR}/launcher_${LAUNCHER_TS}.log"
    LAUNCHER_PID_FILE="${SOAK_RESULTS_DIR}/launcher_${LAUNCHER_TS}.pid"
    export SOAK_DETACHED=1
    # setsid: 新会话 + 新进程组 (process group leader) — 父 shell reap 不传播
    # nohup: 忽略 SIGHUP — 即使父会话关闭也不被杀
    # </dev/null: 断开 stdin, 防止父 shell 关闭时 EOF 触发脚本退出
    # >>LAUNCHER_LOG 2>&1: 全部输出落入 launcher 日志 (父 stdout 已关闭)
    # & + disown: 后台运行, 脱离 shell 作业控制
    if command -v setsid >/dev/null 2>&1; then
        setsid nohup bash "$0" "$@" </dev/null >>"${LAUNCHER_LOG}" 2>&1 &
    else
        # setsid 不可用时的降级: 仅 nohup + & + disown, 仍可挡住大多数 reap 场景
        nohup bash "$0" "$@" </dev/null >>"${LAUNCHER_LOG}" 2>&1 &
    fi
    LAUNCHER_PID=$!
    disown 2>/dev/null || true
    echo "${LAUNCHER_PID}" > "${LAUNCHER_PID_FILE}"
    {
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] === SOAK detached as background process ==="
        echo "  PID:           ${LAUNCHER_PID}"
        echo "  PID file:      ${LAUNCHER_PID_FILE}"
        echo "  launcher log:  ${LAUNCHER_LOG}"
        echo "  结果目录:       ${SOAK_RESULTS_DIR}/"
        echo "  latest_run:    cat ${SOAK_RESULTS_DIR}/latest_run.txt (创建后)"
        echo "  停止方式:       kill -TERM \$(cat ${LAUNCHER_PID_FILE})"
        echo "  再次启动:       bash scripts/soak/run_soak_loop.sh (会自动 detach)"
        echo "  禁用 detach:   SOAK_NO_DETACH=1 bash scripts/soak/run_soak_loop.sh"
    } >&2
    exit 0
fi

log()  { local m="[$(date '+%Y-%m-%d %H:%M:%S')] $*"; echo "${m}" >> "${MONITOR_LOG}"; echo "${m}"; }
warn() { log "WARN: $*"; }
err()  { log "ERROR: $*"; }

cleanup() {
    log "=== 清理 ==="
    if [[ -f "${RUN_DIR}/sysbench.pid" ]]; then
        kill -TERM "$(cat "${RUN_DIR}/sysbench.pid")" 2>/dev/null || true
        sleep 2
    fi
    if [[ -f "${RUN_DIR}/server.pid" ]]; then
        kill -TERM "$(cat "${RUN_DIR}/server.pid")" 2>/dev/null || true
        sleep 2
    fi
    log "清理完成"
    echo ""
    echo "=== SOAK 已停止 ==="
    echo "结果: ${RUN_DIR}"
}

pid_alive() { [[ -n "${1:-}" ]] && kill -0 "$1" 2>/dev/null; }

total_disk_bytes() {
    local total=0
    if [[ -n "${CYCLE_DATA}" && -d "${CYCLE_DATA}" ]]; then
        total=$((total + $(du -sb "${CYCLE_DATA}" 2>/dev/null | cut -f1 || echo 0)))
    fi
    if [[ -d "${LOG_DIR}" ]]; then
        total=$((total + $(du -sb "${LOG_DIR}" 2>/dev/null | cut -f1 || echo 0)))
    fi
    echo "${total}"
}

wal_size_bytes() {
    local path=""
    for cand in "${CYCLE_DATA}/sqlrustgo.wal" "${CYCLE_DATA}/wal"; do
        [[ -f "${cand}" ]] && { path="${cand}"; break; }
    done
    if [[ -n "${path}" ]]; then
        stat -f%z "${path}" 2>/dev/null || stat -c%s "${path}" 2>/dev/null || echo 0
    else
        echo 0
    fi
}

server_rss_kb() {
    local pid="${1:-}"
    [[ -z "${pid}" ]] && { echo 0; return; }
    if [[ -f "/proc/${pid}/status" ]]; then
        awk '/VmRSS:/ {gsub(/[^0-9]/,"",$2); print $2}' "/proc/${pid}/status" 2>/dev/null || echo 0
    else
        ps -o rss= -p "${pid}" 2>/dev/null | tr -d ' ' || echo 0
    fi
}

server_fd_count() {
    local pid="${1:-}"
    [[ -z "${pid}" ]] && { echo 0; return; }
    if [[ -d "/proc/${pid}/fd" ]]; then
        ls -1 "/proc/${pid}/fd" 2>/dev/null | wc -l | tr -d ' '
    else
        lsof -p "${pid}" 2>/dev/null | wc -l | tr -d ' '
    fi
}

server_thread_count() {
    local pid="${1:-}"
    [[ -z "${pid}" ]] && { echo 0; return; }
    if [[ -d "/proc/${pid}/task" ]]; then
        ls -1 "/proc/${pid}/task" 2>/dev/null | wc -l | tr -d ' '
    elif command -v ps >/dev/null 2>&1; then
        ps -M -p "${pid}" 2>/dev/null | tail -n +2 | wc -l | tr -d ' '
    else
        echo 0
    fi
}

# v312-59-d / #4594 P-9: server 实际日志格式是 RESOURCE_MONITOR pid=... total_q={N},
# 原 regex `qps=[0-9]+\.[0-9]+` 在 7h28m run 中 0 次匹配 (45/45 报告 ServerQPS 为空).
# 修复: 取最近两条 RESOURCE_MONITOR 行的 total_q, 求差分除以时间间隔得 QPS.
# State vars (script-level, 初始化在 setup_run_dir 后):
SOAK_PREV_TOTAL_Q=0
SOAK_PREV_QPS_TS=0

server_qps() {
    local last_line cur_total_q cur_ts delta_q delta_t
    last_line=$(grep -a "RESOURCE_MONITOR" "${SERVER_LOG}" 2>/dev/null | tail -1)
    if [[ -z "${last_line}" ]]; then
        echo "0"
        return
    fi
    cur_total_q=$(echo "${last_line}" | grep -oE 'total_q=[0-9]+' | cut -d= -f2 || echo 0)
    cur_ts=$(date +%s)
    if [[ ${SOAK_PREV_QPS_TS} -eq 0 || ${SOAK_PREV_TOTAL_Q} -eq 0 ]]; then
        # 首次采样: 初始化 state, 返回 0 (无前值可比)
        SOAK_PREV_TOTAL_Q=${cur_total_q}
        SOAK_PREV_QPS_TS=${cur_ts}
        echo "0"
        return
    fi
    delta_q=$((cur_total_q - SOAK_PREV_TOTAL_Q))
    delta_t=$((cur_ts - SOAK_PREV_QPS_TS))
    # 更新 state
    SOAK_PREV_TOTAL_Q=${cur_total_q}
    SOAK_PREV_QPS_TS=${cur_ts}
    if [[ ${delta_t} -le 0 || ${delta_q} -lt 0 ]]; then
        # counter reset / 时间倒退 — 返回 0
        echo "0"
        return
    fi
    awk "BEGIN {printf \"%.2f\", ${delta_q} / ${delta_t}}"
}

sysbench_qps() {
    grep -aE "queries per second:" "${SYSBENCH_LOG}" 2>/dev/null | tail -1 | \
        grep -oE '[0-9]+\.[0-9]+' || echo "0"
}

# ── 前置检查 ──

preflight() {
    local fail=0
    if [[ ! -x "${BINARY}" ]]; then
        err "二进制不存在: ${BINARY}"
        err "请先: cargo build --release -p sqlrustgo-mysql-server"
        fail=1
    fi
    if ! command -v sysbench >/dev/null 2>&1; then
        err "sysbench 未安装"
        fail=1
    fi
    if command -v lsof >/dev/null 2>&1; then
        if lsof -i ":${SOAK_PORT}" -sTCP:LISTEN 2>/dev/null | grep -q .; then
            err "端口 ${SOAK_PORT} 已被占用"
            fail=1
        fi
    fi
    # v312-59-d / #4596 P-6: SOAK_WORKLOAD 必须在白名单内, 早失败避免 start_sysbench 才暴露
    if ! load_workload_args "${SOAK_WORKLOAD}" >/dev/null 2>&1; then
        err "SOAK_WORKLOAD='${SOAK_WORKLOAD}' 不在白名单"
        err "支持: oltp_read_write|oltp_read_only|oltp_write_only|oltp_insert|oltp_update_index"
        fail=1
    fi
    if [[ $fail -ne 0 ]]; then exit 1; fi
    log "前置检查通过"
    log "  二进制: ${BINARY}"
    log "  sysbench: $(sysbench --version 2>&1 | head -1)"
    log "  workload: ${SOAK_WORKLOAD}"
}

# ── 服务器管理 ──

start_server() {
    # v312-59-d / #4594 P-2: 默认 data-dir 移到 ${HOME}, 不再用 /tmp (tmpfs 重启即失)
    CYCLE_DATA="${SOAK_DATA_DIR}"
    # 检测 tmpfs 并警告 — 跨主机 SOAK 必须显式 SOAK_DATA_DIR= 才能用 /tmp
    if [[ "$(stat -f -c %T "${CYCLE_DATA}" 2>/dev/null || echo unknown)" == "tmpfs" ]]; then
        warn "data-dir ${CYCLE_DATA} 在 tmpfs — 168h SOAK 跨重启即失!"
        warn "  显式覆盖: SOAK_DATA_DIR=/path/to/disk bash scripts/soak/run_soak_loop.sh"
    fi
    rm -rf "${CYCLE_DATA}"
    mkdir -p "${CYCLE_DATA}" "${LOG_DIR}"

    log "启动服务器 (port=${SOAK_PORT})"
    log "  data-dir: ${CYCLE_DATA}"
    log "  log-dir:  ${LOG_DIR} (server stdout → ${SERVER_LOG}, internal logs go through tracing-subscriber)"
    log "  nice: -n ${SOAK_NICE_SERVER}"
    log "  [v312-59-d / #4499 patch] dropped --log-dir / --tls, --monitor-port → --metrics-port, added --wal-sync batch:10000"

    # v312-59-d / #4594 P-3: nice level 可经 SOAK_NICE_SERVER 覆盖
    nice -n "${SOAK_NICE_SERVER}" \
        "${BINARY}" serve \
        --host 127.0.0.1 \
        --port "${SOAK_PORT}" \
        --data-dir "${CYCLE_DATA}" \
        --server-threads "${SOAK_SERVER_THR}" \
        --max-connections 200 \
        --metrics-port 9300 \
        --log-level info \
        --storage file \
        --wal-sync batch:10000 \
        > "${SERVER_LOG}" 2>&1 &
    local pid=$!
    echo "${pid}" > "${RUN_DIR}/server.pid"

    for i in $(seq 1 30); do
        if ! pid_alive "${pid}"; then
            err "服务器启动失败"
            tail -20 "${SERVER_LOG}" >&2
            return 1
        fi
        if lsof -i ":${SOAK_PORT}" -sTCP:LISTEN 2>/dev/null | grep -q "${pid}"; then
            log "  服务器就绪 (${i}s)"
            return 0
        fi
        sleep 1
    done
    err "服务器 30s 未就绪"
    tail -20 "${SERVER_LOG}" >&2
    return 1
}

stop_current_server() {
    local pid=""
    [[ -f "${RUN_DIR}/server.pid" ]] && pid=$(cat "${RUN_DIR}/server.pid" 2>/dev/null || echo "")
    if pid_alive "${pid}"; then
        log "停止服务器 PID=${pid}..."
        kill -TERM "${pid}" 2>/dev/null || true
        for i in 1 2 3 4 5; do
            ! pid_alive "${pid}" && { log "  已停止"; break; }
            sleep 1
        done
        if pid_alive "${pid}"; then
            kill -KILL "${pid}" 2>/dev/null || true
            sleep 1
        fi
    fi
    rm -rf "${CYCLE_DATA}"
    rm -f "${LOG_DIR:?}/"*
}

# ── Sysbench ──

# v312-59-d / #4596 P-6: SOAK_WORKLOAD → sysbench --test=... 映射
# 支持: oltp_read_write (默认), oltp_read_only, oltp_write_only,
#       oltp_insert, oltp_update_index
# 输出: 单行 sysbench --test=X... 参数, 调用者负责前缀
# 用法: load_workload_args "$SOAK_WORKLOAD"
load_workload_args() {
    local wl="${1:-oltp_read_write}"
    case "${wl}" in
        oltp_read_write)   printf '%s' "--test=oltp_read_write" ;;
        oltp_read_only)    printf '%s' "--test=oltp_read_only" ;;
        oltp_write_only)   printf '%s' "--test=oltp_write_only" ;;
        oltp_insert)       printf '%s' "--test=oltp_insert" ;;
        oltp_update_index) printf '%s' "--test=oltp_update_index" ;;
        *)
            err "未知 SOAK_WORKLOAD='${wl}'"
            err "支持: oltp_read_write|oltp_read_only|oltp_write_only|oltp_insert|oltp_update_index"
            return 1
            ;;
    esac
}

start_sysbench() {
    # v312-59-d / #4598 P-8: 接受可选的线程数覆盖 (线程斜坡模式需要).
    # 默认用 SOAK_SB_THR, 这样既有 main_loop 调用方式不变.
    local threads_override="${1:-}"
    local sb_threads="${threads_override:-${SOAK_SB_THR}}"
    log "=== sysbench prepare (workload=${SOAK_WORKLOAD}) ==="
    local wl_args
    if ! wl_args=$(load_workload_args "${SOAK_WORKLOAD}"); then
        return 1
    fi
    # shellcheck disable=SC2086
    sysbench ${wl_args} \
        --db-driver=mysql \
        --mysql-host=127.0.0.1 \
        --mysql-port="${SOAK_PORT}" \
        --mysql-user=root \
        --mysql-password="" \
        --mysql-db=sbtest \
        --table-size="${SOAK_TABLE_SIZE}" \
        --tables=1 \
        prepare >> "${SYSBENCH_LOG}" 2>&1
    local rc=$?
    if [[ ${rc} -ne 0 ]]; then
        warn "sysbench prepare 退出码=${rc}"
        tail -5 "${SYSBENCH_LOG}"
    fi
    log "  prepare 完成"

    sleep 2

    log "启动 sysbench run (${sb_threads} threads, workload=${SOAK_WORKLOAD}, nice -n ${SOAK_NICE_SYSBENCH})"
    # v312-59-d / #4594 P-4: sysbench --time 必须 ≥ SOAK_HOURS, 否则 SOAK 结束后 sysbench 孤立运行
    # +1h buffer 确保 sysbench 比 main_loop 后退出, 避免 SOAK 中途 orphan 干扰
    local sysbench_seconds=$(( (SOAK_HOURS + 1) * 3600 ))
    # v312-59-d / #4594 P-3: nice level 可经 SOAK_NICE_SYSBENCH 覆盖
    # shellcheck disable=SC2086
    nice -n "${SOAK_NICE_SYSBENCH}" \
        sysbench ${wl_args} \
        --db-driver=mysql \
        --db-ps-mode=disable \
        --mysql-host=127.0.0.1 \
        --mysql-port="${SOAK_PORT}" \
        --mysql-user=root \
        --mysql-password="" \
        --mysql-db=sbtest \
        --table-size="${SOAK_TABLE_SIZE}" \
        --tables=1 \
        --threads="${sb_threads}" \
        --time="${sysbench_seconds}" \
        --report-interval=10 \
        run >> "${SYSBENCH_LOG}" 2>&1 &
    local pid=$!
    echo "${pid}" > "${RUN_DIR}/sysbench.pid"
    log "  sysbench PID: ${pid} (nice -n 15)"
    sleep 2
    if ! pid_alive "${pid}"; then
        err "sysbench 启动后立即退出"
        tail -10 "${SYSBENCH_LOG}"
        return 1
    fi
    return 0
}

stop_sysbench() {
    local pid=""
    [[ -f "${RUN_DIR}/sysbench.pid" ]] && pid=$(cat "${RUN_DIR}/sysbench.pid" 2>/dev/null || echo "")
    if pid_alive "${pid}"; then
        log "停止 sysbench PID=${pid}"
        kill -TERM "${pid}" 2>/dev/null || true
        for _ in 1 2 3 4 5; do
            ! pid_alive "${pid}" && { log "  sysbench 已停止"; break; }
            sleep 1
        done
        if pid_alive "${pid}"; then
            kill -KILL "${pid}" 2>/dev/null || true
            sleep 1
        fi
    fi
    rm -f "${RUN_DIR}/sysbench.pid"
}

# v312-59-d / #4598 P-8: 解析 SOAK_THREAD_RAMP (空格分隔 e.g. "1 2 4 8 16 32").
# 校验: 每个值是正整数 1..256, 失败返回 rc=1 (调用方决定 warn 还是 err).
parse_thread_ramp() {
    local raw="${1:-}"
    if [[ -z "${raw}" ]]; then
        return 1
    fi
    local valid=()
    for tok in ${raw}; do
        if ! [[ "${tok}" =~ ^[1-9][0-9]*$ ]] || [[ "${tok}" -gt 256 ]]; then
            err "SOAK_THREAD_RAMP 包含非法值: '${tok}' (必须是 1-256 的正整数)"
            return 1
        fi
        valid+=("${tok}")
    done
    if [[ ${#valid[@]} -eq 0 ]]; then
        return 1
    fi
    printf '%s\n' "${valid[@]}"
    return 0
}

# v312-59-d / #4598 P-8: 线程斜坡执行器 — 对每个 level:
#   1. 停当前 sysbench
#   2. 用 N 线程重启 sysbench
#   3. 跑 SOAK_RAMP_DURATION_MIN 分钟, 每分钟采样 RSS/QPS
#   4. 把该 level 的 metrics 追加到 ramp_${level}.csv + ramp_all.csv
#   5. 完成后停 sysbench, 进入下一 level
# 返回 rc=0 表示所有 level 完成.
run_thread_ramp() {
    local levels
    if ! levels=$(parse_thread_ramp "${SOAK_THREAD_RAMP}"); then
        err "SOAK_THREAD_RAMP 解析失败: '${SOAK_THREAD_RAMP}'"
        return 1
    fi
    local level_count
    level_count=$(printf '%s\n' "${levels}" | wc -l | tr -d ' ')
    log "=== 线程斜坡模式: ${level_count} 个 level (${SOAK_RAMP_DURATION_MIN}min/level) ==="
    log "  levels: $(printf '%s ' ${levels})"

    local ramp_all="${RUN_DIR}/ramp_all.csv"
    echo "level,threads,ts,rss_mb,server_qps,sysbench_qps" > "${ramp_all}"

    local level_idx=0
    local level pid server_pid
    while IFS= read -r level; do
        level_idx=$((level_idx + 1))
        log "── ramp level ${level_idx}/${level_count}: threads=${level} ──"

        stop_sysbench

        # 每次重启 server 让 baseline 干净 (避免 buffer pool warm 干扰)
        stop_current_server
        setup_run_dir
        if ! start_server; then
            err "ramp level ${level}: server 启动失败"
            return 1
        fi
        server_pid=$(cat "${RUN_DIR}/server.pid")

        # sysbench 在 SOAK_HOURS 小时内跑 (P-4 buffer), 我们只跑 RAMP_DURATION_MIN
        # 然后主动 stop_sysbench, 远早于 sysbench_seconds, 不会 orphan.
        if ! start_sysbench "${level}"; then
            err "ramp level ${level}: sysbench 启动失败"
            return 1
        fi
        # 重置 QPS baseline 避免 level 切换时 QPS 同比告警 (P-11 baseline reset)
        ALERT_PREV_SB_QPS=0

        local ramp_csv="${RUN_DIR}/ramp_${level}.csv"
        echo "ts,elapsed_s,rss_mb,server_qps,sysbench_qps" > "${ramp_csv}"

        local total_ramp_seconds=$((SOAK_RAMP_DURATION_MIN * 60))
        local ramp_start
        ramp_start=$(date +%s)
        local ramp_deadline=$((ramp_start + total_ramp_seconds))

        while [[ $(date +%s) -lt ${ramp_deadline} ]]; do
            sleep 60
            local now
            now=$(date +%s)
            local elapsed=$((now - ramp_start))
            local cur_pid
            cur_pid=$(cat "${RUN_DIR}/server.pid" 2>/dev/null || echo "")
            local rss_mb
            rss_mb=$(awk "BEGIN {printf \"%.1f\", $(server_rss_kb "${cur_pid}")/1024}")
            local sv_qps
            sv_qps=$(server_qps)
            local sb_qps
            sb_qps=$(sysbench_qps)
            echo "${now},${elapsed},${rss_mb},${sv_qps},${sb_qps}" >> "${ramp_csv}"
            echo "${level},${level},${now},${rss_mb},${sv_qps},${sb_qps}" >> "${ramp_all}"
            log "  ramp[${level}] t=${elapsed}s RSS=${rss_mb}MB sv_qps=${sv_qps} sb_qps=${sb_qps}"
        done
        log "  ramp[${level}] 完成 (${SOAK_RAMP_DURATION_MIN}min)"
    done <<< "${levels}"

    stop_sysbench
    log "=== 线程斜坡完成 ==="
    log "  per-level CSV: ${RUN_DIR}/ramp_<threads>.csv"
    log "  aggregated:    ${ramp_all}"
    return 0
}

# ── Alerting (v312-59-d / #4598 P-11) ──

# State: QPS baseline 用于检测同比下降, errors 计数用于检测新错误
ALERT_PREV_SB_QPS=0
ALERT_PREV_ERR_COUNT=0

# send_alert <severity> <metric> <value> <threshold> <message>
#   severity: INFO|WARN|CRITICAL
#   metric:   指标名 (rss_mb / qps_drop_pct / new_errors)
#   value:    当前数值
#   threshold: 阈值
#   message:  人类可读描述
#
# 行为:
#   1. 总是写一行 ALERT 标记到 MONITOR_LOG (事后 grep 友好)
#   2. 若 SOAK_ALERT_WEBHOOK 非空, POST 一行 JSON (Slack/Email 适配见下)
#   3. webhook 调用受 SOAK_ALERT_CURL_TIMEOUT (默认 5s) 限制, 失败不阻塞 SOAK
#
# JSON payload schema (Slack-compatible incoming webhook 简化):
#   {"severity":"WARN","metric":"rss_mb","value":612.3,"threshold":500,
#    "message":"RSS=612.3MB > 500MB","ts":"2026-09-02T21:30:14Z",
#    "soak_run":"soak_20260902_212823","restart_count":3}
send_alert() {
    local severity="${1:-INFO}"
    local metric="${2:-unknown}"
    local value="${3:-0}"
    local threshold="${4:-0}"
    local message="${5:-}"
    local iso_ts
    iso_ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    local run_tag
    run_tag=$(basename "${RUN_DIR:-unknown}")

    # 1. 总是写 monitor log (事后 grep ALERT 即可定位所有告警)
    log "ALERT [${severity}] ${metric}=${value} (阈值 ${threshold}) — ${message}"

    # 2. webhook 失败不影响 SOAK 主循环
    if [[ -z "${SOAK_ALERT_WEBHOOK:-}" ]]; then
        return 0
    fi
    if ! command -v curl >/dev/null 2>&1; then
        warn "SOAK_ALERT_WEBHOOK 已设但 curl 不可用, 跳过"
        return 0
    fi

    # 用 jq 构造 JSON (若 jq 不在, 退到 printf 转义版本)
    local payload
    if command -v jq >/dev/null 2>&1; then
        payload=$(jq -c -n \
            --arg sev "${severity}" \
            --arg met "${metric}" \
            --arg val "${value}" \
            --arg thr "${threshold}" \
            --arg msg "${message}" \
            --arg ts  "${iso_ts}" \
            --arg run "${run_tag}" \
            --argjson rc "${restart_count:-0}" \
            '{severity:$sev,metric:$met,value:$val,threshold:$thr,
              message:$msg,ts:$ts,soak_run:$run,restart_count:$rc}')
    else
        # printf 转义版本: 用 sed 转义双引号 + 反斜杠 (足够覆盖本场景 ASCII-only)
        local esc_msg
        esc_msg=$(printf '%s' "${message}" | sed 's/\\/\\\\/g; s/"/\\"/g')
        payload=$(printf '{"severity":"%s","metric":"%s","value":"%s","threshold":"%s","message":"%s","ts":"%s","soak_run":"%s","restart_count":%d}' \
            "${severity}" "${metric}" "${value}" "${threshold}" \
            "${esc_msg}" "${iso_ts}" "${run_tag}" "${restart_count:-0}")
    fi

    # curl 失败仅 warn, 不阻断主循环
    if ! curl --silent --show-error --fail \
            --max-time "${SOAK_ALERT_CURL_TIMEOUT}" \
            -H "Content-Type: application/json" \
            -X POST \
            --data "${payload}" \
            "${SOAK_ALERT_WEBHOOK}" \
            >> "${MONITOR_LOG}" 2>&1; then
        warn "alert webhook POST 失败 (severity=${severity}, metric=${metric}) — 见 ${MONITOR_LOG}"
        return 1
    fi
    return 0
}

# ── 10分钟指标报告 ──

# v312-59-d / #4596 P-10: 同步输出 machine-parseable JSON line 到 PERIODIC_JSONL
# v312-59-d / #4596 P-5:  在 CSV / JSON 中嵌入 restart_seq 列, 跨 cycle 可拼接
record_metrics() {
    local pid="${1:-}"
    local elapsed="${2:-0}"
    local now
    now=$(date +%s)

    local rss_kb=0 fd=0 threads=0 wal_b=0 disk_b=0
    if pid_alive "${pid}"; then
        rss_kb=$(server_rss_kb "${pid}")
        fd=$(server_fd_count "${pid}")
        threads=$(server_thread_count "${pid}")
    fi
    wal_b=$(wal_size_bytes)
    disk_b=$(total_disk_bytes)

    local sv_qps sb_qps
    sv_qps=$(server_qps)
    sb_qps=$(sysbench_qps)

    # P-5: 写入按 cycle 切分的 METRICS_CYCLE 文件 + 历史汇总 (无 restart_seq)
    echo "${now},${elapsed},${restart_count:-0},${rss_kb},${fd},${threads},${wal_b},${disk_b},${sv_qps},${sb_qps}" >> "${METRICS_CYCLE}"
    echo "${now},${elapsed},${restart_count:-0},${rss_kb},${fd},${threads},${wal_b},${disk_b},${sv_qps},${sb_qps}" >> "${HISTORY_CSV}"

    local rss_mb wal_mb disk_mb
    rss_mb=$(awk "BEGIN {printf \"%.1f\", ${rss_kb}/1024}")
    wal_mb=$(awk "BEGIN {printf \"%.2f\", ${wal_b}/1048576}")
    disk_mb=$(awk "BEGIN {printf \"%.0f\", ${disk_b}/1048576}")

    local builtin_line
    builtin_line=$(grep -a "RESOURCE_MONITOR" "${SERVER_LOG}" 2>/dev/null | tail -1)
    local sb_line
    sb_line=$(grep -a "thds:" "${SYSBENCH_LOG}" 2>/dev/null | tail -1)

    # ISO-8601 timestamp 用于 JSON (jq 友好)
    local iso_ts
    iso_ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    # P-10: 同时输出 human-readable + machine-parseable JSON line
    cat >> "${PERIODIC_LOG}" <<REPORT

$(date '+%Y-%m-%d %H:%M:%S') === SOAK 10min Report (cycle=${restart_count:-0}) ===
RSS:       ${rss_mb} MB
FD:        ${fd}
Threads:   ${threads}
WAL:       ${wal_mb} MB
Disk:      ${disk_mb} MB / 800 MB limit
ServerQPS: ${sv_qps}
SysbenchQPS: ${sb_qps}
${builtin_line}
${sb_line}
REPORT

    # P-10: 一行一个 JSON object — jq -c '. | {ts,rss_mb,sysbench_qps}' 即可消费
    # 字段对齐 schema: ts,restart_seq,rss_mb,fd,threads,wal_mb,disk_mb,server_qps,sysbench_qps
    printf '{"ts":"%s","restart_seq":%d,"rss_mb":%s,"fd":%d,"threads":%d,"wal_mb":%s,"disk_mb":%d,"server_qps":%s,"sysbench_qps":%s}\n' \
        "${iso_ts}" \
        "${restart_count:-0}" \
        "${rss_mb}" \
        "${fd}" \
        "${threads}" \
        "${wal_mb}" \
        "${disk_mb}" \
        "${sv_qps}" \
        "${sb_qps}" \
        >> "${PERIODIC_JSONL}"

    # v312-59-d / #4598 P-11: 阈值告警 (RSS > SOAK_ALERT_RSS_MB, QPS drop > 50%)
    # ERR_PER_S 暂未单独跟踪, 由 SOAK 主循环的 disk-restart / pid-death 分支承担.
    # QPS 同比: 第一个 sample 不触发 (无 baseline), ALERT_PREV_SB_QPS 初始 0.
    if [[ -n "${SOAK_ALERT_RSS_MB:-}" ]] \
        && awk "BEGIN {exit !(${rss_mb} > ${SOAK_ALERT_RSS_MB})}"; then
        send_alert "WARN" "rss_mb" "${rss_mb}" "${SOAK_ALERT_RSS_MB}" \
            "RSS=${rss_mb}MB 超过阈值 ${SOAK_ALERT_RSS_MB}MB"
    fi
    if [[ "${ALERT_PREV_SB_QPS}" -gt 0 ]] \
        && awk "BEGIN {exit !(${ALERT_PREV_SB_QPS} > 0 && ${sb_qps} > 0)}"; then
        local drop_pct
        drop_pct=$(awk "BEGIN {
            if (${ALERT_PREV_SB_QPS} <= 0 || ${sb_qps} <= 0) {print \"-1\"; exit}
            printf \"%.1f\", (1.0 - ${sb_qps} / ${ALERT_PREV_SB_QPS}) * 100.0
        }")
        if awk "BEGIN {exit !(${drop_pct} >= ${SOAK_ALERT_QPS_DROP_PCT})}"; then
            send_alert "WARN" "qps_drop_pct" "${drop_pct}" "${SOAK_ALERT_QPS_DROP_PCT}" \
                "sysbench QPS=${sb_qps} 较上次 ${ALERT_PREV_SB_QPS} 下降 ${drop_pct}% (≥ ${SOAK_ALERT_QPS_DROP_PCT}%)"
        fi
    fi
    # 仅在 sb_qps 是有效数字时更新 baseline (避免 sysbench 未启动时 0 污染)
    if awk "BEGIN {exit !(${sb_qps} > 0)}"; then
        ALERT_PREV_SB_QPS="${sb_qps}"
    fi

    echo ""
    echo "=== SOAK 10min Report (cycle=${restart_count:-0}) ==="
    echo "  RSS: ${rss_mb} MB | FD: ${fd} | Threads: ${threads}"
    echo "  WAL: ${wal_mb} MB | Disk: ${disk_mb}/${disk_limit_mb:-800} MB"
    echo "  Server QPS: ${sv_qps} | Sysbench QPS: ${sb_qps}"
    echo "  [server] ${builtin_line}"
    echo "  [sysbench] ${sb_line}"
    echo "=========================="
    echo ""
}

# ── 主循环 ──

main_loop() {
    local total_seconds=$((SOAK_HOURS * 3600))
    local deadline=$(($(date +%s) + total_seconds))

    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║  SQLRustGo SOAK Test                                       ║"
    echo "║  Duration: ${SOAK_HOURS}h | Port: ${SOAK_PORT}                    ║"
    echo "║  Workload: ${SOAK_WORKLOAD}                                    ║"
    echo "║  Server: nice -n 10 | Sysbench: nice -n 15                 ║"
    echo "║  Stop: kill -TERM \$(cat ${RUN_DIR}/server.pid) or Ctrl+C   ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""
    echo "ts,elapsed_s,restart_seq,rss_kb,fd,threads,wal_bytes,disk_bytes,server_qps,sysbench_qps" > "${METRICS_CYCLE}"
    # HISTORY_CSV 仅在首次初始化时写 header (后续 append)
    if [[ ! -f "${HISTORY_CSV}" ]]; then
        echo "ts,elapsed_s,restart_seq,rss_kb,fd,threads,wal_bytes,disk_bytes,server_qps,sysbench_qps" > "${HISTORY_CSV}"
    fi

    if ! start_server; then
        err "服务器启动失败"
        exit 2
    fi
    local server_pid
    server_pid=$(cat "${RUN_DIR}/server.pid")

    if ! start_sysbench; then
        err "sysbench 启动失败"
        exit 2
    fi

    # v312-59-d / #4598 P-8: 线程斜坡模式 — 跳过正常的 N 小时循环,
    # 改用 run_thread_ramp() 串行执行每个 thread level.
    # ramp 完成通过 return 0 回到 main() 的 cleanup 路径 (trap EXIT).
    if [[ -n "${SOAK_THREAD_RAMP:-}" ]]; then
        log "=== SOAK 线程斜坡模式 (跳过主循环) ==="
        if ! run_thread_ramp; then
            err "线程斜坡失败, 终止"
            exit 2
        fi
        return 0
    fi

    log "=== SOAK 开始 ==="
    echo ""

    local sample=0
    local restart_count=0

    while [[ $(date +%s) -lt ${deadline} ]]; do
        server_pid=$(cat "${RUN_DIR}/server.pid" 2>/dev/null || echo "")

        # ── 检查 server 存活 ──
        if ! pid_alive "${server_pid}"; then
            # 显式记录 PID 死亡事件, 便于事后取证 (参考 INCIDENT-REPORT-2026-08-30.md §3)
            log "=== UNEXPECTED PID DEATH: server ==="
            log "  server_pid=${server_pid} (was alive at previous iteration)"
            log "  sysbench_pid=$(cat "${RUN_DIR}/sysbench.pid" 2>/dev/null || echo 'missing')"
            log "  last_server_line: $(tail -1 "${SERVER_LOG}" 2>/dev/null | head -c 240)"
            log "  last_sysbench_line: $(tail -1 "${SYSBENCH_LOG}" 2>/dev/null | head -c 240)"
            log "  检测时间: $(date '+%Y-%m-%d %H:%M:%S'), restart_count=${restart_count}"
            warn "服务器进程消失! 重启..."
            tail -3 "${SERVER_LOG}" >&2
            restart_count=$((restart_count + 1))

            stop_current_server 2>/dev/null || true
            rm -f "${RUN_DIR}/sysbench.pid"

            setup_run_dir
            if ! start_server; then
                err "重启失败, 终止"
                exit 2
            fi
            server_pid=$(cat "${RUN_DIR}/server.pid")
            if ! start_sysbench; then
                warn "sysbench 重启失败"
            fi
            sample=0
            continue
        fi

        # ── 检查磁盘用量 ──
        local disk_b
        disk_b=$(total_disk_bytes)
        if [[ ${disk_b} -gt ${DISK_LIMIT_BYTES} ]]; then
            warn "磁盘 ${disk_b}B > ${DISK_LIMIT_BYTES}B, 触发循环重启"
            restart_count=$((restart_count + 1))

            local elapsed=$(( $(date +%s) - (deadline - total_seconds) ))
            record_metrics "${server_pid}" "${elapsed}"

            if [[ -f "${RUN_DIR}/sysbench.pid" ]]; then
                kill -TERM "$(cat "${RUN_DIR}/sysbench.pid")" 2>/dev/null || true
                sleep 3
            fi

            stop_current_server

            # v312-59-d / #4596 P-5: disk-restart 边界标记
            # setup_run_dir 重建 RUN_DIR (新 timestamp), 配 setup_run_dir 的 METRICS_CYCLE 初始化
            # 指向新目录的 metrics.csv.0, 旧 cycle 文件保留. 这里只补 header.
            setup_run_dir
            echo "ts,elapsed_s,restart_seq,rss_kb,fd,threads,wal_bytes,disk_bytes,server_qps,sysbench_qps" > "${METRICS_CYCLE}"
            # HISTORY_CSV 仅在首次初始化时写 header (后续 append), 跨 RUN_DIR 共享
            if [[ ! -f "${HISTORY_CSV}" ]]; then
                echo "ts,elapsed_s,restart_seq,rss_kb,fd,threads,wal_bytes,disk_bytes,server_qps,sysbench_qps" > "${HISTORY_CSV}"
            fi

            if ! start_server; then
                err "重启失败, 终止"
                exit 2
            fi
            server_pid=$(cat "${RUN_DIR}/server.pid")
            sleep 2
            if ! start_sysbench; then
                warn "sysbench 重启失败"
            fi
            sample=0
            log "循环重启完成 (restart_count=${restart_count})"
            continue
        fi

        # ── 每 60s 进度 + 每 10 min 报告 ──
        local elapsed=$(( $(date +%s) - (deadline - total_seconds) ))
        server_pid=$(cat "${RUN_DIR}/server.pid" 2>/dev/null || echo "")

        local rss_kb fd threads wal_b
        rss_kb=$(server_rss_kb "${server_pid}")
        fd=$(server_fd_count "${server_pid}")
        threads=$(server_thread_count "${server_pid}")
        wal_b=$(wal_size_bytes)
        disk_b=$(total_disk_bytes)

        local rss_mb wal_mb disk_mb
        rss_mb=$(awk "BEGIN {printf \"%.1f\", ${rss_kb}/1024}")
        wal_mb=$(awk "BEGIN {printf \"%.2f\", ${wal_b}/1048576}")
        disk_mb=$(awk "BEGIN {printf \"%.0f\", ${disk_b}/1048576}")
        local disk_limit_mb=$((DISK_LIMIT_BYTES / 1048576))

        if [[ $((sample % 10)) -eq 0 ]]; then
            record_metrics "${server_pid}" "${elapsed}"
        fi

        local remain=$((deadline - $(date +%s)))
        local remain_hr=$((remain / 3600))
        local remain_min=$(((remain % 3600) / 60))
        printf "\r  [%4ds elapsed, ${remain_hr}h%02dm remain] RSS=${rss_mb}MB FD=${fd} THR=${threads} WAL=${wal_mb}MB DISK=${disk_mb}/${disk_limit_mb}MB restart=${restart_count}  " "${elapsed}"

        sample=$((sample + 1))
        sleep 60
    done

    echo ""
    log "=== SOAK 完成! ${SOAK_HOURS}h ==="
    record_metrics "${server_pid}" "${total_seconds}"
}

# ── 入口 ──

main() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║  SQLRustGo SOAK Test Runner   v3.9.0                       ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""

    preflight

    echo ""
    echo "配置:"
    echo "  时长:           ${SOAK_HOURS}h"
    echo "  端口:           ${SOAK_PORT}"
    echo "  sysbench表:     ${SOAK_TABLE_SIZE} rows × 1 table"
    echo "  服务线程:       ${SOAK_SERVER_THR}"
    echo "  sysbench线程:   ${SOAK_SB_THR}"
    echo "  结果目录:       ${SOAK_RESULTS_DIR}"
    echo "  日志循环:       < 1GB (server 内置 10×100MB)"
    if command -v ionice >/dev/null 2>&1; then
        echo "  ⚡ ionice: 可用"
    else
        echo "  ⚡ ionice: 本平台不支持, 仅使用 nice"
    fi
    echo ""

    setup_run_dir

    # 捕获 EXIT + 常见信号, 确保任何异常退出都会触发 cleanup, 在 monitor.log
    # 中留下 `=== 清理 ===` / `=== SOAK 已停止 ===` 标记, 便于事后取证。
    # 仅 SIGKILL (内核强制) 无法捕获, 此时 PATCHED detach (line 76) 已能
    # 大幅降低被 SIGKILL 的概率。参见 INCIDENT-REPORT-2026-08-30.md §3.1。
    trap cleanup EXIT INT TERM HUP QUIT
    main_loop
}

# v312-59-D / #4597: 允许 chaos_drills.sh 等被 source 而不自动触发 main().
# 设置 SOAK_NO_MAIN=1 即可复用 start_server / stop_current_server 等函数.
if [[ -z "${SOAK_NO_MAIN:-}" ]]; then
    main "$@"
fi
