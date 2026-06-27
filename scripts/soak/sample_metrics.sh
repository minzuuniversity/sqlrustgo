#!/usr/bin/env bash
# sample_metrics.sh — Background procfs sampler for soak tests
# Usage: bash sample_metrics.sh --pid PID --output FILE --interval SECONDS
#
# Samples RSS and FD count from /proc at the given interval.
# Outputs CSV: timestamp_unix,rss_kb,fd_count
# Stops on SIGTERM.

set -euo pipefail

PID=""
OUTPUT=""
INTERVAL=30

while [[ $# -gt 0 ]]; do
    case "$1" in
        --pid)   PID="$2";   shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        --interval) INTERVAL="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ -z "$PID" || -z "$OUTPUT" ]]; then
    echo "Usage: $0 --pid PID --output FILE --interval SECONDS"
    exit 1
fi

# Header
echo "timestamp_unix,rss_kb,fd_count" > "$OUTPUT"

cleanup() {
    echo "Sampling stopped (PID=$PID, interval=${INTERVAL}s)"
    exit 0
}
trap cleanup SIGTERM EXIT

echo "Sampling PID=$PID every ${INTERVAL}s → $OUTPUT"

while true; do
    if ! kill -0 "$PID" 2>/dev/null; then
        echo "Process $PID died, stopping sampling"
        break
    fi

    ts=$(date +%s)
    rss_kb=0
    fd_count=0

    # Read RSS from /proc/PID/status
    if [[ -f "/proc/$PID/status" ]]; then
        rss_kb=$(grep '^VmRSS:' "/proc/$PID/status" 2>/dev/null | awk '{print $2}' || echo "0")
    fi

    # Count FD
    if [[ -d "/proc/$PID/fd" ]]; then
        fd_count=$(ls -1 "/proc/$PID/fd" 2>/dev/null | wc -l || echo "0")
    fi

    echo "$ts,$rss_kb,$fd_count" >> "$OUTPUT"
    sleep "$INTERVAL"
done
