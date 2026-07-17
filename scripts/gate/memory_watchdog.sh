#!/bin/bash
# Memory watchdog - monitors memory and kills process if OOM threshold exceeded
# Usage: ./memory_watchdog.sh <pid> <max_mem_mb> <interval_sec>

PID=$1
MAX_MEM_MB=${2:-4096}
INTERVAL=${3:-5}

if [ -z "$PID" ]; then
    echo "Usage: $0 <pid> <max_mem_mb> [interval_sec]"
    exit 1
fi

echo "[WATCHDOG] Monitoring PID $PID (max=${MAX_MEM_MB}MB, interval=${INTERVAL}s)"

while true; do
    # Check if process still running
    if ! kill -0 $PID 2>/dev/null; then
        echo "[WATCHDOG] Process $PID exited"
        exit 0
    fi
    
    # Get memory usage (RSS in KB)
    MEM_KB=$(ps -o rss= -p $PID 2>/dev/null || echo 0)
    MEM_MB=$((MEM_KB / 1024))
    
    echo "[WATCHDOG] PID $PID: ${MEM_MB}MB / ${MAX_MEM_MB}MB"
    
    if [ "$MEM_MB" -gt "$MAX_MEM_MB" ]; then
        echo "[WATCHDOG] OOM: ${MEM_MB}MB > ${MAX_MEM_MB}MB - killing PID $PID"
        kill -9 $PID 2>/dev/null
        exit 137  # 128 + 9 = SIGKILL
    fi
    
    sleep $INTERVAL
done