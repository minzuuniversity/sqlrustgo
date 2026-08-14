#!/usr/bin/env bash
# repro_4214.sh — Independent reproduction of PR #4214 claim on current develop HEAD
#
# PR #4214 (merged as squash commit 9170661f46 on origin/develop/v3.12.0)
# claims:
#   - oltp_write_only (--db-ps-mode=disable, 2t, 10s, table_size=100) PASS
#     173.56 tps / 1041.34 qps / 0 ignored errors
#   - oltp_read_write (--db-ps-mode=disable, 2t, 10s, table_size=100) PASS
#     137.46 tps / 2749.23 qps / 0 ignored errors
#
# This script independently reproduces those two baselines on the current HEAD
# with my own server lifecycle / my own mysql client setup. Output mirrors
# the PR #4214 evidence structure but the run directory has a distinct name.
#
# Per STRICT PROOF MODE: do NOT just trust PR #4214's claim. Verify it.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

TIME_SEC="${TIME_SEC:-10}"
THREADS="${THREADS:-2}"
TABLE_SIZE="${TABLE_SIZE:-100}"
EVIDENCE_ROOT="$REPO_ROOT/docs/releases/v3.12.0/evidence/issue-4019"
RUN_ID="repro4214_$(date -u +%Y%m%dT%H%M%SZ)_t${THREADS}_s${TABLE_SIZE}"
RUN_DIR="$EVIDENCE_ROOT/$RUN_ID"
SERVER_BIN="${SQLRUSTGO_SERVER_BIN:-$REPO_ROOT/target/debug/sqlrustgo-mysql-server}"
HOST="${HOST:-127.0.0.1}"
USER="${USER:-root}"

# Find free port
PREFERRED_PORT="${PREFERRED_PORT:-23400}"
PORT="$PREFERRED_PORT"
for _ in $(seq 1 50); do
    if ! (echo > "/dev/tcp/127.0.0.1/$PORT") >/dev/null 2>&1; then
        break
    fi
    PORT=$((PORT+1))
done

mkdir -p "$RUN_DIR"

log() { echo -e "\033[0;32m[repro4214]\033[0m $*" | tee -a "$RUN_DIR/run.log" >&2; }
err() { echo -e "\033[0;31m[repro4214:ERR]\033[0m $*" | tee -a "$RUN_DIR/run.log" >&2; }

DB="repro4214_$$"

