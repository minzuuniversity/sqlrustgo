#!/bin/bash
# monitor_disk_io.sh - Disk I/O monitoring for SOAK
# Reports: data file size deltas + process I/O stats
#
# Usage:
#   nohup bash monitor_disk_io.sh 5 > monitor_disk_io.log 2>&1 &
#   INTERVAL_MIN (default 5)

INTERVAL_MIN=${1:-5}

DIR=$(ls -td test_results/wired_soak_72h_* 2>/dev/null | head -1)
[ -z "$DIR" ] && { echo "FATAL: no wired_soak_72h_* results dir found" >&2; exit 1; }

SERVER_PID=$(cat "$DIR/sqlrustgo.pid" 2>/dev/null)
[ -z "$SERVER_PID" ] && { echo "FATAL: no server PID" >&2; exit 1; }

# Find the real server PID via lsof
REAL_PID=$(lsof -ti :$(lsof -p $SERVER_PID -iTCP -sTCP:LISTEN 2>/dev/null | grep TCP | awk '{print $9}' | awk -F: '{print $NF}') 2>/dev/null | head -1)
[ -z "$REAL_PID" ] && REAL_PID=$SERVER_PID

REPORT_FILE="$DIR/disk_io_reports.log"

# Track previous values for deltas
PREV_READ_BYTES=0
PREV_WRITE_BYTES=0
PREV_WAL_SIZE=0
PREV_JSON_SIZE=0

echo "Writing disk I/O reports to $REPORT_FILE every ${INTERVAL_MIN}m" | tee -a "$REPORT_FILE"

while true; do
    TS=$(date '+%Y-%m-%d %H:%M:%S')

    # Disk usage for entire server data directory
    DATA_USAGE=$(du -sh "$DIR/data/" 2>/dev/null | awk '{print $1}')

    # WAL size
    WAL_BYTES=$(stat -f %z "$DIR/data/sqlrustgo.wal" 2>/dev/null || stat -c %s "$DIR/data/sqlrustgo.wal" 2>/dev/null)
    WAL_DELTA=$((WAL_BYTES - PREV_WAL_SIZE))
    [ "$WAL_DELTA" -lt 0 ] && WAL_DELTA=0
    PREV_WAL_SIZE=$WAL_BYTES

    # Total JSON tables size (sum of *.json)
    JSON_BYTES=$(find "$DIR/data/" -name "*.json" -exec stat -f %z {} \; 2>/dev/null | awk '{sum+=$1} END {print sum+0}')
    JSON_DELTA=$((JSON_BYTES - PREV_JSON_SIZE))
    [ "$JSON_DELTA" -lt 0 ] && JSON_DELTA=0
    PREV_JSON_SIZE=$JSON_BYTES

    # Process I/O stats (macOS: read_bytes/write_bytes not in ps; use proxy)
    # Approximate via open file count
    OPEN_FILES=$(lsof -p $REAL_PID 2>/dev/null | wc -l | tr -d ' ')

    # Buffer cache stats
    CACHE_HIT=$(sysctl -n vm.stat.cache_hits 2>/dev/null || echo "0")
    CACHE_MISS=$(sysctl -n vm.stat.cache_misses 2>/dev/null || echo "0")
    if [ "$CACHE_HIT" -gt 0 ] && [ "$CACHE_MISS" -gt 0 ]; then
        HIT_RATE=$(awk -v h="$CACHE_HIT" -v m="$CACHE_MISS" 'BEGIN { printf "%.1f", h/(h+m)*100 }')
    else
        HIT_RATE="n/a"
    fi

    # Per-FD lsof of data dir only
    DATA_FDS=$(lsof -p $REAL_PID 2>/dev/null | grep "$DIR/data" | wc -l | tr -d ' ')

    {
        echo "=========================================================="
        echo "[$TS] Disk I/O Report"
        echo "=========================================================="
        echo "  Data dir total:    ${DATA_USAGE}"
        echo "  WAL file:          ${WAL_BYTES} bytes (+${WAL_DELTA} delta)"
        echo "  JSON tables total: ${JSON_BYTES} bytes (+${JSON_DELTA} delta)"
        echo "  Server open files: $OPEN_FILES (data dir: $DATA_FDS)"
        echo "  Buffer cache hit:  ${HIT_RATE}%"
        echo ""
    } | tee -a "$REPORT_FILE"

    sleep $((INTERVAL_MIN * 60))
done
