#!/usr/bin/env bash
# scripts/tpch_sf1_baseline.sh
#
# TPC-H SF=1.0 cross-engine baseline (sqlrustgo vs SQLite vs MariaDB).
#
# Boots sqlrustgo-mysql-server on an OS-assigned port against the
# /home/openclaw/tpch_baseline/sf1 fixture, runs all 22 TPC-H queries
# via the external `mysql` CLI client (LOAD DATA LOCAL INFILE for
# fixture ingestion), records row counts + wall-clock per query, and
# emits docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md.
#
# Optionally also runs the same 22 queries against SQLite (skipping
# Q7/Q8/Q9 which use EXTRACT(YEAR FROM ...) that SQLite does not
# support) and against MariaDB if a server is reachable on
# 127.0.0.1:3306.
#
# Usage:
#   bash scripts/tpch_sf1_baseline.sh                # full baseline
#   bash scripts/tpch_sf1_baseline.sh --dry-run     # print plan only
#   bash scripts/tpch_sf1_baseline.sh --reload      # force LOAD DATA
#   bash scripts/tpch_sf1_baseline.sh --no-sqlite   # skip SQLite path
#   bash scripts/tpch_sf1_baseline.sh --no-mariadb   # skip MariaDB path
#
# Exit codes:
#   0  all enabled engines captured (or --dry-run)
#   1  fixture / binary / connectivity issue
#   2  sqlrustgo query failures
#
# Reference: openspec/changes/2026-06-18-tpch-sf1-baseline (Issue #3423)

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

SF1_DIR="${SF1_DIR:-/home/openclaw/tpch_baseline/sf1}"
QUERIES_DIR="$PROJECT_ROOT/queries"
REPORT_PATH="$PROJECT_ROOT/docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md"
RUNTIME_DIR="$(mktemp -d -t tpch_sf1_baseline_XXXXXX)"
SERVER_LOG="$RUNTIME_DIR/server.log"
SERVER_PID=""
LOADER_TIMEOUT_S="${LOADER_TIMEOUT_S:-900}"   # 15 min per query
SERVER_BOOT_WAIT_S="${SERVER_BOOT_WAIT_S:-30}"
LOAD_DATA_TIMEOUT_S="${LOAD_DATA_TIMEOUT_S:-1800}"   # 30 min for LOAD DATA
PER_QUERY_TIMEOUT_S="${PER_QUERY_TIMEOUT_S:-900}"

DRY_RUN=false
FORCE_RELOAD=false
RUN_SQLITE=true
RUN_MARIADB=true

usage() {
    sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)    DRY_RUN=true; shift ;;
        --reload)     FORCE_RELOAD=true; shift ;;
        --no-sqlite)  RUN_SQLITE=false; shift ;;
        --no-mariadb) RUN_MARIADB=false; shift ;;
        --sf1-dir)    SF1_DIR="$2"; shift 2 ;;
        --help|-h)    usage ;;
        *)            echo "[ERROR] unknown arg: $1" >&2; usage ;;
    esac
done

cleanup() {
    if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill -TERM "$SERVER_PID" 2>/dev/null || true
        # sqlrustgo-mysql-server (release/2026-06-20) has no tokio
        # signal handler, so SIGTERM is silently ignored and the
        # process sits in hrtimer_nanosleep forever. Wait up to 5s
        # for graceful exit, then force-kill the whole process group
        # to unblock the trap immediately.
        local waited=0
        while (( waited < 5 )) && kill -0 "$SERVER_PID" 2>/dev/null; do
            sleep 1
            waited=$((waited + 1))
        done
        if kill -0 "$SERVER_PID" 2>/dev/null; then
            kill -KILL -"$SERVER_PID" 2>/dev/null || \
                kill -KILL "$SERVER_PID" 2>/dev/null || true
        fi
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$RUNTIME_DIR"
}
trap cleanup EXIT

