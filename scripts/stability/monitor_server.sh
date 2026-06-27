#!/bin/bash
# monitor_server.sh - Z6G4 resource monitor (called by run_wired_soak_with_monitor.sh)
# Monitors sqlrustgo-mysql-server process and writes to resource_monitor.log
# Alerts to resource_alerts.log on threshold breach
#
# Usage: bash monitor_server.sh [PID_FILE] [RESULTS_DIR] [INTERVAL=60]

set -euo pipefail

PID_FILE="${1:-}"
RESULTS_DIR="${2:-test_results/soak_monitor}"
INTERVAL="${3:-30}"

ALERT_LOG="$RESULTS_DIR/resource_alerts.log"
MONITOR_LOG="$RESULTS_DIR/resource_monitor.log"

RSS_SOFT_MB="${RSS_SOFT_MB:-4096}"
RSS_HARD_LIMIT_MB="${RSS_HARD_LIMIT_MB:-8192}"
FD_HARD_LIMIT="${FD_HARD_LIMIT:-8000}"
DISK_MIN_GB="${DISK_MIN_GB:-2}"
RSS_GROWTH_RATE_MB_PER_HR="${RSS_GROWTH_RATE_MB_PER_HR:-200}"

mkdir -p "$RESULTS_DIR"

# Find server PID
get_server_pid() {
    if [ -n "$PID_FILE" ] && [ -f "$PID_FILE" ]; then
        cat "$PID_FILE" 2>/dev/null || echo ""
    else
        pgrep -f "sqlrustgo-mysql-server.*serve" | head -1 || echo ""
    fi
}

SERVER_PID=$(get_server_pid)
if [ -z "$SERVER_PID" ]; then
    echo "[WARN $(date)] No server PID found yet"
fi

INITIAL_RSS=0
INITIAL_TS=0

echo "ts,elapsed_h,rss_mb,rss_growth_mb,fd_count,cpu_pct,disk_free_gb,server_alive" > "$MONITOR_LOG"

START_TS=$(date +%s)
SAMPLE=0

while true; do
    TS=$(date +%s)
    TS_STR=$(date '+%Y-%m-%dT%H:%M:%SZ')
    ELAPSED_H=$(awk "BEGIN {printf "%.2f", ($TS - $START_TS) / 3600}")
    
    SERVER_PID=$(get_server_pid)
    
    RSS_MB=0
    FD_COUNT=0
    CPU_PCT=0
    DISK_GB=0
    ALIVE=0
    
    if [ -n "$SERVER_PID" ] && [ -d "/proc/$SERVER_PID" ]; then
        RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
        RSS_MB=$((RSS_KB / 1024))
        FD_COUNT=$(ls /proc/$SERVER_PID/fd 2>/dev/null | wc -l || echo 0)
        CPU_PCT=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
        ALIVE=1
        
        if [ $SAMPLE -eq 0 ]; then
            INITIAL_RSS=$RSS_MB
            INITIAL_TS=$TS
        fi
    fi
    
    DISK_GB=$(df -BG /tmp 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G' || echo 0)
    RSS_GROWTH=$((RSS_MB - INITIAL_RSS))
    
    echo "$TS_STR,$ELAPSED_H,$RSS_MB,$RSS_GROWTH,$FD_COUNT,$CPU_PCT,$DISK_GB,$ALIVE" >> "$MONITOR_LOG"
    
    # Threshold checks
    if [ "$ALIVE" -eq 0 ]; then
        echo "[CRITICAL $TS_STR] Server dead (PID=$SERVER_PID)" >> "$ALERT_LOG"
        break
    fi
    
    if [ "$RSS_MB" -gt $RSS_HARD_LIMIT_MB ]; then
        echo "[KILL $TS_STR] RSS ${RSS_MB}MB > hard limit ${RSS_HARD_LIMIT_MB}MB" >> "$ALERT_LOG"
        break
    fi
    
    if [ "$FD_COUNT" -gt $FD_HARD_LIMIT ]; then
        echo "[KILL $TS_STR] FD ${FD_COUNT} > hard limit ${FD_HARD_LIMIT}" >> "$ALERT_LOG"
        break
    fi
    
    if [ "$DISK_GB" -lt $DISK_MIN_GB ]; then
        echo "[KILL $TS_STR] Disk /tmp only ${DISK_GB}GB < ${DISK_MIN_GB}GB" >> "$ALERT_LOG"
        break
    fi
    
    if [ "$RSS_MB" -gt $RSS_SOFT_MB ]; then
        echo "[WARN $TS_STR] RSS ${RSS_MB}MB > soft limit ${RSS_SOFT_MB}MB (growth=${RSS_GROWTH}MB)" >> "$ALERT_LOG"
    fi
    
    # RSS growth rate check
    if [ $SAMPLE -gt 5 ] && [ "$ELAPSED_H" != "0.00" ] && [ "$ELAPSED_H" != "0" ]; then
        GROWTH_RATE=$(awk "BEGIN {printf "%.0f", $RSS_GROWTH / $ELAPSED_H}")
        if [ "$GROWTH_RATE" -gt $RSS_GROWTH_RATE_MB_PER_HR ]; then
            echo "[WARN $TS_STR] RSS growth rate ${GROWTH_RATE}MB/hr > ${RSS_GROWTH_RATE_MB_PER_HR}MB/hr (${RSS_GROWTH}MB in ${ELAPSED_H}h)" >> "$ALERT_LOG"
        fi
    fi
    
    # Progress every 20 samples
    if [ $((SAMPLE % 20)) -eq 0 ] && [ $SAMPLE -gt 0 ]; then
        echo "  [monitor sample=$SAMPLE ${ELAPSED_H}h] RSS=${RSS_MB}MB(g${RSS_GROWTH}) FD=${FD_COUNT} DISK=/tmp ${DISK_GB}GB"
    fi
    
    SAMPLE=$((SAMPLE + 1))
    sleep "$INTERVAL"
done

echo "[monitor] Exited at $(date '+%Y-%m-%dT%H:%M:%S')"
