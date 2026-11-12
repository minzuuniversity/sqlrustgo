#!/usr/bin/env bash
# check_168h_runner_executable.sh — prove the 168h soak runner's PATH works,
# without waiting 168 hours.
#
# Why this exists: the CI runner that would execute the 168h/24h soak is
# currently unavailable (#5597 missing image, #5551 docker address pool,
# #5569 runner service dependency, #5573 runners shut down mid-job), so the
# runner path has **no evidence at all** — and AB-07 ("24h runner 可执行")
# is therefore unproven for a reason that has nothing to do with the code.
#
# This script mirrors the workflow's shape
# (.gitea/workflows/soak_168h.yml, job `soak`) with a short duration:
#
#   1. the three runner binaries exist (sqlrustgo-mysql-server, sqlrustgo,
#      sqlrustgo-soak);
#   2. the server starts with the runner's flags and the CLI reaches it;
#   3. the runner's three query files are written;
#   4. the soak workers run and emit their `SOAK:` reports;
#   5. metrics.csv is produced with samples while they run.
#
# It says nothing about 168-hour durability (only the CI runner can produce
# that). What it proves is that the *path* is executable today, so the
# remaining blocker is infrastructure rather than code.
#
# Usage:
#   DURATION_S=60 LOAD_PROFILE=light bash scripts/soak/check_168h_runner_executable.sh
#
# Exit codes: 0 = path executable; 1 = a step failed (the failing step is
# named in the output).
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

DURATION_S="${DURATION_S:-60}"
PORT="${PORT:-3398}"
LOAD_PROFILE="${LOAD_PROFILE:-light}"
TS="$(date +%Y%m%d_%H%M%S)"
RESULTS_DIR="${RESULTS_DIR:-/tmp/srg-168h-path-$TS}"
DATA_DIR="${DATA_DIR:-/tmp/srg-168h-data-$TS}"

