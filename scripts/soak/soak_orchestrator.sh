#!/usr/bin/env bash
# soak_orchestrator.sh — SQLRustGo SOAK 测试总控
# 用法: bash scripts/soak/soak_orchestrator.sh [时长_小时]
#
# 功能:
#  - 启动 server (nice -n 10)
#  - sysbench prepare + sysbench run (nice -n 15)
#  - Python 负载生成 (nice -n 15)
#  - 10min 报告 RSS/WAL/FD/线程/QPS/磁盘
#  - 磁盘 > 800MB 自动循环重启
#  - Ctrl+C 干净退出
# ==============================================================================

set -o pipefail
shopt -s nullglob

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BINARY="${PROJECT_ROOT}/target/release/sqlrustgo-mysql-server"
SOAK_DIR="${HOME}/sqlrustgo-soak-results"
PORT="${SOAK_PORT:-3396}"
HOURS="${1:-24}"
START_TS=$(date +%s)
DEADLINE=$((START_TS + HOURS * 3600))

# 循环用计数器
CYCLE=0

cleanup_all() {
    echo ""
    echo "[清洁] 停止所有进程..."
    pkill -P $$ 2>/dev/null || true
    jobs -p 2>/dev/null | xargs kill -TERM 2>/dev/null || true
    sleep 2
    jobs -p 2>/dev/null | xargs kill -KILL 2>/dev/null || true
    echo "[清洁] 完成"
}
trap cleanup_all EXIT

log() { echo "[$(date '+%H:%M:%S')] $*"; }

start_cycle() {
    CYCLE=$((CYCLE + 1))
    local ts=$(date '+%Y%m%d_%H%M%S')
    local run_dir="${SOAK_DIR}/cycle${CYCLE}_${ts}"
    local data_dir="/tmp/sqlrustgo-soak-${PORT}"
    local log_dir="/tmp/sqlrustgo-soak-logs-${PORT}"

    # 注意：数据库文件绝对不允许删除。data_dir 只在首次启动时创建，
    # 后续 cycle（server crash 重启后）直接复用已有数据和 WAL。
    # 日志目录可以安全清理（重建后会从头生成）。
    mkdir -p "${run_dir}" "${log_dir}"
    # 只在 data_dir 不存在时创建（首次启动）
    if [[ ! -d "${data_dir}" ]]; then
        mkdir -p "${data_dir}"
        log "[cycle${CYCLE}] 创建数据目录 ${data_dir}"
    else
        log "[cycle${CYCLE}] 复用已有数据目录 ${data_dir}"
    fi
    RUN_DIR="${run_dir}"
    echo "${run_dir}" > "${SOAK_DIR}/latest.txt"
    echo "${data_dir}" > "${SOAK_DIR}/data_dir.txt"
    echo "${log_dir}" > "${SOAK_DIR}/log_dir.txt"

    # ── 启动 server ──
    log "[cycle${CYCLE}] 启动 server (port=${PORT}, nice -n 10)"
    nice -n 10 "${BINARY}" serve \
        --port "${PORT}" --tls off \
        --server-threads 16 --max-connections 200 \
        --data-dir "${data_dir}" --log-dir "${log_dir}" \
        --log-level info --storage file \
        > "${run_dir}/server.log" 2>&1 &
    local sv_pid=$!
    echo "${sv_pid}" > "${run_dir}/server.pid"

    # Use port liveness check instead of kill -0: on macOS nice may exec
    # the target (same PID) or fork (different PID), so $sv_pid can diverge.
    # We also accept "listening on PORT" text in server.log as a fallback.
    for i in $(seq 1 30); do
        if lsof -P -i ":${PORT}" -sTCP:LISTEN 2>/dev/null | grep -q LISTEN; then
            log "[cycle${CYCLE}]  server 就绪 (${i}s)"
            break
        fi
        # Fallback: check if server.log mentions "Ready to accept connections"
        if grep -q "Ready to accept connections" "${run_dir}/server.log" 2>/dev/null; then
            log "[cycle${CYCLE}]  server 就绪 (log check, ${i}s)"
            break
        fi
        if [ $i -eq 30 ]; then
            log "[cycle${CYCLE}]  server 启动失败"
            tail -5 "${run_dir}/server.log"
            return 1
        fi
        sleep 1
    done

    # ── sysbench prepare ──
    log "[cycle${CYCLE}] sysbench prepare (10000 rows)"
    sysbench oltp_read_only \
        --db-driver=mysql --mysql-host=127.0.0.1 --mysql-port="${PORT}" \
        --mysql-user=root --mysql-password="" --mysql-db=sbtest \
        --table-size=10000 --tables=1 \
        prepare >> "${run_dir}/sysbench_prepare.log" 2>&1 || true
    log "[cycle${CYCLE}]  prepare 完成"

    sleep 1

    # ── sysbench run (后台) ──
    log "[cycle${CYCLE}] sysbench run (8 threads, nice -n 15)"
    nice -n 15 sysbench oltp_read_only \
        --db-driver=mysql --db-ps-mode=disable \
        --mysql-host=127.0.0.1 --mysql-port="${PORT}" \
        --mysql-user=root --mysql-password="" --mysql-db=sbtest \
        --table-size=10000 --tables=1 \
        --threads=8 --time=86400 --report-interval=10 \
        run >> "${run_dir}/sysbench.log" 2>&1 &
    local sb_pid=$!
    echo "${sb_pid}" > "${run_dir}/sysbench.pid"
    log "[cycle${CYCLE}]  sysbench PID: ${sb_pid}"

    return 0
}

