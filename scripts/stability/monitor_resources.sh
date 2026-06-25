#!/bin/bash
# Z6G4 Resource Monitor — runs during soak test on Z6G4
# Monitors: memory (RSS/virt), disk, CPU, file descriptors, process status
# Auto-alerts on threshold breach
#
# Usage: bash scripts/stability/monitor_resources.sh [PID_FILE] [RESULTS_DIR] [INTERVAL=60]
#
# Thresholds:
#   RSS > 4096 MB → ALERT
#   RSS growth > 200 MB/hour → ALERT  
#   Disk /tmp < 10 GB → ALERT
#   FD > 5000 per process → ALERT
#   Server process dead → CRITICAL
#
# Output:
#   $RESULTS_DIR/resource_monitor.log  — timestamped resource snapshots
#   $RESULTS_DIR/resource_alerts.log   — threshold breach events

set -euo pipefail

PID_FILE="${1:-}"
RESULTS_DIR="${2:-test_results/soak_monitor}"
INTERVAL="${3:-60}"
ALERT_LOG="$RESULTS_DIR/resource_alerts.log"
MONITOR_LOG="$RESULTS_DIR/resource_monitor.log"
SERVER_PID=""

mkdir -p "$RESULTS_DIR"

# Threshold constants
RSS_ALERT_MB=4096
RSS_GROWTH_RATE_MB_PER_HR=200
DISK_MIN_GB=10
FD_ALERT=5000

# Baseline tracking
declare -A BASELINE_RSS
declare -A BASELINE_TS
INITIAL_RSS=0
INITIAL_TS=0

init_baseline() {
    if [ -n "$PID_FILE" ] && [ -f "$PID_FILE" ]; then
        SERVER_PID=$(cat "$PID_FILE" 2>/dev/null || echo "")
    fi
    if [ -z "$SERVER_PID" ]; then
        SERVER_PID=$(pgrep -f "sqlrustgo-mysql-server.*serve" | head -1 || echo "")
    fi
    if [ -n "$SERVER_PID" ] && [ -d "/proc/$SERVER_PID" ]; then
        INITIAL_RSS=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo 0)
        INITIAL_TS=$(date +%s)
        BASELINE_RSS[$SERVER_PID]=$INITIAL_RSS
        BASELINE_TS[$SERVER_PID]=$INITIAL_TS
    fi
}

check_server_alive() {
    if [ -z "$SERVER_PID" ]; then
        SERVER_PID=$(pgrep -f "sqlrustgo-mysql-server.*serve" | head -1 || echo "")
    fi
    if [ -z "$SERVER_PID" ] || [ ! -d "/proc/$SERVER_PID" ]; then
        echo "[CRITICAL $(date '+%Y-%m-%dT%H:%M:%SZ')] Server process dead (PID=$SERVER_PID)" >> "$ALERT_LOG"
        return 1
    fi
    return 0
}

get_rss_mb() {
    local pid=$1
    if [ -d "/proc/$pid" ]; then
        ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ' || echo 0
    else
        echo 0
    fi
}

get_virt_mb() {
    local pid=$1
    if [ -d "/proc/$pid" ]; then
        ps -o vsz= -p "$pid" 2>/dev/null | tr -d ' ' || echo 0
    else
        echo 0
    fi
}

get_fd_count() {
    local pid=$1
    if [ -d "/proc/$pid/fd" ]; then
        ls /proc/$pid/fd 2>/dev/null | wc -l || echo 0
    else
        echo 0
    fi
}

get_cpu_pct() {
    local pid=$1
    if [ -d "/proc/$pid" ]; then
        ps -o %cpu= -p "$pid" 2>/dev/null | tr -d ' ' || echo 0
    else
        echo 0
    fi
}

get_disk_free_gb() {
    df -BG /tmp 2>/dev/null | awk 'NR==2 {print $4}' | tr -d 'G' || echo 0
}

