#!/usr/bin/env bash
# pymysql_chained_lowprio.sh — Auto-chain pymysql wired SOAK with low CPU/IO priority.
#
# Waits for an active SOAK server PID to exit, then launches a series of
# longer pymysql soaks on a fresh data dir copy, with each server/driver
# run inside `nice -n 19` and `ionice -c idle` to avoid starving the user.
#
# Use case: keep the SOAK testing pipeline progressing 24/7 without
# pushing the host's load average past responsive limits.
#
# Usage:
#   bash scripts/soak/pymysql_chained_lowprio.sh [DURATIONS_MIN=...] [WAIT_PID=...]
#
# Env (overridable):
#   DURATIONS_MIN   space-separated minutes per stage (default: "120 240")
#   WAIT_PID        server PID to wait for before starting (default: pid
#                   stored in soak_results/pymysql-60min-*/server.pid)
#   START_PORT      first port (default: 3398, leaves 3396/3397 free)
#   SOURCE_DATA_DIR TPC-H binary dataset (default: data/tpch-sf01-bin)
#   SAMPLE_INTERVAL monitor interval seconds (default: 30)
#   THREADS         client threads (default: 16)
#   SERVER_THREADS  server worker threads (default: 16)
#
# Output: one pymysql-{N}min-<ts>/ directory per stage under soak_results/.
# Logs:  per-stage launcher.log next to STABILITY_REPORT.md.

set -uo pipefail

DURATIONS_MIN="${DURATIONS_MIN:-120 240}"
START_PORT="${START_PORT:-3398}"
SOURCE_DATA_DIR="${SOURCE_DATA_DIR:-$PWD/data/tpch-sf01-bin}"
SAMPLE_INTERVAL="${SAMPLE_INTERVAL:-30}"
THREADS="${THREADS:-16}"
SERVER_THREADS="${SERVER_THREADS:-16}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-$PWD/target/release/sqlrustgo-mysql-server}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# --- discover wait pid if not provided ---
if [ -z "${WAIT_PID:-}" ]; then
  WAIT_PID=$(ls -t soak_results/pymysql-*/server.pid 2>/dev/null | head -1)
  if [ -n "$WAIT_PID" ] && [ -f "$WAIT_PID" ]; then
    WAIT_PID=$(cat "$WAIT_PID")
  else
    WAIT_PID=""
  fi
fi

log() { echo "[$(date '+%Y-%m-%dT%H:%M:%S')] $*"; }

log "=================================================="
log "Chained Low-Priority pymysql SOAK"
log "  DURATIONS_MIN=$DURATIONS_MIN"
log "  START_PORT=$START_PORT"
log "  SOURCE_DATA_DIR=$SOURCE_DATA_DIR"
log "  THREADS=$THREADS  SERVER_THREADS=$SERVER_THREADS"
log "  WAIT_PID=${WAIT_PID:-<none>}"
log "=================================================="

# --- preconditions ---
[ -x "$SQLRUSTGO_BIN" ] || { log "FAIL: $SQLRUSTGO_BIN missing"; exit 1; }
[ -d "$SOURCE_DATA_DIR" ] || { log "FAIL: $SOURCE_DATA_DIR missing"; exit 1; }
python3 -c 'import pymysql' 2>/dev/null || {
  pip3 install --user pymysql 2>&1 | tail -3
}

# --- wait for prior server to exit ---
if [ -n "$WAIT_PID" ] && kill -0 "$WAIT_PID" 2>/dev/null; then
  log "Waiting for prior server PID=$WAIT_PID to exit..."
  while kill -0 "$WAIT_PID" 2>/dev/null; do sleep 10; done
  log "  prior server exited"
fi

# --- run each stage ---
PORT="$START_PORT"
for DUR in $DURATIONS_MIN; do
  if [ "$DUR" -lt 1 ]; then
    log "skip invalid duration: $DUR"
    continue
  fi
  if [ "$DUR" -lt 60 ]; then
    STAGE_LABEL="${DUR}min"
  else
    HOURS=$(awk -v d="$DUR" 'BEGIN{printf "%dh%dm", int(d/60), d%60}')
    STAGE_LABEL="${DUR}min"
  fi
  TS=$(date +%Y%m%d_%H%M%S)
  STAGE_DIR="soak_results/pymysql-${DUR}min-${TS}"
  STAGE_DATA="$STAGE_DIR/data"
  STAGE_LOG="$STAGE_DIR/launcher.log"
  mkdir -p "$STAGE_DIR"

  log "----- stage: DURATION_MIN=$DUR  PORT=$PORT  STAGE_DIR=$STAGE_DIR -----"
  log "  Copying data dir (one-time, ~$(du -sh "$SOURCE_DATA_DIR" 2>/dev/null | awk '{print $1}'))..."
  cp -r "$SOURCE_DATA_DIR" "$STAGE_DATA" 2>&1 | tail -2
  chmod -R u+w "$STAGE_DATA"

  # Re-export env so the soak script picks up our per-stage DATA_DIR
  export DATA_DIR="$STAGE_DATA"
  export RESULTS_DIR="$STAGE_DIR"
  export THREADS
  export SERVER_THREADS
  export SAMPLE_INTERVAL

  # Wrap the soak script with low-priority so the user shell is never starved
  (
    # Apply ionice idle + nice 19 to every server/driver child
    nice -n 19 ionice -c 3 bash "$SCRIPT_DIR/pymysql_30min_soak.sh" "$DUR" "$PORT"
  ) > "$STAGE_LOG" 2>&1 &
  WRAPPER_PID=$!
  log "  wrapper PID=$WRAPPER_PID (stage log: $STAGE_LOG)"

  # wait for this stage's server to start, then renice/ionice it explicitly
  # (defensive: the inner script spawns the server with `$BIN serve &`)
  STAGE_SERVER_PID=""
  for _ in $(seq 1 30); do
    sleep 2
    STAGE_SERVER_PID=$(ss -tlnp 2>/dev/null | awk -v p=":$PORT " '$4 ~ p {print $0}' | grep -oP 'pid=\K[0-9]+' | head -1)
    if [ -n "$STAGE_SERVER_PID" ]; then break; fi
  done
  if [ -n "$STAGE_SERVER_PID" ] && kill -0 "$STAGE_SERVER_PID" 2>/dev/null; then
    renice -n 19 -p "$STAGE_SERVER_PID" 2>/dev/null || true
    ionice -c 3 -p "$STAGE_SERVER_PID" 2>/dev/null || true
    log "  applied nice 19 + ionice idle to stage server PID=$STAGE_SERVER_PID"
  fi

  # wait for the stage to complete (wrapper exits when soak script exits)
  wait "$WRAPPER_PID"
  STAGE_RC=$?
  log "  stage complete rc=$STAGE_RC; report: $STAGE_DIR/STABILITY_REPORT.md"
  if [ "$STAGE_RC" -ne 0 ]; then
    log "  stage FAILED — aborting chain"
    exit "$STAGE_RC"
  fi
  PORT=$((PORT + 1))
done

log "All stages PASSED."
