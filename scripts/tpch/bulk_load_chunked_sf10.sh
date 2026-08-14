#!/usr/bin/env bash
# bulk_load_chunked_sf10.sh — V312-26 followup / Issue #4217 chunked bulk-load runner
#
# Companion to bulk_load_sf10.sh that exercises the new
# `--bulk-insert-rows-per-flush` knob (issue #4217). Replaces the legacy
# hardcoded `PERIODIC_FLUSH_ROWS = 100` with a configurable value (default
# 10_000) so the LOAD DATA LOCAL INFILE handler issues ~600 storage flush
# checkpoints for SF=10 lineitem (~6 M rows) instead of ~60_000.
#
# Why a separate script?
#   * bulk_load_sf10.sh is the locked-baseline evidence script for issue
#     #4020. It must keep using the historical PERIODIC_FLUSH_ROWS=100
#     default so the comparison vs the prior run is apples-to-apples.
#   * This script (#4217) is the post-fix runner that proves the new knob
#     lifts the throughput cliff without regressing row-count parity.
#
# Outputs (per-run):
#   docs/releases/v3.12.0/evidence/issue-4217/<run-id>/
#     ├── server.log                (server stderr/stdout)
#     ├── bulk_load_summary.json    (per-table elapsed_sec, rows/sec, row_count_parity)
#     ├── bulk_load_log.txt         (load sequence + status)
#     ├── metadata.json             (config + versions + bulk_insert_rows_per_flush)
#     └── <tbl>_load.log            (per-table LOAD DATA output)
#
# Usage:
#   bash scripts/tpch/bulk_load_chunked_sf10.sh
#   DATA_DIR=/tmp/tpch-sf10 BULK_INSERT_ROWS_PER_FLUSH=50000 \
#       bash scripts/tpch/bulk_load_chunked_sf10.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# ---- knobs -----------------------------------------------------------------
DATA_DIR="${DATA_DIR:-/tmp/tpch-sf10}"
EVIDENCE_ROOT="$REPO_ROOT/docs/releases/v3.12.0/evidence/issue-4217"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)_chunked_sf10"
RUN_DIR="$EVIDENCE_ROOT/$RUN_ID"
SERVER_BIN="${SQLRUSTGO_SERVER_BIN:-target/debug/sqlrustgo-mysql-server}"
HOST="${HOST:-127.0.0.1}"
USER="${USER:-root}"
DB="${DB:-tpch_sf10}"
PREFERRED_PORT="${PREFERRED_PORT:-23318}"
LOAD_TIMEOUT_SEC="${LOAD_TIMEOUT_SEC:-300}"
# Issue #4217 knob — the entire point of this script. Default 10_000 matches
# the EphemeralConfig default and is the recommended starting point; tweak via
# env var for benchmarking different chunk sizes.
BULK_INSERT_ROWS_PER_FLUSH="${BULK_INSERT_ROWS_PER_FLUSH:-10000}"

mkdir -p "$RUN_DIR"

log() { echo -e "\033[0;32m[bulk_load_chunked]\033[0m $*" | tee -a "$RUN_DIR/bulk_load_log.txt" >&2; }
warn() { echo -e "\033[0;33m[bulk_load_chunked:WARN]\033[0m $*" | tee -a "$RUN_DIR/bulk_load_log.txt" >&2; }
err() { echo -e "\033[0;31m[bulk_load_chunked:ERROR]\033[0m $*" | tee -a "$RUN_DIR/bulk_load_log.txt" >&2; }