# ---------------------------------------------------------------------
# Step 1: Validate the SF=1 fixture is present
# ---------------------------------------------------------------------
echo "=== Step 1: validate fixture at $SF1_DIR ==="
EXPECTED_ROWS=(
    "region:5"
    "nation:25"
    "supplier:10000"
    "customer:150000"
    "part:200000"
    "partsupp:800000"
    "orders:1500000"
    "lineitem:6001215"
)
FIXTURE_OK=true
for spec in "${EXPECTED_ROWS[@]}"; do
    tbl="${spec%:*}"
    expected="${spec#*:}"
    f="$SF1_DIR/$tbl.tbl"
    if [[ ! -f "$f" ]]; then
        echo "  [FAIL] $f missing"
        FIXTURE_OK=false
        continue
    fi
    actual=$(wc -l < "$f" | tr -d ' ')
    if [[ "$actual" == "$expected" ]]; then
        printf "  [OK]   %-10s %9s rows\n" "$tbl.tbl" "$actual"
    else
        printf "  [FAIL] %-10s %9s rows (expected %s)\n" "$tbl.tbl" "$actual" "$expected"
        FIXTURE_OK=false
    fi
done
if ! $FIXTURE_OK; then
    echo "[ERROR] fixture incomplete; aborting." >&2
    echo "        Generate it with:" >&2
    echo "          bash scripts/generate_tpch_data.sh --sf 1 --backend dbgen" >&2
    echo "        or move dbgen output to $SF1_DIR" >&2
    exit 1
fi

# ---------------------------------------------------------------------
# Step 2: Locate the sqlrustgo-mysql-server binary
# ---------------------------------------------------------------------
echo
echo "=== Step 2: locate sqlrustgo-mysql-server binary ==="
BIN=""
for candidate in \
    "$PROJECT_ROOT/target/release/sqlrustgo-mysql-server" \
    "$PROJECT_ROOT/target/debug/sqlrustgo-mysql-server"
do
    if [[ -x "$candidate" ]]; then
        BIN="$candidate"
        break
    fi
done
if [[ -z "$BIN" ]]; then
    echo "[ERROR] sqlrustgo-mysql-server binary not found." >&2
    echo "        Build it with:" >&2
    echo "          cargo build --release --bin sqlrustgo-mysql-server" >&2
    exit 1
fi
echo "  [OK] $BIN"

# ---------------------------------------------------------------------
# Step 3: Allocate an OS-assigned port
# ---------------------------------------------------------------------
echo
echo "=== Step 3: allocate free port ==="
PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(('127.0.0.1', 0))
print(s.getsockname()[1])
s.close()
PY
)"
echo "  [OK] port=$PORT"

# ---------------------------------------------------------------------
# Step 4: Pick sqlrustgo data dir
# ---------------------------------------------------------------------
# The server requires LOAD DATA source paths to be inside --data-dir.
# /home/openclaw/tpch_baseline/sf1 already contains the .tbl files,
# so we point the server there directly. The server's per-table .json
# files will be written into the same directory.
SQLRUSTGO_DATA_DIR="$SF1_DIR"
# Verify .tbl files live under data dir (LOAD DATA whitelist)
for tbl in region nation supplier customer part partsupp orders lineitem; do
    if [[ ! -f "$SQLRUSTGO_DATA_DIR/$tbl.tbl" ]]; then
        echo "[ERROR] expected $tbl.tbl inside $SQLRUSTGO_DATA_DIR" >&2
        exit 1
    fi
done
echo "  [OK] data_dir=$SQLRUSTGO_DATA_DIR"

# ---------------------------------------------------------------------
# Step 5: If .json files are pre-generated AND actually contain rows,
# skip LOAD DATA unless --reload was passed.  We require lineitem.json
# to be larger than 100 KB (a fully materialized SF=1.0 lineitem is
# >1 GB; schema-only files are <3 KB).  An operator who has only the
# schema-only bootstrap state must run `--reload`.
# ---------------------------------------------------------------------
SKIP_LOAD_DATA=false
LINEITEM_JSON="$SQLRUSTGO_DATA_DIR/lineitem.json"
if [[ -f "$LINEITEM_JSON" ]] && \
   [[ $(stat -c %s "$LINEITEM_JSON" 2>/dev/null || echo 0) -gt 102400 ]] && \
   ! $FORCE_RELOAD; then
    SKIP_LOAD_DATA=true