cleanup() {
    if [ -n "${SERVER_PID:-}" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        log "stopping server (pid=$SERVER_PID)"
        kill "$SERVER_PID" 2>/dev/null || true
    fi
    mysql -h "$HOST" -P "$PORT" -u "$USER" \
        -e "DROP DATABASE IF EXISTS ${DB};" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Start server
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
echo "$SERVER_PID" > "$RUN_DIR/server.pid"

# Wait for port
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

# Prepare sbtest1
log "preparing sbtest1 (table_size=$TABLE_SIZE)"
mysql -h "$HOST" -P "$PORT" -u "$USER" \
    -e "CREATE DATABASE IF NOT EXISTS ${DB};" >/dev/null 2>&1
mysql -h "$HOST" -P "$PORT" -u "$USER" "$DB" \
    -e "CREATE TABLE IF NOT EXISTS sbtest1 (
        id INT PRIMARY KEY,
        k INT NOT NULL,
        c VARCHAR(64) NOT NULL,
        pad VARCHAR(64) NOT NULL
    );" >/dev/null 2>&1

# Insert rows
values=""
for i in $(seq 1 "$TABLE_SIZE"); do
    k=$(( (i * 31) % 1000 ))
    if [ -n "$values" ]; then values="$values,"; fi
    values="${values}(${i},${k},'payload-${i}','pad-${i}')"
    if [ $((i % 100)) -eq 0 ] || [ "$i" = "$TABLE_SIZE" ]; then
        mysql -h "$HOST" -P "$PORT" -u "$USER" "$DB" \
            -e "INSERT INTO sbtest1 (id,k,c,pad) VALUES ${values};" \
            >/dev/null 2>&1 || true
        values=""
    fi
done
n=$(mysql -h "$HOST" -P "$PORT" -u "$USER" "$DB" -N -B \
    -e "SELECT COUNT(*) FROM sbtest1;" 2>/dev/null | tr -d ' \r' || echo 0)
log "sbtest1 prepared: $n rows"

# ---- oltp_write_only (--db-ps-mode=disable) ----
log "[1/2] oltp_write_only (--db-ps-mode=disable, ${TIME_SEC}s, ${THREADS}t)"
if timeout $((TIME_SEC + 60)) sysbench --db-driver=mysql \
    --mysql-host="$HOST" --mysql-port="$PORT" \
    --mysql-user="$USER" --mysql-db="$DB" \
    --table-size="$TABLE_SIZE" --tables=1 \
    --threads="$THREADS" --time="$TIME_SEC" \
    --db-ps-mode=disable \
    oltp_write_only run > "$RUN_DIR/sysbench_oltp_write_only.log" 2>&1; then
    tps=$(grep "transactions:" "$RUN_DIR/sysbench_oltp_write_only.log" \
        | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
    qps=$(grep "queries:" "$RUN_DIR/sysbench_oltp_write_only.log" \
        | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
    ie=$(grep "ignored errors:" "$RUN_DIR/sysbench_oltp_write_only.log" \
        | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
    log "  oltp_write_only: tps=${tps:-N/A} qps=${qps:-N/A} ignored_errors=${ie:-N/A}"
    echo "OK" > "$RUN_DIR/.rc_oltp_write_only"
else
    rc=$?
    err "  oltp_write_only exited rc=$rc"
    tail -10 "$RUN_DIR/sysbench_oltp_write_only.log" >&2
    echo "FAIL" > "$RUN_DIR/.rc_oltp_write_only"
fi

# ---- oltp_read_write (--db-ps-mode=disable) ----
log "[2/2] oltp_read_write (--db-ps-mode=disable, ${TIME_SEC}s, ${THREADS}t)"
if timeout $((TIME_SEC + 60)) sysbench --db-driver=mysql \
    --mysql-host="$HOST" --mysql-port="$PORT" \
    --mysql-user="$USER" --mysql-db="$DB" \
    --table-size="$TABLE_SIZE" --tables=1 \
    --threads="$THREADS" --time="$TIME_SEC" \
    --db-ps-mode=disable \
    oltp_read_write run > "$RUN_DIR/sysbench_oltp_read_write.log" 2>&1; then
    tps=$(grep "transactions:" "$RUN_DIR/sysbench_oltp_read_write.log" \
        | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
    qps=$(grep "queries:" "$RUN_DIR/sysbench_oltp_read_write.log" \
        | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
    ie=$(grep "ignored errors:" "$RUN_DIR/sysbench_oltp_read_write.log" \
        | head -1 | awk -F: '{print $2}' | awk -F'[()]' '{print $2}' | awk '{print $1}')
    log "  oltp_read_write: tps=${tps:-N/A} qps=${qps:-N/A} ignored_errors=${ie:-N/A}"
    echo "OK" > "$RUN_DIR/.rc_oltp_read_write"
else
    rc=$?
    err "  oltp_read_write exited rc=$rc"
    tail -10 "$RUN_DIR/sysbench_oltp_read_write.log" >&2
    echo "FAIL" > "$RUN_DIR/.rc_oltp_read_write"
fi

# Summary
{
    echo "repro4214 summary:"
    echo "  HEAD: $(git rev-parse HEAD)"
    echo "  THREADS: $THREADS  TIME_SEC: $TIME_SEC  TABLE_SIZE: $TABLE_SIZE"
    echo "  PORT: $PORT"
    echo "  rc_oltp_write_only: $(cat "$RUN_DIR/.rc_oltp_write_only" 2>/dev/null || echo MISSING)"
    echo "  rc_oltp_read_write: $(cat "$RUN_DIR/.rc_oltp_read_write" 2>/dev/null || echo MISSING)"
} | tee "$RUN_DIR/summary.txt"

log "DONE — outputs in $RUN_DIR"