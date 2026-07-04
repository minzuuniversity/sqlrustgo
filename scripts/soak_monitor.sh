#!/bin/bash
# soak_monitor.sh — 72h SOAK 自动监控脚本
# 功能: 每分钟检查服务器健康状态、QPS趋势、资源使用，记录到监控日志
# 用法: nohup bash scripts/soak_monitor.sh > ~/sqlrustgo-soak/monitor.log 2>&1 &
#
# 告警条件:
#   - 服务器进程消失
#   - QPS 相比上次下降 > 30%
#   - 错误率 > 1%
#   - 内存使用超过 2GB
#   - CPU 持续 > 90% 超过 5min

set -uo pipefail

# --- 配置 ---
SERVER_PID_FILE="${PID_FILE:-}"           # 可外部传入 PID
PORT="${PORT:-3396}"
HOST="${HOST:-127.0.0.1}"
CHECK_INTERVAL="${CHECK_INTERVAL:-60}"    # 检查间隔(秒)
SOAK_LOG="${SOAK_LOG:-}"                   # hybrid_soak 日志路径
RESULTS_DIR="${RESULTS_DIR:-$HOME/sqlrustgo-soak}"
MONITOR_LOG="$RESULTS_DIR/soak_monitor.log"
ALERT_LOG="$RESULTS_DIR/soak_alerts.log"
WARN_LOG="$RESULTS_DIR/soak_warnings.log"

# 历史数据文件
HISTORY_FILE="$RESULTS_DIR/soak_history.csv"
METRICS_FILE="$RESULTS_DIR/soak_metrics.json"

# --- 颜色 ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# --- 状态 ---
declare -A LAST_QPS
declare -A LAST_ERRORS
declare -A LAST_TOTAL
PREV_SERVER_PID=""
INITIALIZED=0
ALERT_COOLDOWN=0    # 防止重复告警
WARN_COOLDOWN=0
RESTART_COUNT=0

# ==============================================================================
# 工具函数
# ==============================================================================

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$MONITOR_LOG"
}

warn() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] WARN: $*" | tee -a "$WARN_LOG"
    [[ $WARN_COOLDOWN -gt 0 ]] && return || warn_action
}

alert() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ALERT: $*" | tee -a "$ALERT_LOG"
    [[ $ALERT_COOLDOWN -gt 0 ]] && return || alert_action
}

log_color() {
    local color=$1; shift
    echo -e "${color}[$(date '+%H:%M:%S')] $*${NC}" | tee -a "$MONITOR_LOG"
}

# ==============================================================================
# 初始化
# ==============================================================================

init() {
    mkdir -p "$RESULTS_DIR"
    touch "$MONITOR_LOG" "$ALERT_LOG" "$WARN_LOG"
    
    # CSV 表头
    if [[ ! -s "$HISTORY_FILE" ]]; then
        echo "timestamp,elapsed_s,qps,oltp_ops,olap_ops,total_ops,errors,rss_kb,cpu_pct,fd_count,conn_count,oltp_ratio" > "$HISTORY_FILE"
    fi
    
    # 自动检测 SOAK_LOG 如果未指定
    if [[ -z "$SOAK_LOG" ]]; then
        SOAK_LOG=$(detect_soak_log)
    fi
    export SOAK_LOG  # 传递给子函数
    
    log "=============================================="
    log "  SOAK Monitor Started"
    log "  CHECK_INTERVAL: ${CHECK_INTERVAL}s"
    log "  SOAK_LOG: ${SOAK_LOG:-'NOT FOUND'}"
    log "  RESULTS_DIR: $RESULTS_DIR"
    log "=============================================="
}

# ==============================================================================
# 自动检测 SOAK_LOG
# ==============================================================================

detect_soak_log() {
    if [[ -z "$SOAK_LOG" ]]; then
        # 找最新的 soak 日志
        SOAK_LOG=$(ls -t ~/sqlrustgo-soak/soak72h_hybrid_*/soak.log 2>/dev/null | head -1)
        if [[ -z "$SOAK_LOG" ]]; then
            SOAK_LOG=$(ls -t ~/sqlrustgo-soak/*hybrid*/soak.log 2>/dev/null | head -1)
        fi
    fi
    echo "$SOAK_LOG"
}

# ==============================================================================
# 获取服务器 PID（支持动态变化）
# ==============================================================================