fi

# ---------------------------------------------------------------------
# Step 6: Print plan
# ---------------------------------------------------------------------
echo
echo "=== Step 6: plan ==="
echo "  fixture:            $SF1_DIR (verified)"
echo "  sqlrustgo binary:   $BIN"
echo "  sqlrustgo data_dir: $SQLRUSTGO_DATA_DIR"
echo "  sqlrustgo port:     $PORT"
echo "  load data:          $($SKIP_LOAD_DATA && echo "skip (json files present)" || echo "yes (LOAD DATA LOCAL INFILE)")"
echo "  per-query timeout:  ${PER_QUERY_TIMEOUT_S}s"
echo "  sqlite engine:      $($RUN_SQLITE && echo "yes" || echo "no")"
echo "  mariadb engine:     $($RUN_MARIADB && echo "yes (probe 127.0.0.1:3306)" || echo "no")"
echo "  report:             $REPORT_PATH"

if $DRY_RUN; then
    echo
    echo "[DRY-RUN] no work performed."
    exit 0
fi

# ---------------------------------------------------------------------
# Step 7: Boot the sqlrustgo ephemeral server
# ---------------------------------------------------------------------
echo
echo "=== Step 7: boot sqlrustgo server ==="
"$BIN" serve --port "$PORT" --data-dir "$SQLRUSTGO_DATA_DIR" \
    --auth-mode none --verbose \
    > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!
echo "  pid=$SERVER_PID"

# Wait for listener
for _ in $(seq 1 $((SERVER_BOOT_WAIT_S * 10))); do
    if ss -lntp 2>/dev/null | grep -q ":$PORT "; then
        break
    fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "[ERROR] sqlrustgo server died during startup" >&2
        echo "--- server log (tail) ---" >&2
        tail -30 "$SERVER_LOG" >&2
        exit 1
    fi
    sleep 0.1
done
if ! ss -lntp 2>/dev/null | grep -q ":$PORT "; then
    echo "[ERROR] server did not start listening on $PORT within ${SERVER_BOOT_WAIT_S}s" >&2
    tail -30 "$SERVER_LOG" >&2
    exit 1
fi
echo "  [OK] listening on 127.0.0.1:$PORT"

MYSQL_ARGS=(-h 127.0.0.1 -P "$PORT" -u root --ssl-mode=DISABLED --local-infile=1)
MYSQL_Q() {
    timeout "$PER_QUERY_TIMEOUT_S" mysql "${MYSQL_ARGS[@]}" -B -N -e "$1" 2>&1
}

# ---------------------------------------------------------------------
# Step 8: Load fixture (if not pre-generated)
# ---------------------------------------------------------------------
echo
echo "=== Step 8: load fixture ==="
if $SKIP_LOAD_DATA; then
    echo "  [SKIP] .json files present; using pre-generated data"
else
    echo "  Creating tables..."
    for ddl in \
        "CREATE TABLE IF NOT EXISTS region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)" \
        "CREATE TABLE IF NOT EXISTS nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)" \
        "CREATE TABLE IF NOT EXISTS supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)" \
        "CREATE TABLE IF NOT EXISTS customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)" \
        "CREATE TABLE IF NOT EXISTS part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)" \
        "CREATE TABLE IF NOT EXISTS partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))" \
        "CREATE TABLE IF NOT EXISTS orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)" \
        "CREATE TABLE IF NOT EXISTS lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)"
    do
        MYSQL_Q "$ddl" >/dev/null
    done
    for tbl in region nation supplier customer part partsupp orders lineitem; do
        f="$SQLRUSTGO_DATA_DIR/$tbl.tbl"
        echo "  LOAD DATA $tbl ..."
        start=$(date +%s)
        MYSQL_Q "LOAD DATA LOCAL INFILE '$f' INTO TABLE $tbl FIELDS TERMINATED BY '|' LINES TERMINATED BY '|';" >/dev/null
        elapsed=$(( $(date +%s) - start ))
        count=$(MYSQL_Q "SELECT COUNT(*) FROM $tbl;" | tail -1 | tr -d ' ')
        printf "    %-10s loaded %9s rows in %4ss\n" "$tbl" "$count" "$elapsed"
    done
    echo "  [OK] fixture loaded"
