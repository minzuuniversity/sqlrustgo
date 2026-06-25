#!/bin/bash
# run_72h_soak_v2_dispatch_168h.sh - Auto-dispatch 168h after 72h completes
# Runs run_72h_soak_v2.sh with HOURS=168 once current 72h soak ends
# Polls every 5 min for completion

set -uo pipefail

RESULTS_DIR="${RESULTS_DIR:-$(ls -dt ~/sqlrustgo-soak/soak72h_* 2>/dev/null | head -1)}"
[ -z "$RESULTS_DIR" ] && { echo "FATAL: no soak72h results dir" >&2; exit 1; }

SOAK_PID_FILE="$RESULTS_DIR/soak.pid"
EXPECTED_END_TS=$(awk -v h=72 'BEGIN{print systime() + h*3600 - 43200}')

echo "[dispatch_168h] Polling every 300s for 72h completion (PID file: $SOAK_PID_FILE)"
echo "[dispatch_168h] Will fire 168h soak ~30 min before 72h estimated end"

while true; do
    NOW=$(date +%s)
    REMAINING=$((EXPECTED_END_TS - NOW))
    if [ -f "$RESULTS_DIR/SOAK_72H_REPORT.md" ]; then
        echo "[dispatch_168h] 72h complete (report found). Firing 168h soak."
        break
    fi
    if [ $REMAINING -lt 1800 ]; then
        echo "[dispatch_168h] 72h soak within 30 min of expected end. Firing 168h."
        break
    fi
    if [ -f "$SOAK_PID_FILE" ] && kill -0 "$(cat $SOAK_PID_FILE)" 2>/dev/null; then
        echo "[dispatch_168h] $(date) - 72h still running (PID $(cat $SOAK_PID_FILE), ~${REMAINING}s remaining)"
    else
        echo "[dispatch_168h] $(date) - 72h soak process not found, assuming complete"
        break
    fi
    sleep 300
done

cd ~/sqlrustgo-soak
nohup bash run_72h_soak_v2.sh > soak-168h.log 2>&1 & disown
echo "[dispatch_168h] 168h soak dispatched. Monitor: ssh z6g4 'tail -3 ~/sqlrustgo-soak/soak-168h.log'"