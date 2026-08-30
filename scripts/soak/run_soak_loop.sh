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
    if [[ $fail -ne 0 ]]; then exit 1; fi
    log "前置检查通过"
    log "  二进制: ${BINARY}"
    log "  sysbench: $(sysbench --version 2>&1 | head -1)"
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

start_sysbench() {
    log "=== sysbench prepare ==="
    sysbench oltp_read_write \
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

    log "启动 sysbench run (${SOAK_SB_THR} threads, nice -n ${SOAK_NICE_SYSBENCH})"
    # v312-59-d / #4594 P-4: sysbench --time 必须 ≥ SOAK_HOURS, 否则 SOAK 结束后 sysbench 孤立运行
    # +1h buffer 确保 sysbench 比 main_loop 后退出, 避免 SOAK 中途 orphan 干扰
    local sysbench_seconds=$(( (SOAK_HOURS + 1) * 3600 ))
    # v312-59-d / #4594 P-3: nice level 可经 SOAK_NICE_SYSBENCH 覆盖
    nice -n "${SOAK_NICE_SYSBENCH}" \
        sysbench oltp_read_write \
        --db-driver=mysql \
        --db-ps-mode=disable \
        --mysql-host=127.0.0.1 \
        --mysql-port="${SOAK_PORT}" \
        --mysql-user=root \
        --mysql-password="" \
        --mysql-db=sbtest \
        --table-size="${SOAK_TABLE_SIZE}" \
        --tables=1 \
        --threads="${SOAK_SB_THR}" \
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

# ── 10分钟指标报告 ──

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

    echo "${now},${elapsed},${rss_kb},${fd},${threads},${wal_b},${disk_b},${sv_qps},${sb_qps}" >> "${METRICS_CSV}"
    echo "${now},${elapsed},${rss_kb},${fd},${threads},${wal_b},${disk_b},${sv_qps},${sb_qps}" >> "${HISTORY_CSV}"

    local rss_mb wal_mb disk_mb
    rss_mb=$(awk "BEGIN {printf \"%.1f\", ${rss_kb}/1024}")
    wal_mb=$(awk "BEGIN {printf \"%.2f\", ${wal_b}/1048576}")
    disk_mb=$(awk "BEGIN {printf \"%.0f\", ${disk_b}/1048576}")

    local builtin_line
    builtin_line=$(grep -a "RESOURCE_MONITOR" "${SERVER_LOG}" 2>/dev/null | tail -1)
    local sb_line
    sb_line=$(grep -a "thds:" "${SYSBENCH_LOG}" 2>/dev/null | tail -1)

    cat >> "${PERIODIC_LOG}" <<REPORT

$(date '+%Y-%m-%d %H:%M:%S') === SOAK 10min Report ===
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

    echo ""
    echo "=== SOAK 10min Report ==="
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
    echo "║  Server: nice -n 10 | Sysbench: nice -n 15                 ║"
    echo "║  Stop: kill -TERM \$(cat ${RUN_DIR}/server.pid) or Ctrl+C   ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""
    echo "ts,elapsed_s,rss_kb,fd,threads,wal_bytes,disk_bytes,server_qps,sysbench_qps" > "${METRICS_CSV}"

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

            setup_run_dir
            echo "ts,elapsed_s,rss_kb,fd,threads,wal_bytes,disk_bytes,server_qps,sysbench_qps" > "${METRICS_CSV}"

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

main "$@"
