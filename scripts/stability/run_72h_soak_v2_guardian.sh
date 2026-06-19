#!/bin/bash
# run_72h_soak_v2_guardian.sh - Watchdog that restarts crashed soak processes
# Runs every 60s, restarts server or soak if either dies, logs to guardian.log
# Used in conjunction with run_72h_soak_v2.sh

set -uo pipefail

RESULTS_DIR="${RESULTS_DIR:-$(ls -dt ~/sqlrustgo-soak/soak72h_* 2>/dev/null | head -1)}"
[ -z "$RESULTS_DIR" ] && { echo "FATAL: no soak72h results dir found" >&2; exit 1; }

SERVER_BIN="$HOME/sqlrustgo-soak/bin/sqlrustgo-mysql-server"
GUARDIAN_LOG="$RESULTS_DIR/guardian.log"
RESTART_COUNT_FILE="$RESULTS_DIR/restart_count"
MAX_RESTARTS=${MAX_RESTARTS:-5}
CHECK_INTERVAL_S=${CHECK_INTERVAL_S:-60}

log() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] GUARDIAN: $*" | tee -a "$GUARDIAN_LOG"; }
alert() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] GUARDIAN ALERT: $*" | tee -a "$GUARDIAN_LOG" >&2; }

[ -f "$RESTART_COUNT_FILE" ] || echo 0 > "$RESTART_COUNT_FILE"
RESTART_COUNT=$(cat "$RESTART_COUNT_FILE")
log "Watchdog started, max_restarts=$MAX_RESTARTS, results=$RESULTS_DIR"

while [ "$RESTART_COUNT" -lt "$MAX_RESTARTS" ]; do
    sleep "$CHECK_INTERVAL_S"
    SERVER_PID=$(cat "$RESULTS_DIR/server.pid" 2>/dev/null)
    SOAK_PID=$(cat "$RESULTS_DIR/soak.pid" 2>/dev/null)
    SERVER_DEAD=0; SOAK_DEAD=0
    [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null || SERVER_DEAD=1
    [ -n "$SOAK_PID" ] && kill -0 "$SOAK_PID" 2>/dev/null || SOAK_DEAD=1
    if [ $SERVER_DEAD -eq 0 ] && [ $SOAK_DEAD -eq 0 ]; then
        log "OK: server=$SERVER_PID soak=$SOAK_PID"
        continue
    fi
    RESTART_COUNT=$((RESTART_COUNT+1))
    echo "$RESTART_COUNT" > "$RESTART_COUNT_FILE"
    if [ $SERVER_DEAD -eq 1 ]; then
        alert "Server died (pid $SERVER_PID), restart #$RESTART_COUNT"
        tail -30 "$RESULTS_DIR/server.log" >> "$GUARDIAN_LOG"
        nohup "$SERVER_BIN" serve --port 13306 --data-dir "$RESULTS_DIR/data" >> "$RESULTS_DIR/server.log" 2>&1 &
        echo $! > "$RESULTS_DIR/server.pid"
        sleep 5
    fi
    if [ $SOAK_DEAD -eq 1 ]; then
        alert "Soak died (pid $SOAK_PID), restart #$RESTART_COUNT"
        tail -30 "$RESULTS_DIR/soak.log" >> "$GUARDIAN_LOG"
        nohup "$SERVER_BIN" soak --duration 72 --qps 1.0 \
            --output "$RESULTS_DIR/soak.jsonl" \
            --sample-interval-s 60 >> "$RESULTS_DIR/soak.log" 2>&1 &
        echo $! > "$RESULTS_DIR/soak.pid"
        sleep 5
    fi
    log "Restarted: server=$(cat $RESULTS_DIR/server.pid) soak=$(cat $RESULTS_DIR/soak.pid)"
done

alert "Max restarts ($MAX_RESTARTS) exceeded - giving up"
echo "GUARDIAN_FAILED" > "$RESULTS_DIR/guardian_status"
exit 1