fi

# ---------------------------------------------------------------------
# Step 9: Run 22 queries on sqlrustgo via external mysql CLI
# ---------------------------------------------------------------------
echo
echo "=== Step 9: 22 queries on sqlrustgo (external mysql CLI) ==="
SQLRUSTGO_RESULTS=()
SQLRUSTGO_FAILED=0
for n in $(seq 1 22); do
    qfile="$QUERIES_DIR/q$n.sql"
    if [[ ! -f "$qfile" ]]; then
        echo "  Q$n: missing $qfile" >&2
        SQLRUSTGO_FAILED=$((SQLRUSTGO_FAILED + 1))
        SQLRUSTGO_RESULTS+=("$n|0|0.000|MISSING")
        continue
    fi
    sql=$(cat "$qfile")
    out_file="$RUNTIME_DIR/q${n}.out"
    err_file="$RUNTIME_DIR/q${n}.err"
    # mysql -B -N: tab-separated, no header
    start=$(date +%s.%N)
    set +e
    timeout "$PER_QUERY_TIMEOUT_S" mysql "${MYSQL_ARGS[@]}" -B -N -e "$sql" \
        > "$out_file" 2> "$err_file"
    rc=$?
    set -e
    end=$(date +%s.%N)
    elapsed=$(awk -v s="$start" -v e="$end" 'BEGIN { printf "%.3f", e - s }')
    if [[ $rc -ne 0 ]]; then
        printf "  Q%-3s FAIL (rc=%d) %ss err=%s\n" "$n" "$rc" "$elapsed" \
            "$(head -1 "$err_file" | tr -d '\n')"
        SQLRUSTGO_FAILED=$((SQLRUSTGO_FAILED + 1))
        SQLRUSTGO_RESULTS+=("$n|0|$elapsed|FAIL: $(head -1 "$err_file" | tr '|' ' ')")
        continue
    fi
    # Count rows (each row has at least one tab)
    rows=$(awk 'END { print NR }' "$out_file")
    tr '\t' '|' < "$out_file" > "$out_file.norm"
    SQLRUSTGO_RESULTS+=("$n|$rows|$elapsed|ok")
    printf "  Q%-3s %9s rows %8ss\n" "$n" "$rows" "$elapsed"
done

# ---------------------------------------------------------------------
# Step 10: SQLite cross-check (optional)
# ---------------------------------------------------------------------
SQLITE_RESULTS=()
SQLITE_FAILED=0
SQLITE_SKIPPED_QS=(7 8 9)   # use EXTRACT(YEAR FROM ...) not in SQLite
if $RUN_SQLITE; then
    echo
    echo "=== Step 10: 22 queries on SQLite (Q7/Q8/Q9 unsupported, skipped) ==="
    SQLITE_DB="$RUNTIME_DIR/tpch.sqlite"
    sqlite3 "$SQLITE_DB" <<'SQL'
CREATE TABLE region   (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT);
CREATE TABLE nation   (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT);
CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT);
CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT);
CREATE TABLE part     (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT);
CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey));
CREATE TABLE orders   (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL);
CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL);
SQL

    # Load via .import (Pipe-separated TPC-H .tbl files)
    echo "  importing 8 .tbl files into sqlite..."
    python3 - <<PY