# ── 指标采样 ──
read_metric() {
    # Find real server PID via port (server.pid may be nice's PID on macOS)
    local port="${1:-3396}"
    local pid
    pid=$(lsof -P -i ":${port}" -sTCP:LISTEN -t 2>/dev/null | head -1)
    [[ -z "${pid}" ]] && { echo "0 0 0 0 0"; return; }

    # RSS (KB) — macOS: ps -o rss=  Linux: /proc/pid/status
    local rss=0
    [[ -f "/proc/${pid}/status" ]] && rss=$(awk '/VmRSS:/{gsub(/[^0-9]/,"",$2);print $2}' "/proc/${pid}/status" 2>/dev/null) || rss=$(ps -o rss= -p "${pid}" 2>/dev/null | tr -d ' ')
    rss=${rss:-0}

    # FD count
    local fd=0
    if [[ -d "/proc/${pid}/fd" ]]; then fd=$(ls -1 "/proc/${pid}/fd" 2>/dev/null | wc -l); else fd=$(lsof -p "${pid}" 2>/dev/null | wc -l); fi
    fd=${fd:-0}

    # Thread count
    local thr=0
    if [[ -d "/proc/${pid}/task" ]]; then thr=$(ls -1 "/proc/${pid}/task" 2>/dev/null | wc -l); else thr=$(ps -M -p "${pid}" 2>/dev/null | tail -n +2 | wc -l); fi
    thr=${thr:-0}

    # WAL file size
    local wal=0 data_dir=$(cat "${SOAK_DIR}/data_dir.txt" 2>/dev/null || echo "")
    for w in "${data_dir}/sqlrustgo.wal" "${data_dir}/wal"; do
        [[ -f "${w}" ]] && { wal=$(stat -f%z "${w}" 2>/dev/null); break; }
    done
    wal=${wal:-0}

    echo "${rss} ${fd} ${thr} ${wal}"
}
# ==============================================================================
# 主循环
# ==============================================================================

log "===== SQLRustGo SOAK ${HOURS}h ====="
log "Port: ${PORT}, Binary: ${BINARY}"
log ""

start_cycle || { log "启动失败"; exit 1; }
log "结果: ${RUN_DIR}"
log ""

echo "ts,elapsed,rss_kb,fd,threads,wal_bytes,disk_bytes,sysbench_qps" > "${SOAK_DIR}/all_metrics.csv"