log()  { printf '%s\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------- artifacts
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
find_bin() {
  local name="$1"
  for profile in release debug; do
    if [ -x "$CARGO_TARGET_DIR/$profile/$name" ]; then
      printf '%s' "$CARGO_TARGET_DIR/$profile/$name"
      return 0
    fi
  done
  return 1
}
SERVER_BIN="$(find_bin sqlrustgo-mysql-server)" \
  || fail "sqlrustgo-mysql-server not found under $CARGO_TARGET_DIR/{release,debug} — build it: cargo build --bin sqlrustgo-mysql-server -p sqlrustgo-mysql-server"
CLI_BIN="$(find_bin sqlrustgo)" \
  || fail "sqlrustgo not found under $CARGO_TARGET_DIR/{release,debug} — build it: cargo build --bin sqlrustgo -p sqlrustgo-cli"
SOAK_BIN="$(find_bin sqlrustgo-soak)" \
  || fail "sqlrustgo-soak not found under $CARGO_TARGET_DIR/{release,debug} — build it: cargo build --bin sqlrustgo-soak -p sqlrustgo-soak"
log "binaries: $SERVER_BIN | $CLI_BIN | $SOAK_BIN"

rm -rf "$DATA_DIR" "$RESULTS_DIR"
mkdir -p "$DATA_DIR" "$RESULTS_DIR" || fail "cannot create $DATA_DIR / $RESULTS_DIR"

cleanup() {
  if [ -n "${SERVER_PID:-}" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# ------------------------------------------------------------------- server
"$SERVER_BIN" serve \
  --port "$PORT" \
  --data-dir "$DATA_DIR" \
  --server-threads 16 \
  --max-connections 200 \
  --log-level warn \
  > "$RESULTS_DIR/server.log" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$RESULTS_DIR/server.pid"

READY=0
for i in $(seq 1 30); do
  if echo "SELECT 1" | "$CLI_BIN" soak --port "$PORT" --user root > /dev/null 2>&1; then
    READY=1
    log "server ready after ${i}s"
    break
  fi
  sleep 2
done
[ "$READY" = "1" ] || { tail -20 "$RESULTS_DIR/server.log" >&2; fail "server never became reachable through the CLI"; }

echo "CREATE DATABASE IF NOT EXISTS sbtest" \
  | "$CLI_BIN" soak --port "$PORT" --user root > /dev/null 2>&1 || true

# Read one scalar out of the soak CLI.
#
# The REPL prints a preamble plus ROWS/COL/DATA/ROW lines. Measured on the
# current CLI:
#   * `DATA<TAB>n` is the number of **result rows** (a scalar query has
#     exactly one result row ⇒ DATA is always 1 — reading it as the value
#     is wrong);
#   * the value of a scalar query is on the `ROW` line (`ROW<TAB>3`).
query_scalar() {
  "$CLI_BIN" soak --port "$PORT" --user root 2>/dev/null \
    | awk -F'\t' '$1=="ROW"{print $2; exit}'
}

# Provision the table the workload reads. Without it every query fails while
# the runner still reports a qps number (measured on the same shape:
# 1122 queries, 1122 errors, "18.7 qps") — the "exit 0 ≠ the arm really ran"
# failure mode, in the runner AB-07 depends on.
SBTEST_ROWS="${SBTEST_ROWS:-1000}"
{
  echo "CREATE DATABASE IF NOT EXISTS sbtest"
  echo "CREATE TABLE IF NOT EXISTS sbtest.sbtest1 (id INTEGER PRIMARY KEY, k INTEGER NOT NULL DEFAULT 0, c TEXT NOT NULL DEFAULT '', pad TEXT NOT NULL DEFAULT '')"
  i=1
  while [ "$i" -le "$SBTEST_ROWS" ]; do
    echo "INSERT INTO sbtest.sbtest1 (id, k, c, pad) VALUES ($i, $((i % 100)), 'c$i', 'p$i')"
    i=$((i + 1))
  done
} | "$CLI_BIN" soak --port "$PORT" --user root > /dev/null
SBTEST_COUNT="$(echo "SELECT COUNT(*) FROM sbtest.sbtest1" | query_scalar)"
case "$SBTEST_COUNT" in
  ''|*[!0-9]*) fail "sbtest.sbtest1 was not provisioned — the workload would fail every query" ;;
esac
[ "$SBTEST_COUNT" -ge 1 ] || fail "sbtest.sbtest1 is empty — the workload would fail every query"
log "sbtest1 rows: $SBTEST_COUNT"

# --------------------------------------------------------------- query files
SOAK_DIR="$RESULTS_DIR/soak"
mkdir -p "$SOAK_DIR"
printf '%s\n' \
  'SELECT COUNT(*) FROM sbtest.sbtest1' \
  'SELECT * FROM sbtest.sbtest1 WHERE id = 1' \
  'SELECT * FROM sbtest.sbtest1 WHERE id BETWEEN 1 AND 100' \
  'SELECT id, k, c, pad FROM sbtest.sbtest1 WHERE k = 500' \
  'SELECT SUM(k), AVG(k), MIN(k), MAX(k) FROM sbtest.sbtest1' \
  > "$SOAK_DIR/queries_read.sql"
printf '%s\n' \
  'SELECT id, k FROM sbtest.sbtest1 WHERE id = 1' \
  "INSERT INTO sbtest.sbtest1 (id, k, c, pad) VALUES (999991, 1000, 'i', 'p')" \
  'DELETE FROM sbtest.sbtest1 WHERE id = 999991' \
  'SELECT COUNT(*) FROM sbtest.sbtest1' \
  "INSERT INTO sbtest.sbtest1 (id, k, c, pad) VALUES (999992, 2000, 'w', 'x')" \
  'DELETE FROM sbtest.sbtest1 WHERE id = 999992' \
  > "$SOAK_DIR/queries_write.sql"
printf '%s\n' \
  'SELECT * FROM sbtest.sbtest1 WHERE k BETWEEN 100 AND 500 ORDER BY k' \
  'SELECT AVG(k) AS avg_k, MIN(id), MAX(id), COUNT(*) FROM sbtest.sbtest1' \
  > "$SOAK_DIR/queries_olap.sql"

# ------------------------------------------------------------------ workers
WORKER_PIDS=""
start_worker() {
  local tag="$1" rate="$2" file="$3"
  "$SOAK_BIN" soak --port "$PORT" --user root \
    --duration "$DURATION_S" --rate "$rate" \
    --query-file "$file" \
    --report-interval 10 \
    > "$SOAK_DIR/${tag}_worker.log" 2>&1 &
  WORKER_PIDS="$WORKER_PIDS $!"
}
start_worker read 0 "$SOAK_DIR/queries_read.sql"
if [ "$LOAD_PROFILE" != "light" ]; then
  start_worker write 50 "$SOAK_DIR/queries_write.sql"
  start_worker olap 5 "$SOAK_DIR/queries_olap.sql"
fi
log "workers started:$WORKER_PIDS (profile=$LOAD_PROFILE duration=${DURATION_S}s)"

# ------------------------------------------------------------------ metrics
get_rss_kb()  { ps -o rss= -p "$1" 2>/dev/null | tr -d ' ' || echo 0; }
get_fd_count() { if [ -d "/proc/$1/fd" ]; then ls "/proc/$1/fd" 2>/dev/null | wc -l | tr -d ' '; else echo 0; fi; }
# Thread count: `nlwp` is a Linux-only `ps` field. On macOS `ps -o nlwp=` does
# not fail — it prints the **list of unknown field names**, which would then
# land in the metrics row as garbage (measured: "%cpu%memacflagacflgargs…").
# So: take `nlwp` only when it is numeric, else count the threads with
# `ps -M`, else report 0 rather than a non-numeric value.
get_threads()  {
  local n
  n="$(ps -o nlwp= -p "$1" 2>/dev/null | tr -d ' ')"
  case "$n" in
    ''|*[!0-9]*) ;;
    *) printf '%s' "$n"; return 0 ;;
  esac
  n="$(ps -M -p "$1" 2>/dev/null | tail -n +2 | wc -l | tr -d ' ')"
  case "$n" in
    ''|*[!0-9]*) echo 0 ;;
    *) printf '%s' "$n" ;;
  esac
}
get_wal_bytes(){ du -sk "$DATA_DIR" 2>/dev/null | awk '{print $1*1024}' || echo 0; }
get_disk_kb()  { df -k / 2>/dev/null | tail -1 | awk '{print $4}' || echo 0; }
parse_soak_qps() {
  [ -f "$1" ] || { echo "$2:--:--"; return; }
  local last
  # `-a`: the worker log contains binary bytes (the server's raw output), and
  # without it grep prints "Binary file … matches" instead of the line —
  # which then lands in the metrics row as garbage (measured on macOS).
  # The pattern selects the **report** line: the log also contains per-query
  # chatter ("SOAK: reconnected (total N)"), and `tail -1` over all SOAK:
  # lines picks that chatter instead of the report.
  last="$(grep -a 'SOAK: [0-9]* queries' "$1" 2>/dev/null | tail -1)"
  [ -n "$last" ] || { echo "$2:--:--"; return; }
  local q p
  q="$(printf '%s' "$last" | sed 's/.*SOAK:[[:space:]]*\([0-9][0-9]*\).*/\1/')"
  # The report line reads "SOAK: 11049 queries, 0 errors, 220.9 qps, elapsed 50s,
  # reconnects 0" — the qps value **precedes** the "qps" tag, so the old
  # "qps, <value>" pattern never matched and the whole line leaked into the
  # metrics column. The pattern is POSIX-safe (`\+` is a GNU extension and
  # silently never matches on BSD sed — measured).
  p="$(printf '%s' "$last" | sed -n 's/.*[^0-9.]\([0-9.][0-9.]*\)[[:space:]]*qps.*/\1/p')"
  echo "$2:${q:-0}:${p:-0}"
}