# ---- preflight -------------------------------------------------------------
preflight() {
    if [ ! -d "$DATA_DIR" ]; then
        err "DATA_DIR not found: $DATA_DIR"
        err "  Run: bash scripts/tpch/setup_sf10.sh"
        exit 1
    fi
    if ! command -v mysql >/dev/null 2>&1; then
        err "mysql client not found in PATH"
        exit 1
    fi
    if [ ! -x "$SERVER_BIN" ]; then
        err "sqlrustgo-mysql-server not at $SERVER_BIN"
        err "  Build first: cargo build --bin sqlrustgo-mysql-server"
        exit 1
    fi
    if ! [[ "$BULK_INSERT_ROWS_PER_FLUSH" =~ ^[0-9]+$ ]] || [ "$BULK_INSERT_ROWS_PER_FLUSH" -lt 1 ]; then
        err "BULK_INSERT_ROWS_PER_FLUSH must be a positive integer (got '$BULK_INSERT_ROWS_PER_FLUSH')"
        exit 1
    fi
    local missing=()
    for tbl in region nation supplier customer part partsupp orders lineitem; do
        if [ ! -f "$DATA_DIR/${tbl}.tbl" ]; then
            missing+=("${tbl}.tbl")
        fi
    done
    if [ ${#missing[@]} -gt 0 ]; then
        err "Missing .tbl files: ${missing[*]}"
        err "  Generate with: bash scripts/tpch/setup_sf10.sh $DATA_DIR"
        exit 1
    fi
}

# ---- helpers ---------------------------------------------------------------
find_free_port() {
    local port="$PREFERRED_PORT"
    for _ in $(seq 1 50); do
        if ! (echo > "/dev/tcp/127.0.0.1/$port") >/dev/null 2>&1; then
            echo "$port"; return 0
        fi
        port=$((port+1))
    done
    err "no free port in $PREFERRED_PORT..$((PREFERRED_PORT+49))"
    return 1
}

wait_for_port() {
    local port="$1" deadline=$((SECONDS + 30))
    while [ "$SECONDS" -lt "$deadline" ]; do
        if (echo > "/dev/tcp/127.0.0.1/$port") >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.3
    done
    return 1
}

# Schema source of truth: same column order as bulk_load_sf10.sh and
# scripts/stability/load_tpch_fixture.sh. Keep in lockstep.
SCHEMA_REGION="CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)"
SCHEMA_NATION="CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)"
SCHEMA_SUPPLIER="CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)"
SCHEMA_CUSTOMER="CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT NOT NULL)"
SCHEMA_PART="CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)"
SCHEMA_PARTSUPP="CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))"
SCHEMA_ORDERS="CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)"
SCHEMA_LINEITEM="CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL, PRIMARY KEY (l_orderkey, l_linenumber))"

TABLES=(region nation supplier customer part partsupp orders lineitem)
SCHEMAS=(
    "$SCHEMA_REGION"
    "$SCHEMA_NATION"
    "$SCHEMA_SUPPLIER"
    "$SCHEMA_CUSTOMER"
    "$SCHEMA_PART"
    "$SCHEMA_PARTSUPP"
    "$SCHEMA_ORDERS"
    "$SCHEMA_LINEITEM"
)

# ---- server lifecycle ------------------------------------------------------
start_server() {
    local port="$1"
    local data_dir="$RUN_DIR/data"
    mkdir -p "$data_dir"
    local wal_sync="${WAL_SYNC:-every}"
    log "starting sqlrustgo-mysql-server on $HOST:$port (data=$data_dir, infile=$DATA_DIR, bulk_insert_rows_per_flush=$BULK_INSERT_ROWS_PER_FLUSH, wal-sync=$wal_sync)"
    "$SERVER_BIN" serve \
        --host "$HOST" --port "$port" \
        --data-dir "$data_dir" \
        --load-infile-dir "$DATA_DIR" \
        --auth-mode none \
        --max-connections 8 \
        --server-threads 4 \
        --storage file \
        --wal-sync "$wal_sync" \
        --bulk-insert-rows-per-flush "$BULK_INSERT_ROWS_PER_FLUSH" \
        --log-level info \
        > "$RUN_DIR/server.log" 2>&1 &
    SERVER_PID=$!
    echo "$SERVER_PID" > "$RUN_DIR/server.pid"
    if wait_for_port "$port"; then
        log "server listening (pid=$SERVER_PID)"
    else
        err "server failed to listen on $port within 30s"
        tail -20 "$RUN_DIR/server.log" >&2
        kill "$SERVER_PID" 2>/dev/null
        return 1
    fi
}

stop_server() {
    if [ -n "${SERVER_PID:-}" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        log "stopping server (pid=$SERVER_PID)"
        kill "$SERVER_PID" 2>/dev/null || true
        for _ in $(seq 1 50); do
            kill -0 "$SERVER_PID" 2>/dev/null || break
            sleep 0.1
        done
        kill -9 "$SERVER_PID" 2>/dev/null || true
    fi
}

# ---- mysql helpers ---------------------------------------------------------
MYSQL_TIMEOUT_SEC="${MYSQL_TIMEOUT_SEC:-30}"

mysql_q() {
    local sql="$1"
    timeout "$MYSQL_TIMEOUT_SEC" mysql -h "$HOST" -P "${PORT}" -u "$USER" \
        --local-infile=1 --connect-timeout=10 -e "$sql" 2>&1
}

# ---- schema creation -------------------------------------------------------
create_schemas() {
    log "creating database + 8 TPC-H schemas"
    if ! mysql_q "CREATE DATABASE IF NOT EXISTS ${DB};" > "$RUN_DIR/schema_create.log" 2>&1; then
        err "CREATE DATABASE failed (see $RUN_DIR/schema_create.log)"
        return 1
    fi
    local i
    for i in "${!TABLES[@]}"; do
        local tbl="${TABLES[$i]}"
        local schema="${SCHEMAS[$i]}"
        if ! mysql_q "USE ${DB}; DROP TABLE IF EXISTS ${tbl}; ${schema};" \
                > "$RUN_DIR/${tbl}_create.log" 2>&1; then
            err "CREATE TABLE ${tbl} failed (see $RUN_DIR/${tbl}_create.log)"
            return 1
        fi
        log "  schema: ${tbl} created"
    done
    return 0
}

# ---- bulk load per table ---------------------------------------------------
# Records one JSON object per table into $RUN_DIR/bulk_load_summary.jsonl.
load_one_table() {
    local tbl="$1"
    local file="$DATA_DIR/${tbl}.tbl"
    local logf="$RUN_DIR/${tbl}_load.log"
    local src_lines
    src_lines=$(wc -l < "$file" | tr -d ' ')

    local start_ns end_ns elapsed_sec
    start_ns=$(date +%s%N)
    local rc=0
    timeout "$LOAD_TIMEOUT_SEC" mysql -h "$HOST" -P "${PORT}" -u "$USER" \
        --local-infile=1 --connect-timeout=10 "$DB" \
        -e "LOAD DATA LOCAL INFILE '${file}' INTO TABLE ${tbl} FIELDS TERMINATED BY '|' LINES TERMINATED BY '\n';" \
        > "$logf" 2>&1 || rc=$?
    end_ns=$(date +%s%N)
    elapsed_sec=$(awk -v s="$start_ns" -v e="$end_ns" 'BEGIN{printf "%.3f", (e-s)/1e9}')

    local count=-1
    if [ "$rc" -eq 0 ]; then
        count=$(timeout "$MYSQL_TIMEOUT_SEC" mysql -h "$HOST" -P "${PORT}" -u "$USER" "$DB" -N -B \
            -e "SELECT COUNT(*) FROM ${tbl};" 2>/dev/null | tr -d ' \r' || echo -1)
    fi

    local rows_per_sec
    if [ "$rc" -eq 0 ] && [ "$elapsed_sec" != "0.000" ]; then
        rows_per_sec=$(awk -v c="$count" -v s="$elapsed_sec" 'BEGIN{printf "%.0f", c/s}')
    else
        rows_per_sec=0
    fi

    local parity
    if [ "$rc" -ne 0 ]; then
        parity="load_failed"
    elif [ "$count" = "$src_lines" ]; then
        parity="match"
    else
        parity="mismatch"
    fi

    cat >> "$RUN_DIR/bulk_load_summary.jsonl" <<EOF
{"table":"${tbl}","src_lines":${src_lines},"loaded_rows":${count},"elapsed_sec":${elapsed_sec},"rows_per_sec":${rows_per_sec},"rc":${rc},"parity":"${parity}","bulk_insert_rows_per_flush":${BULK_INSERT_ROWS_PER_FLUSH}}
EOF

    if [ "$rc" -eq 0 ] && [ "$parity" = "match" ]; then
        log "  ${tbl}: ${count} rows in ${elapsed_sec}s (${rows_per_sec} rows/s) — parity=match"
    elif [ "$rc" -eq 0 ]; then
        warn "  ${tbl}: ${count}/${src_lines} rows in ${elapsed_sec}s — parity=${parity}"
    else
        err "  ${tbl}: LOAD failed (rc=$rc) — see ${logf}"
    fi
    return $rc
}

# ---- summary writer --------------------------------------------------------
write_summary_json() {
    python3 -c "
import json
items = []
with open('$RUN_DIR/bulk_load_summary.jsonl', 'r') as f:
    for line in f:
        line = line.strip()
        if line:
            items.append(json.loads(line))
out = {
    'issue': '#4217',
    'run_id': '$RUN_ID',
    'data_dir': '$DATA_DIR',
    'host_port': '$HOST:${PORT}',
    'db': '$DB',
    'bulk_insert_rows_per_flush': $BULK_INSERT_ROWS_PER_FLUSH,
    'mysql_timeout_sec': $MYSQL_TIMEOUT_SEC,
    'load_timeout_sec': $LOAD_TIMEOUT_SEC,
    'tables_total': len(items),
    'tables_loaded_ok': sum(1 for x in items if x['rc'] == 0 and x['parity'] == 'match'),
    'tables_failed': sum(1 for x in items if x['rc'] != 0),
    'tables_parity_mismatch': sum(1 for x in items if x['rc'] == 0 and x['parity'] != 'match'),
    'total_loaded_rows': sum(x['loaded_rows'] for x in items if x['loaded_rows'] > 0),
    'total_src_lines': sum(x['src_lines'] for x in items),
    'total_elapsed_sec': round(sum(x['elapsed_sec'] for x in items), 3),
    'tables': items,
}
with open('$RUN_DIR/bulk_load_summary.json', 'w') as f:
    json.dump(out, f, indent=2)
print(json.dumps({
    'tables_loaded_ok': out['tables_loaded_ok'],
    'tables_failed': out['tables_failed'],
    'bulk_insert_rows_per_flush': out['bulk_insert_rows_per_flush'],
    'total_elapsed_sec': out['total_elapsed_sec'],
}, indent=2))
"
}

write_metadata() {
    cat > "$RUN_DIR/metadata.json" <<EOF
{
  "issue": "#4217",
  "run_id": "$RUN_ID",
  "captured_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "sqlrustgo_version": "$($SERVER_BIN --version 2>&1 | head -1)",
  "mysql_version": "$(mysql --version 2>&1 | head -1)",
  "config": {
    "host": "$HOST",
    "port": ${PORT},
    "db": "$DB",
    "user": "$USER",
    "data_dir": "$DATA_DIR",
    "auth_mode": "none",
    "max_connections": 8,
    "server_threads": 4,
    "storage": "file",
    "wal_sync": "${WAL_SYNC:-every}",
    "bulk_insert_rows_per_flush": $BULK_INSERT_ROWS_PER_FLUSH
  },
  "knobs": {
    "mysql_timeout_sec": $MYSQL_TIMEOUT_SEC,
    "load_timeout_sec": $LOAD_TIMEOUT_SEC
  }
}
EOF
}

# ---- main ------------------------------------------------------------------
main() {
    preflight
    local port
    port=$(find_free_port) || exit 1
    export PORT="$port"
    trap 'stop_server' EXIT

    start_server "$port" || exit 1

    if ! create_schemas; then
        err "schema creation failed — see $RUN_DIR/*_create.log"
        write_metadata
        exit 1
    fi

    : > "$RUN_DIR/bulk_load_summary.jsonl"
    local total_rc=0
    for tbl in "${TABLES[@]}"; do
        load_one_table "$tbl" || total_rc=$?
    done

    write_summary_json
    write_metadata

    if [ "$total_rc" -ne 0 ]; then
        warn "DONE with failures — see $RUN_DIR/bulk_load_summary.json"
        local loaded_ok
        loaded_ok=$(python3 -c "
import json
with open('$RUN_DIR/bulk_load_summary.json') as f:
    d = json.load(f)
print(d['tables_loaded_ok'])
")
        if [ "$loaded_ok" -eq 0 ]; then
            err "no tables loaded successfully — see logs"
            exit 1
        fi
        exit 0
    fi
    log "DONE — all 8 tables loaded; outputs in $RUN_DIR"
}

main "$@"