check_thresholds() {
    local pid=$1
    local rss_mb=$2
    local fd_count=$3
    local cpu_pct=$4
    local disk_gb=$5
    local ts=$6
    
    # Server dead
    if [ -z "$pid" ] || [ ! -d "/proc/$pid" ]; then
        echo "[CRITICAL $ts] Server PID $pid not found — CRASH DETECTED"
        return 1
    fi
    
    # RSS absolute
    if [ "$rss_mb" -gt $RSS_ALERT_MB ]; then
        echo "[ALERT $ts] RSS ${rss_mb}MB exceeds ${RSS_ALERT_MB}MB limit"
    fi
    
    # RSS growth rate (MB/hour)
    local baseline_rss=${BASELINE_RSS[$pid]:-$INITIAL_RSS}
    local baseline_ts=${BASELINE_TS[$pid]:-$INITIAL_TS}
    if [ "$baseline_rss" -gt 0 ] && [ "$baseline_ts" -gt 0 ]; then
        local elapsed_hr=$(awk "BEGIN {printf "%.2f", ($ts - $baseline_ts) / 3600}")
        local growth_mb=$((rss_mb - baseline_rss))
        if [ "$elapsed_hr" > 0 ]; then
            local growth_rate=$(awk "BEGIN {printf "%.0f", $growth_mb / $elapsed_hr}")
            if [ "$growth_rate" -gt $RSS_GROWTH_RATE_MB_PER_HR ]; then
                echo "[ALERT $ts] RSS growth rate ${growth_rate}MB/hr exceeds ${RSS_GROWTH_RATE_MB_PER_HR}MB/hr limit (${growth_mb}MB in ${elapsed_hr}h)"
            fi
        fi
    fi
    
    # FD count
    if [ "$fd_count" -gt $FD_ALERT ]; then
        echo "[ALERT $ts] FD count ${fd_count} exceeds ${FD_ALERT}"
    fi
    
    # Disk space
    if [ "$disk_gb" -lt $DISK_MIN_GB ]; then
        echo "[ALERT $ts] Disk /tmp only ${disk_gb}GB free (min ${DISK_MIN_GB}GB)"
    fi
    
    return 0
}

# ---- Main monitoring loop ----
echo "=========================================="
echo "Z6G4 Resource Monitor"
echo "PID_FILE: $PID_FILE"
echo "RESULTS_DIR: $RESULTS_DIR"
echo "INTERVAL: ${INTERVAL}s"
echo "=========================================="

init_baseline

# CSV header for monitor log
echo "ts,elapsed_h,rss_mb,rss_growth_mb,fd_count,cpu_pct,disk_free_gb,server_alive" > "$MONITOR_LOG"

START_TS=$(date +%s)
SAMPLE=0

while true; do
    TS=$(date +%s)
    TS_STR=$(date '+%Y-%m-%dT%H:%M:%SZ')
    ELAPSED_H=$(awk "BEGIN {printf "%.2f", ($TS - $START_TS) / 3600}")
    
    # Find server PID if not set
    if [ -z "$SERVER_PID" ]; then
        SERVER_PID=$(pgrep -f "sqlrustgo-mysql-server.*serve" | head -1 || echo "")
    fi
    
    RSS_MB=0
    FD_COUNT=0
    CPU_PCT=0
    DISK_GB=0
    ALIVE=0
    
    if [ -n "$SERVER_PID" ] && [ -d "/proc/$SERVER_PID" ]; then
        RSS_MB=$(get_rss_mb "$SERVER_PID")
        FD_COUNT=$(get_fd_count "$SERVER_PID")
        CPU_PCT=$(get_cpu_pct "$SERVER_PID")
        ALIVE=1
        
        # Update baseline on first sample
        if [ $SAMPLE -eq 0 ]; then
            BASELINE_RSS[$SERVER_PID]=$RSS_MB
            BASELINE_TS[$SERVER_PID]=$TS
        fi
    fi
    
    DISK_GB=$(get_disk_free_gb)
    
    # Log snapshot
    RSS_GROWTH=$((RSS_MB - ${BASELINE_RSS[$SERVER_PID]:-0}))
    echo "$TS_STR,$ELAPSED_H,$RSS_MB,$RSS_GROWTH,$FD_COUNT,$CPU_PCT,$DISK_GB,$ALIVE" >> "$MONITOR_LOG"
    
    # Check thresholds and alert
    check_thresholds "$SERVER_PID" "$RSS_MB" "$FD_COUNT" "$CPU_PCT" "$DISK_GB" "$TS_STR" >> "$ALERT_LOG" 2>&1 || true
    
    # Progress output every 10 samples
    if [ $((SAMPLE % 10)) -eq 0 ]; then
        echo "[$(date '+%H:%M:%S') sample=$SAMPLE elapsed=${ELAPSED_H}h] RSS=${RSS_MB}MB (g${RSS_GROWTH}) FD=${FD_COUNT} CPU=${CPU_PCT}% DISK=/tmp ${DISK_GB}GB free"
    fi
    
    # Server dead → stop monitoring
    if [ "$ALIVE" -eq 0 ]; then
        echo "[CRITICAL] Server dead at sample $SAMPLE. Stopping monitor."
        echo "[CRITICAL $TS_STR] Monitor stopped — server dead" >> "$ALERT_LOG"
        break
    fi
    
    SAMPLE=$((SAMPLE + 1))
    sleep "$INTERVAL"
done
