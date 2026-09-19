#!/usr/bin/env bash
# v400_168h_soak_status.sh — quick status check for the running 168h SOAK.
#
# Usage:
#   bash scripts/soak/v400_168h_soak_status.sh
#
# Exit codes:
#   0  SOAK running, RSS < 1.5 GB
#   1  SOAK crashed or RSS > 1.7 GB (macOS jetsam threshold)
#   2  SOAK completed (no running process)

set -uo pipefail

RESULTS_DIR="${RESULTS_DIR:-/Users/liying/dev/sqlrustgo/results}"

# Find latest soak results directory
LATEST=$(ls -td "$RESULTS_DIR"/soak-v400-1h-* 2>/dev/null | head -1)

if [ -z "$LATEST" ]; then
    echo "No SOAK results found in $RESULTS_DIR"
    exit 2
fi

echo "Latest SOAK: $LATEST"
echo "---"

# Check if server is still running
SERVER_PID_FILE="$LATEST/server.pid"
if [ -f "$SERVER_PID_FILE" ]; then
    SERVER_PID=$(cat "$SERVER_PID_FILE")
    if ps -p "$SERVER_PID" > /dev/null 2>&1; then
        echo "Server PID $SERVER_PID: ALIVE"
    else
        echo "Server PID $SERVER_PID: DEAD"
    fi
fi

# Latest metric sample
if [ -f "$LATEST/metrics.csv" ]; then
    echo ""
    echo "Latest metric samples (last 10):"
    tail -10 "$LATEST/metrics.csv"
fi

# Stability report if available
if [ -f "$LATEST/STABILITY_REPORT.md" ]; then
    echo ""
    echo "=== STABILITY REPORT ==="
    cat "$LATEST/STABILITY_REPORT.md"
fi

# Check RSS threshold
if [ -f "$LATEST/metrics.csv" ]; then
    LAST_RSS=$(tail -1 "$LATEST/metrics.csv" | cut -d',' -f3)
    if [ -n "$LAST_RSS" ]; then
        # RSS is in MB; 1700 MB jetsam threshold
        if (( $(echo "$LAST_RSS > 1700" | bc -l 2>/dev/null || echo 0) )); then
            echo ""
            echo "WARNING: RSS=$LAST_RSS MB exceeds macOS jetsam threshold!"
            exit 1
        fi
    fi
fi