SAMPLE=0
while [[ $(date +%s) -lt ${DEADLINE} ]]; do
    sleep 60

    # 获取最新 run_dir
    RUN_DIR=$(head -1 "${SOAK_DIR}/latest.txt" 2>/dev/null || echo "${RUN_DIR}")
    ELAPSED=$(( $(date +%s) - START_TS ))

    # 检查 server
    # 检查 server — macOS lsof 把 3396 解析成 service name (printer_agent),
    # 所以 grep 端口号字符串会失败。改用 -P 禁用名称解析，然后用 grep LISTEN。
    if ! lsof -P -i ":${PORT}" -sTCP:LISTEN 2>/dev/null | grep -q LISTEN; then
        log "[${ELAPSED}s] Server 挂了! 重启"
        start_cycle || { log "重启失败"; continue; }
        SAMPLE=0
        continue
    fi
    DATA_DIR=$(cat "${SOAK_DIR}/data_dir.txt" 2>/dev/null || echo "")
    LOG_DIR=$(cat "${SOAK_DIR}/log_dir.txt" 2>/dev/null || echo "/tmp/sqlrustgo-soak-logs-${PORT}")
    # 读取指标 (RSS, FD, THR, WAL)
    read -r RSS FD THR WAL _ <<< "$(read_metric "${PORT}")"
    SB_QPS=$(grep -a "qps:" "${RUN_DIR}/sysbench.log" 2>/dev/null | tail -1 | sed 's/.*qps: *//' | sed 's/ .*//' || echo "0")
    DISK_B=$(du -sk "${DATA_DIR}" "${LOG_DIR}" 2>/dev/null | cut -f1 | awk '{s+=$1}END{printf "%d",s*1024}' 2>/dev/null || echo 0)

    # 磁盘限制：8 GB。超过则清理旧日志归档（.gz），绝不丢失数据库文件。
    # 数据库文件丢失是最严重的事故，禁止重启服务器或删除 data_dir。
    DISK_LIMIT=$((8 * 1024 * 1024 * 1024))  # 8 GB
    if [[ ${DISK_B} -gt ${DISK_LIMIT} ]]; then
        log "[${ELAPSED}s] ⚠ 磁盘 ${DISK_B}B > 8GB, 清理旧日志"
        # 只清理旧的 gzip 日志归档，不碰当前活动日志和数据目录
        local log_archives
        log_archives=$(ls -1t "${LOG_DIR}"/*.log.gz 2>/dev/null | tail -n +5)
        if [[ -n "${log_archives}" ]]; then
            while IFS= read -r arch; do
                rm -f "${arch}"
                log "  删除旧日志: ${arch}"
            done <<< "${log_archives}"
        fi
        # 如果清理后仍然超过，最坏情况：只警告，不重启
        sleep 1
        DISK_B=$(du -sk "${DATA_DIR}" "${LOG_DIR}" 2>/dev/null | cut -f1 | awk '{s+=$1}END{printf "%d",s*1024}' 2>/dev/null || echo 0)
        log "[${ELAPSED}s] 清理后磁盘: ${DISK_B}B"
    elif [[ ${DISK_B} -gt $((DISK_LIMIT * 8 / 10)) ]]; then
        log "[${ELAPSED}s] ⚠ 磁盘 ${DISK_B}B > 6.4GB (80%), 接近上限"
    fi


    # 写 CSV
    echo "$(date +%s),${ELAPSED},${RSS},${FD},${THR},${WAL},${DISK_B},${SB_QPS}" >> "${SOAK_DIR}/all_metrics.csv"

    RSS_MB=$(awk "BEGIN{printf \"%.1f\",${RSS}/1024}")
    WAL_MB=$(awk "BEGIN{printf \"%.2f\",${WAL}/1048576}")
    DISK_MB=$(awk "BEGIN{printf \"%.0f\",${DISK_B}/1048576}")

    # 10min 报告
    if [[ $((SAMPLE % 10)) -eq 0 ]]; then
        # server 内置 monitor 最新一行
        BUILTIN=$(grep -a "RESOURCE_MONITOR" "${RUN_DIR}/server.log" 2>/dev/null | tail -1)
        # sysbench 最新 report
        SB_REPORT=$(grep -a "thds:" "${RUN_DIR}/sysbench.log" 2>/dev/null | tail -1)

        echo ""
        echo "══════════════════════════════════════════════════════"
        echo "  SOAK 报告  cycle=${CYCLE}  ${ELAPSED}s ($((ELAPSED/60))m)"
        echo "──────────────────────────────────────────────────────"
        echo "  RSS:    ${RSS_MB} MB"
        echo "  FD:     ${FD}"
        echo "  线程:   ${THR}"
        echo "  WAL:    ${WAL_MB} MB"
        echo "  磁盘:   ${DISK_MB} MB / 800 MB"
        echo "  QPS:    ${SB_QPS}"
        echo "──────────────────────────────────────────────────────"
        echo "  [server] ${BUILTIN}"
        echo "  [sysbench] ${SB_REPORT}"
        echo "══════════════════════════════════════════════════════"
        echo ""

        # 保存到报告文件
        {
            echo "=== $(date '+%Y-%m-%d %H:%M:%S') cycle=${CYCLE} elapsed=${ELAPSED}s ==="
            echo "RSS=${RSS_MB}MB FD=${FD} THR=${THR} WAL=${WAL_MB}MB DISK=${DISK_MB}MB QPS=${SB_QPS}"
            echo "server: ${BUILTIN}"
            echo "sysbench: ${SB_REPORT}"
            echo ""
        } >> "${RUN_DIR}/10min_reports.log"
    fi

    # 进度
    REMAIN=$((DEADLINE - $(date +%s)))
    printf "\r  [%4ds] cycle=${CYCLE} RSS=${RSS_MB}MB FD=${FD} THR=${THR} WAL=${WAL_MB}MB DISK=${DISK_MB}MB QPS=${SB_QPS}  " "${ELAPSED}"

    SAMPLE=$((SAMPLE + 1))
done

echo ""
log "===== SOAK ${HOURS}h 完成 ====="
log "指标文件: ${SOAK_DIR}/all_metrics.csv"
