#!/bin/bash
# monitor_soak.sh - Report SOAK test metrics every N minutes (default 10)
# Reports: QPS, RSS, threads, CPU load

INTERVAL_MIN=${1:-10}
DIR=$(ls -td test_results/wired_soak_72h_* 2>/dev/null | head -1)
[ -z "$DIR" ] && { echo "FATAL: no wired_soak_72h_* results dir found" >&2; exit 1; }

SERVER_PID=$(cat "$DIR/sqlrustgo.pid" 2>/dev/null)
[ -z "$SERVER_PID" ] && { echo "FATAL: no server PID in $DIR/sqlrustgo.pid" >&2; exit 1; }

REPORT_FILE="$DIR/monitor_reports.log"
echo "Writing reports to $REPORT_FILE every ${INTERVAL_MIN}m"

# Track start time for QPS calculation
START_TS=$(stat -f %m "$DIR/data/sqlrustgo.wal" 2>/dev/null || stat -c %Y "$DIR/data/sqlrustgo.wal" 2>/dev/null || date +%s)
LAST_TPC_COUNT=0

while true; do
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    ELAPSED=$(( $(date +%s) - START_TS ))
    ELAPSED_HMS=$(printf '%dh%02dm%02ds' $((ELAPSED/3600)) $((ELAPSED%3600/60)) $((ELAPSED%60)))

    # Check if server is alive
    if ! kill -0 "$SERVER_PID" 2>/dev/null && ! lsof -nP -p "$SERVER_PID" -iTCP -sTCP:LISTEN 2>/dev/null | grep -q 13306; then
        STATUS="❌ DEAD"
    else
        STATUS="✅ ALIVE"
    fi

    # Get RSS (KB → MB)
    RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ')
    RSS_MB=$(( RSS_KB / 1024 ))

    # Get virtual memory (MB)
    VSZ_KB=$(ps -o vsz= -p "$SERVER_PID" 2>/dev/null | tr -d ' ')
    VSZ_MB=$(( VSZ_KB / 1024 ))

    # Get thread count (macOS top)
    THREADS=$(top -l 1 -pid "$SERVER_PID" -stats th 2>/dev/null | tail -1 | awk '{print $1}')

    # Get CPU% (5 sample average)
    CPU_SAMPLES=5
    CPU_TOTAL=0
    for i in $(seq 1 $CPU_SAMPLES); do
        CPU_LINE=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ')
        # Strip decimals and accumulate integer micro-CPU
        CPU_INT=$(echo "$CPU_LINE" | awk '{printf "%d", $1*100}')
        CPU_TOTAL=$(( CPU_TOTAL + CPU_INT ))
        sleep 0.2
    done
    CPU_AVG_PCT=$(awk -v t="$CPU_TOTAL" -v n="$CPU_SAMPLES" 'BEGIN { printf "%.1f", t/n/100 }')

    # Get FD count (lsof)
    FD_COUNT=$(lsof -p "$SERVER_PID" 2>/dev/null | wc -l | tr -d ' ')

    # Get TCP LISTEN
    LISTEN_PORT=$(lsof -nP -p "$SERVER_PID" -iTCP -sTCP:LISTEN 2>/dev/null | awk 'NR>1 {print $9}' | head -1)

    # Get TPC-H query count for QPS calculation
    TPC_COUNT=$(wc -l < "$DIR/tpch_22_rotate.log" 2>/dev/null)
    TPC_NEW=$(( TPC_COUNT - LAST_TPC_COUNT ))
    LAST_TPC_COUNT=$TPC_COUNT
    if [ "$ELAPSED" -gt 0 ] && [ "$TPC_COUNT" -gt 1 ]; then
        # Each TPC round = 22 queries
        QPS=$(awk -v c="$TPC_COUNT" -v e="$ELAPSED" 'BEGIN { printf "%.2f", (c-1)*22/e }')
    else
        QPS="n/a"
    fi

    # Get WAL size
    WAL_BYTES=$(stat -f %z "$DIR/data/sqlrustgo.wal" 2>/dev/null || stat -c %s "$DIR/data/sqlrustgo.wal" 2>/dev/null)
    WAL_MB=$(awk -v b="$WAL_BYTES" 'BEGIN { printf "%.2f", b/1024/1024 }')

    # Write report
    {
        echo "=========================================================="
        echo "[$TS] Elapsed: $ELAPSED_HMS"
        echo "=========================================================="
        echo "  Server PID:      $SERVER_PID $STATUS"
        echo "  Listen:          ${LISTEN_PORT:-none}"
        echo "  QPS (TPC-H):     $QPS queries/sec"
        echo "  TPC-H queries:   $TPC_COUNT total ($TPC_NEW new since last report)"
        echo "  RSS:             ${RSS_MB} MB"
        echo "  Virtual memory:  ${VSZ_MB} MB"
        echo "  Threads:         ${THREADS:-n/a}"
        echo "  CPU load:        ${CPU_AVG_PCT}% (avg of $CPU_SAMPLES samples)"
        echo "  File descriptors: ${FD_COUNT}"
        echo "  WAL size:        ${WAL_MB} MB"
        echo ""
    } | tee -a "$REPORT_FILE"

    sleep $((INTERVAL_MIN * 60))
done
