#!/usr/bin/env bash
# repro_4214_mirror.sh — Mirror PR #4214's setup exactly: use sysbench oltp_insert prepare
# with secondary index, then run oltp_write_only + oltp_read_write with both
# --db-ps-mode=disable AND --db-ps-mode=auto to test both protocol modes.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

TIME_SEC="${TIME_SEC:-10}"
THREADS="${THREADS:-2}"
TABLE_SIZE="${TABLE_SIZE:-100}"
EVIDENCE_ROOT="$REPO_ROOT/docs/releases/v3.12.0/evidence/issue-4019"
RUN_ID="repro4214mirror_$(date -u +%Y%m%dT%H%M%SZ)_t${THREADS}_s${TABLE_SIZE}"
RUN_DIR="$EVIDENCE_ROOT/$RUN_ID"
SERVER_BIN="${SQLRUSTGO_SERVER_BIN:-$REPO_ROOT/target/debug/sqlrustgo-mysql-server}"
HOST="${HOST:-127.0.0.1}"
USER="${USER:-root}"

PREFERRED_PORT="${PREFERRED_PORT:-23500}"
PORT="$PREFERRED_PORT"
for _ in $(seq 1 50); do
    if ! (echo > "/dev/tcp/127.0.0.1/$PORT") >/dev/null 2>&1; then
        break
    fi
    PORT=$((PORT+1))
done

mkdir -p "$RUN_DIR"
log() { echo -e "\033[0;32m[repro4214mirror]\033[0m $*" | tee -a "$RUN_DIR/run.log" >&2; }
err() { echo -e "\033[0;31m[repro4214mirror:ERR]\033[0m $*" | tee -a "$RUN_DIR/run.log" >&2; }

DB="repro4214mirror_$$"

cleanup() {
    if [ -n "${SERVER_PID:-}" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
    fi
    mysql -h "$HOST" -P "$PORT" -u "$USER" \
        -e "DROP DATABASE IF EXISTS ${DB};" >/dev/null 2>&1 || true
}
trap cleanup EXIT

log "starting sqlrustgo-mysql-server on $HOST:$PORT"
DATA_DIR="$RUN_DIR/data"
mkdir -p "$DATA_DIR"
"$SERVER_BIN" serve \
    --host "$HOST" --port "$PORT" \
    --data-dir "$DATA_DIR" \
    --auth-mode none \
    --max-connections 32 \
    --server-threads 8 \
    --storage file \
    --wal-sync every \
    --log-level info \
    > "$RUN_DIR/server.log" 2>&1 &
SERVER_PID=$!

deadline=$((SECONDS + 30))
while [ "$SECONDS" -lt "$deadline" ]; do
    if (echo > "/dev/tcp/127.0.0.1/$PORT") >/dev/null 2>&1; then
        break
    fi
    sleep 0.3
done
if ! (echo > "/dev/tcp/127.0.0.1/$PORT") >/dev/null 2>&1; then
    err "server failed to listen on $PORT within 30s"
    tail -20 "$RUN_DIR/server.log" >&2
    exit 1
fi
log "server listening (pid=$SERVER_PID)"

# Use sysbench oltp_insert prepare (mirrors PR #4214)
log "preparing sbtest1 with sysbench oltp_insert prepare (creates secondary index)"
mysql -h "$HOST" -P "$PORT" -u "$USER" \
    -e "CREATE DATABASE IF NOT EXISTS ${DB};" >/dev/null 2>&1

timeout 60 sysbench oltp_insert \
    --db-driver=mysql \
    --mysql-host="$HOST" --mysql-port="$PORT" \
    --mysql-user="$USER" --mysql-db="$DB" \
    --table-size="$TABLE_SIZE" --tables=1 \
    prepare > "$RUN_DIR/sysbench_prepare.log" 2>&1 || true
n=$(mysql -h "$HOST" -P "$PORT" -u "$USER" "$DB" -N -B \
    -e "SELECT COUNT(*) FROM sbtest1;" 2>/dev/null | tr -d ' \r' || echo 0)
log "sbtest1 prepared: $n rows (expected=$TABLE_SIZE)"

run_workload() {
    local workload="$1" ps_mode="$2"
    local logfile="$RUN_DIR/sysbench_${workload}_${ps_mode}.log"
    log "[$workload ps_mode=$ps_mode ${TIME_SEC}s ${THREADS}t]"
    if timeout $((TIME_SEC + 60)) sysbench --db-driver=mysql \
        --mysql-host="$HOST" --mysql-port="$PORT" \
        --mysql-user="$USER" --mysql-db="$DB" \
        --table-size="$TABLE_SIZE" --tables=1 \
        --threads="$THREADS" --time="$TIME_SEC" \
        --db-ps-mode="$ps_mode" \
        "$workload" run > "$logfile" 2>&1; then
        local tps qps ie
        tps=$(grep "transactions:" "$logfile" | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
        qps=$(grep "queries:" "$logfile" | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
        ie=$(grep "ignored errors:" "$logfile" | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
        log "  tps=${tps:-N/A} qps=${qps:-N/A} ignored_errors=${ie:-N/A}"
        echo "OK" > "$RUN_DIR/.rc_${workload}_${ps_mode}"
    else
        local rc=$?
        err "  exited rc=$rc"
        grep -E "FATAL|errno" "$logfile" | head -3 >&2
        echo "FAIL" > "$RUN_DIR/.rc_${workload}_${ps_mode}"
    fi
}

# 4 workloads
run_workload oltp_write_only disable
run_workload oltp_read_write disable
run_workload oltp_write_only auto
run_workload oltp_read_write auto

{
    echo "repro4214mirror summary:"
    echo "  HEAD: $(git rev-parse HEAD)"
    echo "  THREADS: $THREADS  TIME_SEC: $TIME_SEC  TABLE_SIZE: $TABLE_SIZE"
    echo "  rc_oltp_write_only_disable: $(cat "$RUN_DIR/.rc_oltp_write_only_disable" 2>/dev/null || echo MISSING)"
    echo "  rc_oltp_read_write_disable: $(cat "$RUN_DIR/.rc_oltp_read_write_disable" 2>/dev/null || echo MISSING)"
    echo "  rc_oltp_write_only_auto: $(cat "$RUN_DIR/.rc_oltp_write_only_auto" 2>/dev/null || echo MISSING)"
    echo "  rc_oltp_read_write_auto: $(cat "$RUN_DIR/.rc_oltp_read_write_auto" 2>/dev/null || echo MISSING)"
} | tee "$RUN_DIR/summary.txt"

log "DONE — outputs in $RUN_DIR"