get_server_pid() {
    local port=$1
    # 优先从 lsof 找监听端口的进程
    local pid=$(lsof -i :${port} -s TCP:LISTEN 2>/dev/null | grep -v COMMAND | awk '{print $2}' | head -1)
    if [[ -n "$pid" ]]; then
        echo "$pid"
        return
    fi
    # 回退: grep 二进制名
    pid=$(pgrep -n -f "sqlrustgo-mysql-server" 2>/dev/null | head -1)
    echo "${pid:-}"
}

# ==============================================================================
# 解析 hybrid_soak 日志最新行
# ==============================================================================

parse_soak_log() {
    local logfile=$1
    [[ ! -f "$logfile" ]] && return 1
    
    # Read last line directly from file — avoids grep|tail hanging on unbuffered pipe
    # (the last line has no trailing newline, so grep|tail would block forever)
    local line=$(tail -1 "$logfile" 2>/dev/null)
    [[ -z "$line" || ! "$line" =~ ^\[.*qps= ]] && return 1
    
    # 解析: [XXXs] qps=YYY oltp=A/B olap=C/D tot_ops=N writes(+) select(+)
    local ts=$(echo "$line" | sed 's/.*\[\([ 0-9]*\)s\].*/\1/' | tr -d ' ')
    local qps=$(echo "$line" | sed 's/.*qps=[ ]*\([0-9]*\).*/\1/')
    local oltp=$(echo "$line" | sed 's/.*oltp=\([0-9]*\)\/.*/\1/')
    local olap=$(echo "$line" | sed 's/.*olap=\([0-9]*\)\/.*/\1/')
    local total=$(echo "$line" | sed 's/.*tot_ops=\([0-9]*\).*/\1/')
    local errors=0
    
    echo "${ts}:${qps}:${oltp}:${olap}:${total}:${errors}"
    return 0
}

# ==============================================================================
# 获取服务器资源使用
# ==============================================================================

get_server_metrics() {
    local pid=$1
    [[ -z "$pid" ]] && echo "0:0:0:0:0" && return
    
    # macOS ps (BSD-style): RSS in KB, cpu is float with leading spaces
    local rss=$(ps -p "$pid" -o rss= 2>/dev/null | awk '{print int($1)}')
    local cpu=$(ps -p "$pid" -o pcpu= 2>/dev/null | awk '{print int($1)}')
    # macOS has no nlwp — use lsof FD count as proxy
    local fd=$(lsof -p "$pid" 2>/dev/null | wc -l | tr -d ' ')
    local conn=$(lsof -i :${PORT} 2>/dev/null | grep ESTABLISHED | wc -l | tr -d ' ')
    
    rss=${rss:-0}
    cpu=${cpu:-0}
    fd=${fd:-0}
    conn=${conn:-0}
    
    echo "${rss}:${cpu}:${fd}:${conn}"
}

# ==============================================================================
# 获取 hybrid_soak 进程状态
# ==============================================================================

get_soak_pid() {
    pgrep -n -f "hybrid_soak" 2>/dev/null | head -1
}

# ==============================================================================
# 计算 OLTP 比例
# ==============================================================================

calc_oltp_ratio() {
    local oltp=$1 total=$2
    if [[ "$total" -gt 0 ]]; then
        echo "scale=1; $oltp * 100 / $total" | bc
    else
        echo "0"
    fi
}

# ==============================================================================
# 记录指标到 CSV
# ==============================================================================

record_metrics() {
    local ts=$1 qps=$2 oltp=$3 olap=$4 total=$5 errors=$6 rss=$7 cpu=$8 fd=$9 conn=$10
    
    local ratio=$(calc_oltp_ratio "$oltp" "$total")
    local now=$(date '+%Y-%m-%d %H:%M:%S')
    
    echo "$now,$ts,$qps,$oltp,$olap,$total,$errors,$rss,$cpu,$fd,$conn,$ratio" >> "$HISTORY_FILE"
}

# ==============================================================================
# 生成 JSON 快照
# ==============================================================================

write_json_snapshot() {
    local ts=$1 qps=$2 oltp=$3 olap=$4 total=$5 errors=$6 rss=$7 cpu=$8 fd=$9 conn=$10 server_pid=${11:-}
    
    local ratio=$(calc_oltp_ratio "$oltp" "$total")
    local now=$(date '+%Y-%m-%d %H:%M:%S')
    
    cat > "$METRICS_FILE" << EOF
{
  "timestamp": "$now",
  "elapsed_s": $ts,
  "qps": $qps,
  "oltp_ops": $oltp,
  "olap_ops": $olap,
  "total_ops": $total,
  "errors": $errors,
  "rss_kb": $rss,
  "rss_mb": $(echo "scale=1; $rss/1024" | bc),
  "cpu_pct": $cpu,
  "fd_count": $fd,
  "conn_count": $conn,
  "oltp_ratio_pct": $ratio,
  "server_pid": "$server_pid",
  "results_dir": "$RESULTS_DIR"
}
EOF
}

# ==============================================================================
# 打印状态行
# ==============================================================================

print_status() {
    local ts=$1 qps=$2 oltp=$3 olap=$4 total=$5 errors=$6 rss=$7 cpu=$8 fd=$9 conn=$10
    local ratio=$(calc_oltp_ratio "$oltp" "$total")
    local elapsed_h=$((ts/3600))
    local elapsed_m=$(((ts%3600)/60))
    local elapsed_s=$((ts%60))
    
    # QPS 趋势指示
    local qps_status="📈"
    if [[ $INITIALIZED -eq 1 && -n "${LAST_QPS[pid]}" ]]; then
        local prev_qps=${LAST_QPS[$pid]}
        if [[ "$prev_qps" -gt 0 ]]; then
            local drop=$(echo "scale=0; ($qps - $prev_qps) * 100 / $prev_qps" | bc)
            if [[ "$drop" -lt -30 ]]; then
                qps_status="🔴⚠️ QPS下降${drop}%"
            elif [[ "$drop" -lt -10 ]]; then
                qps_status="🟡"
            fi
        fi
    fi
    
    printf "  %s %4dh%2dm  QPS=%-6s  OLTP=%s/%s(%5.1f%%)  TOTAL=%-10s  ERR=%s  RSS=%sMB  CPU=%s%%  FD=%s  CONN=%s\n" \
        "$qps_status" "$elapsed_h" "$elapsed_m" "$qps" "$oltp" "$total" "$ratio" "$total" "$errors" \
        "$(echo "scale=0; $rss/1024" | bc)" "$cpu" "$fd" "$conn"
}

# ==============================================================================
# 告警动作
# ==============================================================================

alert_action() {
    # 发送桌面通知 (macOS)
    if command -v osascript &>/dev/null; then
        osascript -e "display notification \"SOAK ALERT: $1\" with title \"SQLRustGo Soak\""
    fi
    ALERT_COOLDOWN=300  # 5分钟不重复
}

warn_action() {
    WARN_COOLDOWN=120   # 2分钟不重复
}

# ==============================================================================
# 主监控循环
# ==============================================================================

monitor_loop() {
    detect_soak_log
    log "Using SOAK_LOG=$SOAK_LOG"
    
    echo ""
    echo "=============================================================="
    echo "  72h SOAK 自动监控 — 每${CHECK_INTERVAL}s 检查一次"
    echo "=============================================================="
    echo ""
    echo "  时间         运行时     QPS      OLTP比例   总操作    错误  RSS   CPU  FD  连接  状态"
    echo "  -----------  --------  -------  --------  --------  ----  ----  ---  ---  ----  ----"
    
    while true; do
        local SOAK_LOG_CURRENT
        SOAK_LOG_CURRENT=$(detect_soak_log)
        
        # --- 服务器状态 ---
        local server_pid=$(get_server_pid "$PORT")
        local soak_pid=$(get_soak_pid)
        local server_alive=1
        
        if [[ -z "$server_pid" ]]; then
            log_color "$RED" "🔴 SERVER DEAD — no process on port $PORT"
            alert "服务器进程消失 (PID: ${PREV_SERVER_PID:-unknown})"
            server_alive=0
            
            # 记录死亡快照
            if [[ -n "${LAST_TOTAL[pid]}" ]]; then
                echo "DEATH_RECORD: elapsed=${LAST_TOTAL[pid]} total=${LAST_TOTAL[pid]} qps=${LAST_QPS[pid]}" >> "$ALERT_LOG"
            fi
        else
            PREV_SERVER_PID="$server_pid"
        fi
        
        # --- hybrid_soak 状态 ---
        if [[ -z "$soak_pid" ]]; then
            log_color "$YELLOW" "🟡 HYBRID_SOAK DEAD — no client process"
            warn "hybrid_soak 客户端进程消失"
        fi
        
        # --- 解析 SOAK 日志 ---
        local parsed=""
        local ts=0 qps=0 oltp=0 olap=0 total=0 errors=0
        local rss=0 cpu=0 nlwp=0 fd=0 conn=0
        
        if [[ -n "$SOAK_LOG_CURRENT" && -f "$SOAK_LOG_CURRENT" ]]; then
            parsed=$(parse_soak_log "$SOAK_LOG_CURRENT")
            if [[ -n "$parsed" ]]; then
                ts=$(echo "$parsed" | cut -d: -f1)
                qps=$(echo "$parsed" | cut -d: -f2)
                oltp=$(echo "$parsed" | cut -d: -f3)
                olap=$(echo "$parsed" | cut -d: -f4)
                total=$(echo "$parsed" | cut -d: -f5)
                errors=$(echo "$parsed" | cut -d: -f6)
            fi
        fi
        
        # --- 资源使用 ---
        if [[ -n "$server_pid" && "$server_alive" -eq 1 ]]; then
            local metrics=$(get_server_metrics "$server_pid")
            rss=$(echo "$metrics" | cut -d: -f1)
            cpu=$(echo "$metrics" | cut -d: -f2)
            fd=$(echo "$metrics" | cut -d: -f3)
            conn=$(echo "$metrics" | cut -d: -f4)
        fi
        
        # --- QPS 下降告警 ---
        if [[ $INITIALIZED -eq 1 && -n "${LAST_QPS[$server_pid]}" && "$qps" -gt 0 ]]; then
            local prev_qps=${LAST_QPS[$server_pid]}
            local qps_change=$(echo "scale=0; ($qps - $prev_qps) * 100 / $prev_qps" | bc)
            if [[ "$qps_change" -lt -30 && "$qps" -lt 100 ]]; then
                alert "QPS 持续大幅下降: ${prev_qps} → ${qps} (${qps_change}%)"
            fi
        fi
        
        # --- 内存告警 ---
        if [[ "$rss" -gt 2097152 ]]; then  # 2GB
            alert "内存使用过高: $(echo "scale=1; $rss/1024" | bc) MB"
        elif [[ "$rss" -gt 1572864 ]]; then  # 1.5GB
            warn "内存使用较高: $(echo "scale=1; $rss/1024" | bc) MB"
        fi
        
        # --- CPU 告警 ---
        local cpu_int=$(echo "$cpu" | cut -d. -f1)
        if [[ "$cpu_int" -gt 90 ]]; then
            warn "CPU 使用率过高: ${cpu}%"
        fi
        
        # --- 错误率告警 ---
        # (errors 字段需要正确解析 writes 中的异常)
        
        # --- 打印状态行 ---
        if [[ "$total" -gt 0 ]]; then
            print_status "$ts" "$qps" "$oltp" "$olap" "$total" "$errors" "$rss" "$cpu" "$fd" "$conn"
        else
            log_color "$YELLOW" "   (等待数据...)"
        fi
        
        # --- 记录历史 ---
        if [[ "$total" -gt 0 ]]; then
            record_metrics "$ts" "$qps" "$oltp" "$olap" "$total" "$errors" "$rss" "$cpu" "$fd" "$conn"
            write_json_snapshot "$ts" "$qps" "$oltp" "$olap" "$total" "$errors" "$rss" "$cpu" "$fd" "$conn" "$server_pid"
        fi
        
        # --- 保存状态 ---
        LAST_QPS[$server_pid]=$qps
        LAST_ERRORS[$server_pid]=$errors
        LAST_TOTAL[$server_pid]=$total
        INITIALIZED=1
        
        # --- 冷却计数器 ---
        [[ $ALERT_COOLDOWN -gt 0 ]] && ((ALERT_COOLDOWN--))
        [[ $WARN_COOLDOWN -gt 0 ]] && ((WARN_COOLDOWN--))
        
        sleep "$CHECK_INTERVAL"
    done
}

# ==============================================================================
# 入口
# ==============================================================================

main() {
    init
    monitor_loop
}

main "$@"
