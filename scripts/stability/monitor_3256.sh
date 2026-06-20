#!/bin/bash
# monitor_3256.sh - Quick status check for soak ladder task 3256
PORT=3397
RESULTS_DIR=$(ls -td /home/openclaw/dev/yinglichina163/sqlrustgo/test_results/short_soak_ladder_* 2>/dev/null | head -1)

echo "=== Task 3256 Soak Monitor — $(date '+%Y-%m-%d %H:%M:%S') ==="
echo ""

# Check status
STATUS=$(cat "$RESULTS_DIR/short_soak_ladder_STATUS.txt" 2>/dev/null || echo "N/A")
echo "Status: $STATUS"
echo ""

# Check server
SERVER_PID=$(pgrep -f "sqlrustgo.*serve.*3397" | head -1)
if [ -n "$SERVER_PID" ]; then
    RSS=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo "N/A")
    CPU=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo "N/A")
    FD=$(lsof -p "$SERVER_PID" 2>/dev/null | wc -l | tr -d ' ')
    ELAPSED=$(ps -o etime= -p "$SERVER_PID" 2>/dev/null | tr -d ' ' || echo "N/A")
    echo "Server PID=$SERVER_PID RSS=${RSS}KB CPU=${CPU}% FD=$FD Uptime=$ELAPSED"
else
    echo "Server: NOT RUNNING"
fi

# Check sysbench
SB_PID=$(pgrep -f "sysbench.*oltp.*3397" | head -1)
if [ -n "$SB_PID" ]; then
    echo "Sysbench PID=$SB_PID: RUNNING"
else
    echo "Sysbench: NOT RUNNING (may have finished)"
fi

# Show last metrics
if [ -f "$RESULTS_DIR/step_30m/metrics.csv" ]; then
    LINES=$(wc -l < "$RESULTS_DIR/step_30m/metrics.csv")
    if [ "$LINES" -gt 1 ]; then
        echo ""
        echo "Last metrics:"
        tail -1 "$RESULTS_DIR/step_30m/metrics.csv" | awk -F, '{printf "  ts=%s elapsed=%ss rss=%sMB fd=%s cpu=%s%%\n", $1, $2, $3, $4, $5}'
    fi
fi

echo ""
echo "Log: $RESULTS_DIR/short_soak_ladder.log"
echo "=========================================="