METRICS="$RESULTS_DIR/metrics.csv"
echo "ts,elapsed_s,rss_kb,fd,threads,wal_bytes,disk_kb,queries_ok,r_qps,w_qps,o_qps" > "$METRICS"
END_TS=$(( $(date +%s) + DURATION_S ))
START_TS=$(date +%s)
SAMPLE=0
QUERIES_OK=0
FAILS=0
SERVER_DIED=0
while [ "$(date +%s)" -lt "$END_TS" ]; do
  SAMPLE=$(( SAMPLE + 1 ))
  TS=$(date +%s)
  ELAPSED=$(( TS - START_TS ))
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    SERVER_DIED=1
    break
  fi
  if echo "SELECT $SAMPLE" | "$CLI_BIN" soak --port "$PORT" --user root > /dev/null 2>&1; then
    QUERIES_OK=$(( QUERIES_OK + 1 ))
  else
    FAILS=$(( FAILS + 1 ))
  fi
  R_DATA="$(parse_soak_qps "$SOAK_DIR/read_worker.log" r)"
  W_DATA="$(parse_soak_qps "$SOAK_DIR/write_worker.log" w)"
  O_DATA="$(parse_soak_qps "$SOAK_DIR/olap_worker.log" o)"
  echo "$TS,$ELAPSED,$(get_rss_kb "$SERVER_PID"),$(get_fd_count "$SERVER_PID"),$(get_threads "$SERVER_PID"),$(get_wal_bytes),$(get_disk_kb),$QUERIES_OK,${R_DATA#*:},${W_DATA#*:},${O_DATA#*:}" >> "$METRICS"
  sleep 5
