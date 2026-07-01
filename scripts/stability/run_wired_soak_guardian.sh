#!/bin/bash
# run_wired_soak_guardian.sh - Watchdog for wired SOAK
# Runs every 60s, restarts crashed server/TPC-H rotate if either dies, logs to guardian.log
# Used with run_wired_soak.sh (wired mode via MySQL protocol)

set -uo pipefail

RESULTS_DIR="${RESULTS_DIR:-$(ls -dt test_results/wired_soak_* 2>/dev/null | head -1)}"
[ -z "$RESULTS_DIR" ] && { echo "FATAL: no wired_soak results dir found" >&2; exit 1; }

GUARDIAN_LOG="$RESULTS_DIR/guardian.log"
RESTART_COUNT_FILE="$RESULTS_DIR/restart_count"
MAX_RESTARTS=${MAX_RESTARTS:-10}
CHECK_INTERVAL_S=${CHECK_INTERVAL_S:-60}
PORT="${PORT:-13306}"

log() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] GUARDIAN: $*" | tee -a "$GUARDIAN_LOG"; }
alert() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] GUARDIAN ALERT: $*" | tee -a "$GUARDIAN_LOG" >&2; }

[ -f "$RESTART_COUNT_FILE" ] || echo 0 > "$RESTART_COUNT_FILE"
RESTART_COUNT=$(cat "$RESTART_COUNT_FILE")
log "Wired SOAK Guardian started, max_restarts=$MAX_RESTARTS, results=$RESULTS_DIR"

while [ "$RESTART_COUNT" -lt "$MAX_RESTARTS" ]; do
    # Check for STABILITY_REPORT.md (natural completion)
    if [ -f "$RESULTS_DIR/STABILITY_REPORT.md" ]; then
        log "Natural completion detected (STABILITY_REPORT.md present), exiting cleanly"
        echo "GUARDIAN_OK" > "$RESULTS_DIR/guardian_status"
        exit 0
    fi
    
    sleep "$CHECK_INTERVAL_S"
    
    SERVER_PID=$(cat "$RESULTS_DIR/sqlrustgo.pid" 2>/dev/null)
    TPCH_PID=$(cat "$RESULTS_DIR/tpch_rotate.pid" 2>/dev/null)
    SERVER_DEAD=0; TPCH_DEAD=0
    
    [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null || SERVER_DEAD=1
    [ -n "$TPCH_PID" ] && kill -0 "$TPCH_PID" 2>/dev/null || TPCH_DEAD=1
    
    if [ $SERVER_DEAD -eq 0 ] && [ $TPCH_DEAD -eq 0 ]; then
        log "OK: server=$SERVER_PID tpch=$TPCH_PID"
        continue
    fi
    
    RESTART_COUNT=$((RESTART_COUNT+1))
    echo "$RESTART_COUNT" > "$RESTART_COUNT_FILE"
    
    if [ $SERVER_DEAD -eq 1 ]; then
        alert "Server died (pid $SERVER_PID), restart #$RESTART_COUNT"
        tail -30 "$RESULTS_DIR/sqlrustgo.log" >> "$GUARDIAN_LOG" 2>/dev/null || true
        # Kill stale TPCH process too
        [ -n "$TPCH_PID" ] && kill "$TPCH_PID" 2>/dev/null || true
        # Restart server
        nohup ./target/release/sqlrustgo-mysql-server serve \
            --host 127.0.0.1 --port $PORT \
            --data-dir "$RESULTS_DIR/data" \
            --log-level info \
            --server-threads 16 \
            >> "$RESULTS_DIR/sqlrustgo.log" 2>&1 &
        echo $! > "$RESULTS_DIR/sqlrustgo.pid"
        sleep 5
        # Restart TPCH rotate
        nohup bash scripts/stability/tpch_22_rotate.sh \
            HOST=127.0.0.1 PORT=$PORT \
            INTERVAL=120 MAX_ROUNDS=0 \
            LOG_FILE="$RESULTS_DIR/tpch_22_rotate.log" \
            >> "$RESULTS_DIR/tpch_rotate.stdout" 2>&1 &
        echo $! > "$RESULTS_DIR/tpch_rotate.pid"
        sleep 2
    fi
    
    log "Restarted: server=$(cat $RESULTS_DIR/sqlrustgo.pid) tpch=$(cat $RESULTS_DIR/tpch_rotate.pid)"
done

alert "Max restarts ($MAX_RESTARTS) exceeded - giving up"
echo "GUARDIAN_FAILED" > "$RESULTS_DIR/guardian_status"
exit 1
