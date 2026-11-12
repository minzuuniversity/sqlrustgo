#!/usr/bin/env bash
# AB-07 real 1h soak wrapper: soak script + heartbeat emitter on the REAL
# driver pid + fail-closed heartbeat check at closure. All artifacts under /tmp.
set -uo pipefail
export PATH="/Volumes/workspace/dev/srg-soak-venv/bin:$PATH"
TS="$(date +%Y%m%d_%H%M%S)"
RUNLOG="/tmp/srg-ab003-soak-run-$TS.log"
HB="/tmp/srg-ab003-soak-hb-$TS.jsonl"
export SQLRUSTGO_BIN=/Volumes/workspace/dev/walgc2-target/debug/sqlrustgo-mysql-server
export RESULTS_DIR=/tmp/srg-ab003-soak-$TS
export DATA_DIR=/tmp/sqlrustgo-v400-soak-ab003
cd /Volumes/workspace/dev/sqlrustgo
bash scripts/soak/v400_1h_soak.sh 60 3412 > "$RUNLOG" 2>&1 &
SPID=$!
echo "SOAK_TID=$TS driver_shell_pid=$SPID results=$RESULTS_DIR"
python3 /Volumes/workspace/dev/srg-ab07/scripts/soak/soak_heartbeat.py --pid "$SPID" --out "$HB" --interval 15 --log "$RUNLOG"
wait "$SPID"; SRC=$?
echo "SOAK_RC=$SRC"
python3 /Volumes/workspace/dev/srg-ab07/scripts/soak/check_soak_heartbeat.py --file "$HB" --max-gap 90
echo "HB_RC=$?"
grep -c "^" "$RESULTS_DIR/metrics.csv" 2>/dev/null | sed 's/^/metrics_rows=/'
grep -m1 "FATAL" "$RESULTS_DIR/driver.stdout" 2>/dev/null && echo "DRIVER-FATAL-DETECTED"
tail -6 "$RUNLOG"