done

[ "$SERVER_DIED" = "0" ] || fail "the server died during the segment (see $RESULTS_DIR/server.log)"

for pid in $WORKER_PIDS; do
  wait "$pid" || true
done

# Each worker must have produced its report, and the workload must have
# actually SUCCEEDED: a runner that reports qps while every query errors is
# the "exit 0 ≠ the arm really ran" failure mode.
for logfile in "$SOAK_DIR"/*_worker.log; do
  [ -f "$logfile" ] || continue
  if ! grep -aq 'SOAK:' "$logfile"; then
    tail -20 "$logfile" >&2
    fail "worker $(basename "$logfile") produced no SOAK: report"
  fi
  last_report="$(grep -a 'SOAK: [0-9]* queries' "$logfile" | tail -1)"
  errs="$(printf '%s' "$last_report" | sed -n 's/.*queries, \([0-9]*\) errors.*/\1/p')"
  case "$errs" in
    ''|*[!0-9]*) fail "cannot read the error count from $(basename "$logfile")" ;;
  esac
  if [ "$errs" -ne 0 ]; then
    tail -20 "$logfile" >&2
    fail "worker $(basename "$logfile") reported $errs query errors — the workload did not really run"
  fi
done

SAMPLES=$(grep -c "" "$METRICS")
[ "$SAMPLES" -ge 2 ] || fail "metrics.csv has no samples ($SAMPLES lines)"

# Every reading must be numeric. A metric column that carries a log line
# (or a `ps` field list) is a broken instrument, not a measurement — this is
# exactly how the runner's qps/threads columns were garbage before the fix.
awk -F',' 'NR==1{next} {
  for (i = 3; i <= 8; i++) if ($i !~ /^[0-9]+$/) { printf "metrics row %d field %d is not numeric: %s\n", NR, i, $i > "/dev/stderr"; exit 1 }
  for (i = 9; i <= 11; i++) if ($i !~ /^[0-9]+:[0-9.]+$/) { printf "metrics row %d field %d is not numeric: %s\n", NR, i, $i > "/dev/stderr"; exit 1 }
}' "$METRICS" || fail "metrics.csv contains a non-numeric reading (see above)"

log "--- path executable ---"
log "duration=${DURATION_S}s profile=$LOAD_PROFILE samples=$SAMPLES queries_ok=$QUERIES_OK fails=$FAILS"
log "results: $RESULTS_DIR"
exit 0