import sqlite3, sys
db = "$SQLITE_DB"
data = "$SF1_DIR"
con = sqlite3.connect(db)
cur = con.cursor()
type_map = {
    'region':   ['INTEGER','TEXT','TEXT'],
    'nation':   ['INTEGER','TEXT','INTEGER','TEXT'],
    'supplier': ['INTEGER','TEXT','TEXT','INTEGER','TEXT','REAL','TEXT'],
    'customer': ['INTEGER','TEXT','TEXT','INTEGER','TEXT','REAL','TEXT','TEXT'],
    'part':     ['INTEGER','TEXT','TEXT','TEXT','TEXT','INTEGER','TEXT','REAL','TEXT'],
    'partsupp': ['INTEGER','INTEGER','INTEGER','REAL','TEXT'],
    'orders':   ['INTEGER','INTEGER','TEXT','REAL','TEXT','TEXT','TEXT','INTEGER','TEXT'],
    'lineitem': ['INTEGER','INTEGER','INTEGER','INTEGER','REAL','REAL','REAL','REAL','TEXT','TEXT','TEXT','TEXT','TEXT','TEXT','TEXT','TEXT'],
}
for tbl, types in type_map.items():
    n = 0
    with open(f'{data}/{tbl}.tbl') as f:
        for line in f:
            line = line.rstrip('\n')
            if line.endswith('|'):
                line = line[:-1]
            cols = line.split('|')
            if len(cols) != len(types):
                continue
            cast = []
            for v, t in zip(cols, types):
                if t == 'INTEGER':
                    try:
                        cast.append(int(v))
                    except ValueError:
                        cast.append(None)
                elif t == 'REAL':
                    try:
                        cast.append(float(v))
                    except ValueError:
                        cast.append(None)
                else:
                    cast.append(v)
            placeholders = ','.join('?' * len(cast))
            cur.execute(f'INSERT INTO {tbl} VALUES ({placeholders})', cast)
            n += 1
    con.commit()
    print(f'    {tbl}: {n} rows')
con.close()
PY

    for n in $(seq 1 22); do
        if [[ " ${SQLITE_SKIPPED_QS[*]} " == *" $n "* ]]; then
            SQLITE_RESULTS+=("$n|0|0.000|SKIP (EXTRACT unsupported)")
            printf "  Q%-3s SKIP (EXTRACT)\n" "$n"
            continue
        fi
        sql=$(cat "$QUERIES_DIR/q$n.sql")
        # Strip trailing semicolon (sqlite3 -cmd wants no semicolon sometimes)
        sql="${sql%;}"
        out_file="$RUNTIME_DIR/sqlite_q${n}.out"
        start=$(date +%s.%N)
        set +e
        timeout "$PER_QUERY_TIMEOUT_S" sqlite3 -separator "|" "$SQLITE_DB" "$sql" \
            > "$out_file" 2>&1
        rc=$?
        set -e
        end=$(date +%s.%N)
        elapsed=$(awk -v s="$start" -v e="$end" 'BEGIN { printf "%.3f", e - s }')
        if [[ $rc -ne 0 ]]; then
            printf "  Q%-3s FAIL (rc=%d) %ss err=%s\n" "$n" "$rc" "$elapsed" \
                "$(head -1 "$out_file" | tr -d '\n')"
            SQLITE_FAILED=$((SQLITE_FAILED + 1))
            SQLITE_RESULTS+=("$n|0|$elapsed|FAIL")
            continue
        fi
        rows=$(awk 'END { print NR }' "$out_file")
        SQLITE_RESULTS+=("$n|$rows|$elapsed|ok")
        printf "  Q%-3s %9s rows %8ss\n" "$n" "$rows" "$elapsed"
    done
fi

# ---------------------------------------------------------------------
# Step 11: MariaDB cross-check (optional, only if reachable on 3306)
# ---------------------------------------------------------------------
MARIADB_RESULTS=()
MARIADB_FAILED=0
MARIADB_AVAILABLE=false
if $RUN_MARIADB; then
    if command -v mysql >/dev/null && \
       timeout 2 mysql -h 127.0.0.1 -P 3306 -u root --ssl-mode=DISABLED -e "SELECT 1" >/dev/null 2>&1; then
        MARIADB_AVAILABLE=true
    fi
fi
if $MARIADB_AVAILABLE; then
    echo
    echo "=== Step 11: 22 queries on MariaDB (127.0.0.1:3306) ==="
    MARIADB_DB="tpch_sf1_baseline_$$"
    mysql -h 127.0.0.1 -P 3306 -u root --ssl-mode=DISABLED \
        -e "DROP DATABASE IF EXISTS $MARIADB_DB; CREATE DATABASE $MARIADB_DB;" >/dev/null 2>&1

    # Load fixture via LOAD DATA LOCAL INFILE
    mysql -h 127.0.0.1 -P 3306 -u root --ssl-mode=DISABLED --local-infile=1 \
        "$MARIADB_DB" <<SQL >/dev/null 2>&1
CREATE TABLE region   (r_regionkey INTEGER PRIMARY KEY, r_name VARCHAR(25) NOT NULL, r_comment VARCHAR(152));
CREATE TABLE nation   (n_nationkey INTEGER PRIMARY KEY, n_name VARCHAR(25) NOT NULL, n_regionkey INTEGER NOT NULL, n_comment VARCHAR(152));
CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name VARCHAR(25) NOT NULL, s_address VARCHAR(40) NOT NULL, s_nationkey INTEGER NOT NULL, s_phone VARCHAR(15) NOT NULL, s_acctbal DECIMAL(15,2) NOT NULL, s_comment VARCHAR(101));
CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name VARCHAR(25) NOT NULL, c_address VARCHAR(40) NOT NULL, c_nationkey INTEGER NOT NULL, c_phone VARCHAR(15) NOT NULL, c_acctbal DECIMAL(15,2) NOT NULL, c_mktsegment VARCHAR(10), c_comment VARCHAR(117));
CREATE TABLE part     (p_partkey INTEGER PRIMARY KEY, p_name VARCHAR(55) NOT NULL, p_mfgr VARCHAR(25) NOT NULL, p_brand VARCHAR(10) NOT NULL, p_type VARCHAR(25) NOT NULL, p_size INTEGER NOT NULL, p_container VARCHAR(10) NOT NULL, p_retailprice DECIMAL(15,2) NOT NULL, p_comment VARCHAR(23));
CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost DECIMAL(15,2) NOT NULL, ps_comment VARCHAR(199), PRIMARY KEY (ps_partkey, ps_suppkey));
CREATE TABLE orders   (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus CHAR(1) NOT NULL, o_totalprice DECIMAL(15,2) NOT NULL, o_orderdate DATE NOT NULL, o_orderpriority VARCHAR(15) NOT NULL, o_clerk VARCHAR(15) NOT NULL, o_shippriority INTEGER NOT NULL, o_comment VARCHAR(79) NOT NULL);
CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity DECIMAL(15,2) NOT NULL, l_extendedprice DECIMAL(15,2) NOT NULL, l_discount DECIMAL(15,2) NOT NULL, l_tax DECIMAL(15,2) NOT NULL, l_returnflag CHAR(1) NOT NULL, l_linestatus CHAR(1) NOT NULL, l_shipdate DATE NOT NULL, l_commitdate DATE NOT NULL, l_receiptdate DATE NOT NULL, l_shipinstruct VARCHAR(25) NOT NULL, l_shipmode VARCHAR(10) NOT NULL, l_comment VARCHAR(44) NOT NULL);
SQL
    for tbl in region nation supplier customer part partsupp orders lineitem; do
        f="$SF1_DIR/$tbl.tbl"
        mysql -h 127.0.0.1 -P 3306 -u root --ssl-mode=DISABLED --local-infile=1 \
            "$MARIADB_DB" -e "LOAD DATA LOCAL INFILE '$f' INTO TABLE $tbl FIELDS TERMINATED BY '|' LINES TERMINATED BY '|';" >/dev/null 2>&1
    done

    for n in $(seq 1 22); do
        sql=$(cat "$QUERIES_DIR/q$n.sql")
        out_file="$RUNTIME_DIR/mariadb_q${n}.out"
        start=$(date +%s.%N)
        set +e
        timeout "$PER_QUERY_TIMEOUT_S" mysql -h 127.0.0.1 -P 3306 -u root --ssl-mode=DISABLED \
            -B -N "$MARIADB_DB" -e "$sql" > "$out_file" 2>&1
        rc=$?
        set -e
        end=$(date +%s.%N)
        elapsed=$(awk -v s="$start" -v e="$end" 'BEGIN { printf "%.3f", e - s }')
        if [[ $rc -ne 0 ]]; then
            MARIADB_FAILED=$((MARIADB_FAILED + 1))
            MARIADB_RESULTS+=("$n|0|$elapsed|FAIL")
            printf "  Q%-3s FAIL (rc=%d) %ss\n" "$n" "$rc" "$elapsed"
            continue
        fi
        rows=$(awk 'END { print NR }' "$out_file")
        MARIADB_RESULTS+=("$n|$rows|$elapsed|ok")
        printf "  Q%-3s %9s rows %8ss\n" "$n" "$rows" "$elapsed"
    done

    mysql -h 127.0.0.1 -P 3306 -u root --ssl-mode=DISABLED \
        -e "DROP DATABASE $MARIADB_DB" >/dev/null 2>&1
fi

# ---------------------------------------------------------------------
# Step 12: Write report
# ---------------------------------------------------------------------
echo
echo "=== Step 12: write report ==="
mkdir -p "$(dirname "$REPORT_PATH")"

{
    echo "# TPC-H SF=1.0 cross-engine baseline"
    echo
    echo "- Issue: #3423"
    echo "- Spec: openspec/changes/2026-06-18-tpch-sf1-baseline"
    echo "- Surface: external \`mysql\` CLI client against \`sqlrustgo-mysql-server serve\`"
    echo "- Branch: $(git -C "$PROJECT_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
    echo "- Commit: $(git -C "$PROJECT_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
    echo "- Date:   $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "- Fixture: \`$SF1_DIR\` (dbgen -s 1 -f)"
    echo "- Per-query timeout: ${PER_QUERY_TIMEOUT_S}s"
    echo
    echo "## Engines"
    echo
    echo "- **sqlrustgo**: in-process server bound to 127.0.0.1:$PORT"
    if $RUN_SQLITE; then
        echo "- **SQLite**:   local sqlite3 $(sqlite3 --version | awk '{print $1}')"
    else
        echo "- **SQLite**:   (skipped via --no-sqlite)"
    fi
    if $RUN_MARIADB; then
        if $MARIADB_AVAILABLE; then
            echo "- **MariaDB**:  reachable at 127.0.0.1:3306"
        else
            echo "- **MariaDB**:  not reachable at 127.0.0.1:3306 (skipped)"
        fi
    else
        echo "- **MariaDB**:  (skipped via --no-mariadb)"
    fi
    echo
    echo "## Fixture row counts (verified)"
    echo
    echo "| Table | Rows |"
    echo "|-------|------|"
    for spec in "${EXPECTED_ROWS[@]}"; do
        tbl="${spec%:*}"
        expected="${spec#*:}"
        echo "| $tbl | $expected |"
    done
    echo
    echo "## Per-query results"
    echo
    if $RUN_SQLITE && $MARIADB_AVAILABLE; then
        echo "| Q | sqlrustgo rows | sqlrustgo ms | SQLite rows | SQLite ms | MariaDB rows | MariaDB ms |"
        echo "|---|---------------:|-------------:|------------:|----------:|------------:|-----------:|"
        for n in $(seq 1 22); do
            sr=$(printf '%s\n' "${SQLRUSTGO_RESULTS[@]}" | awk -F'|' -v q="$n" '$1==q')
            xr=$(printf '%s\n' "${SQLITE_RESULTS[@]}" | awk -F'|' -v q="$n" '$1==q')
            mr=$(printf '%s\n' "${MARIADB_RESULTS[@]}" | awk -F'|' -v q="$n" '$1==q')
            printf "| Q%-2s | %s | %s | %s | %s | %s | %s |\n" \
                "$n" \
                "$(echo "$sr" | awk -F'|' '{print $2}')" \
                "$(echo "$sr" | awk -F'|' '{print $3}')" \
                "$(echo "$xr" | awk -F'|' '{print $2}')" \
                "$(echo "$xr" | awk -F'|' '{print $3}')" \
                "$(echo "$mr" | awk -F'|' '{print $2}')" \
                "$(echo "$mr" | awk -F'|' '{print $3}')"
        done
    elif $RUN_SQLITE; then
        echo "| Q | sqlrustgo rows | sqlrustgo ms | SQLite rows | SQLite ms |"
        echo "|---|---------------:|-------------:|------------:|----------:|"
        for n in $(seq 1 22); do
            sr=$(printf '%s\n' "${SQLRUSTGO_RESULTS[@]}" | awk -F'|' -v q="$n" '$1==q')
            xr=$(printf '%s\n' "${SQLITE_RESULTS[@]}" | awk -F'|' -v q="$n" '$1==q')
            printf "| Q%-2s | %s | %s | %s | %s |\n" \
                "$n" \
                "$(echo "$sr" | awk -F'|' '{print $2}')" \
                "$(echo "$sr" | awk -F'|' '{print $3}')" \
                "$(echo "$xr" | awk -F'|' '{print $2}')" \
                "$(echo "$xr" | awk -F'|' '{print $3}')"
        done
    else
        echo "| Q | sqlrustgo rows | sqlrustgo ms |"
        echo "|---|---------------:|-------------:|"
        for n in $(seq 1 22); do
            sr=$(printf '%s\n' "${SQLRUSTGO_RESULTS[@]}" | awk -F'|' -v q="$n" '$1==q')
            printf "| Q%-2s | %s | %s |\n" \
                "$n" \
                "$(echo "$sr" | awk -F'|' '{print $2}')" \
                "$(echo "$sr" | awk -F'|' '{print $3}')"
        done
    fi
    echo
    echo "## Summary"
    echo
    echo "- sqlrustgo 22/22 PASS: $([[ $SQLRUSTGO_FAILED -eq 0 ]] && echo yes || echo "no ($SQLRUSTGO_FAILED failed)")"
    if $RUN_SQLITE; then
        echo "- SQLite (Q7/Q8/Q9 skipped) failures: $SQLITE_FAILED"
    fi
    if $MARIADB_AVAILABLE; then
        echo "- MariaDB failures: $MARIADB_FAILED"
    fi
    echo
    echo "## Limitations"
    echo
    echo "- Q7/Q8/Q9 use \`EXTRACT(YEAR FROM ...)\` which SQLite does not"
    echo "  support natively; SQLite rows for those queries are reported"
    echo "  as SKIP, not 0. MariaDB is the cross-check for those three."
    echo "- This is a row-count + wall-clock baseline. Cell-by-cell value"
    echo "  comparison is out of scope for this script (covered separately"
    echo "  via the in-process test harness at tests/tpch_sf1_22_vs_3engines_test.rs)."
    echo
} > "$REPORT_PATH"

echo "  [OK] wrote $REPORT_PATH"

if [[ $SQLRUSTGO_FAILED -gt 0 ]]; then
    echo
    echo "[FAIL] sqlrustgo had $SQLRUSTGO_FAILED failed queries"
    exit 2
fi

echo
echo "[DONE] baseline captured. Report: $REPORT_PATH"
exit 0
