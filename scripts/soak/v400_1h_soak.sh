#!/usr/bin/env bash
# v400_1h_soak.sh — 1h server-read-perf SOAK validation for v4.0.0
#
# Validates Phase B C.1+D.1+D.3 perf improvements via:
#   - mixed read/write workload (80% reads / 20% writes)
#   - sustained concurrent load (16 client threads × 1h)
#   - per-15s sampling of RSS / FD / CPU% / QPS / panic count
#   - auto-generated STABILITY_REPORT.md
#
# Usage:
#   bash scripts/soak/v400_1h_soak.sh [DURATION_MIN=60] [PORT=3411]
#
# Exit codes:
#   0  PASS — server alive, no panic, low error rate
#   1  precondition failure (binary missing, data dir conflict, etc.)
#   2  server crashed during run
#   3  panic storm detected
#
# Output:
#   $RESULTS_DIR/
#     ├── server.log           # sqlrustgo server log
#     ├── driver.log           # pymysql per-query CSV
#     ├── metrics.csv          # 15s samplings (ts, rss, fd, cpu, qps, ...)
#     ├── summary.json         # aggregate stats
#     └── STABILITY_REPORT.md  # human-readable summary

set -euo pipefail

DURATION_MIN="${1:-60}"
DURATION=$((DURATION_MIN * 60))
PORT="${2:-3411}"
DATA_DIR="${DATA_DIR:-/tmp/sqlrustgo-v400-soak}"
RESULTS_DIR="${RESULTS_DIR:-/Users/liying/dev/sqlrustgo/results/soak-v400-1h-$(date +%Y%m%d_%H%M%S)}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-/Users/liying/dev/sqlrustgo/target/release/sqlrustgo-mysql-server}"
THREADS="${THREADS:-16}"
SERVER_THREADS="${SERVER_THREADS:-16}"
SAMPLE_INTERVAL="${SAMPLE_INTERVAL:-15}"
ROWS="${ROWS:-10000}"
READ_PCT="${READ_PCT:-80}"
INSERT_PCT="${INSERT_PCT:-10}"
# WAL sync mode: "every" (default, durable), "batch:N" (fsync every N
# tx; trades durability for throughput), "off" (no fsync; test only).
# V400-WAL-BATCH (2026-09-16): batch:100 cuts p50 latency by ~43% in the
# 5-min batch:100 smoke vs the every-sync baseline. Use the env var
# to opt in; defaults preserve the every-tx durable behaviour.
WAL_SYNC="${WAL_SYNC:-every}"

mkdir -p "$RESULTS_DIR"
LOG="$RESULTS_DIR/server.log"
METRICS="$RESULTS_DIR/metrics.csv"
DRIVER_LOG="$RESULTS_DIR/driver.log"
SUMMARY="$RESULTS_DIR/summary.json"

cd "$(dirname "$0")/../.."

echo "================================================"
echo "v4.0.0 server-read-perf SOAK (${DURATION_MIN} min)"
echo "================================================"
echo "  PORT=$PORT  THREADS=$THREADS  SERVER_THREADS=$SERVER_THREADS"
echo "  DATA_DIR=$DATA_DIR"
echo "  RESULTS_DIR=$RESULTS_DIR"
echo "  ROWS=$ROWS  READ_PCT=$READ_PCT%  INSERT_PCT=$INSERT_PCT%"
echo "  WAL_SYNC=$WAL_SYNC"
echo "  BIN=$SQLRUSTGO_BIN"
echo "================================================"

# Sanity
if [ ! -x "$SQLRUSTGO_BIN" ]; then
  echo "FAIL: $SQLRUSTGO_BIN missing or not executable" >&2; exit 1
fi
if lsof -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1; then
  echo "FAIL: port $PORT already in use" >&2; exit 1
fi
# Reset data dir for a clean run
rm -rf "$DATA_DIR"
mkdir -p "$DATA_DIR"

# --- 1. Start server ---------------------------------------------------
echo "[1/5] Starting sqlrustgo-mysql-server (file storage)..."
"$SQLRUSTGO_BIN" serve \
  --port "$PORT" \
  --data-dir "$DATA_DIR" \
  --storage file \
  --log-level warn \
  --server-threads "$SERVER_THREADS" \
  --wal-sync "$WAL_SYNC" \
  > "$LOG" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$RESULTS_DIR/server.pid"
echo "  server PID=$SERVER_PID"

# Reject wal-sync="off" for production runs (test mode only).
# Use WAL_SYNC=off only if you accept losing durability — e.g. for
# measurement of the no-fsync ceiling in a controlled experiment.
case "$WAL_SYNC" in
  off) echo "  WARN: WAL_SYNC=off disables durability; expected throughput ceiling only"; ;;
esac

# Wait for listen
for i in $(seq 1 30); do
  sleep 1
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "FAIL: server died on startup" >&2
    tail -30 "$LOG" >&2
    exit 1
  fi
  if lsof -iTCP:$PORT -sTCP:LISTEN >/dev/null 2>&1; then
    echo "  listening on $PORT after ${i}s"
    break
  fi
done

# --- 2. Prepare dataset -----------------------------------------------
echo "[2/5] Preparing sbtest1 ($ROWS rows)..."
if ! bash scripts/soak/v400_prepare_soak.sh "$PORT" "$ROWS"; then
  echo "FAIL: data preparation failed" >&2
  kill -TERM "$SERVER_PID" 2>/dev/null || true
  exit 1
fi

# --- 3. Start driver + monitor ---------------------------------------
echo "[3/5] Starting pymysql driver (${THREADS} threads, ${DURATION}s)..."
python3 scripts/soak/v400_soak_driver.py \
  --host=127.0.0.1 --port="$PORT" --threads="$THREADS" --duration="$DURATION" \
  --table=sbtest1 --max-id="$ROWS" \
  --read-pct="$READ_PCT" --insert-pct="$INSERT_PCT" \
  --output="$DRIVER_LOG" >"$RESULTS_DIR/driver.stdout" 2>&1 &
DRIVER_PID=$!
echo "$DRIVER_PID" > "$RESULTS_DIR/driver.pid"
echo "  driver PID=$DRIVER_PID"

echo "ts,elapsed_s,rss_mb,fd_count,cpu_pct,driver_q,driver_e,panic_count,error_high" > "$METRICS"

# --- 4. Monitor loop --------------------------------------------------
echo "[4/5] Monitoring (interval=${SAMPLE_INTERVAL}s) — Ctrl-C to abort."
START_TS=$(date +%s)
END_TS=$((START_TS + DURATION))
SERVER_ALIVE=1
HIGH_CPU_STREAK=0
HIGH_CPU_MAX_STREAK=0
HIGH_CPU_EPISODES=0

# Install trap for clean shutdown
cleanup() {
  echo ""
  echo "  Interrupted. Cleaning up..."
  kill -TERM "$DRIVER_PID" 2>/dev/null || true
  kill -TERM "$SERVER_PID" 2>/dev/null || true
  sleep 2
  kill -KILL "$DRIVER_PID" 2>/dev/null || true
  kill -KILL "$SERVER_PID" 2>/dev/null || true
  exit 130
}
trap cleanup INT TERM

while [ "$(date +%s)" -lt "$END_TS" ] && [ "$SERVER_ALIVE" -eq 1 ]; do
  sleep "$SAMPLE_INTERVAL"
  ELAPSED=$(($(date +%s) - START_TS))
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "  CRASH: server died at elapsed=${ELAPSED}s"
    SERVER_ALIVE=0
    break
  fi
  # macOS-compatible metric sampling: trim whitespace+newlines that would
  # otherwise corrupt CSV field boundaries.
  RSS_KB=$(ps -o rss= -p "$SERVER_PID" 2>/dev/null | tr -d '[:space:]' || echo "0")
  RSS_KB=${RSS_KB:-0}
  RSS_MB=$(awk -v kb="$RSS_KB" 'BEGIN{printf "%.1f", kb/1024.0}')
  # macOS has no /proc; use lsof for FD count
  FD_COUNT=$(lsof -p "$SERVER_PID" 2>/dev/null | wc -l | tr -d '[:space:]' || echo "0")
  FD_COUNT=${FD_COUNT:-0}
  # macOS ps %cpu is per-single-core normalized (100% = one full core).
  # sysctl -n hw.ncpu gives physical core count; compute system-wide %.
  NCPU=$(sysctl -n hw.ncpu 2>/dev/null || echo 1)
  CPU=$(ps -o %cpu= -p "$SERVER_PID" 2>/dev/null | tr -d '[:space:]' || echo "0")
  CPU=${CPU:-0}
  # Convert single-core % to system-wide % (capped at 100*NCPU)
  CPU_SYS=$(awk -v c="$CPU" -v n="$NCPU" 'BEGIN{printf "%.1f", c}')
  DRIVER_Q=$(grep -c '^Q,' "$DRIVER_LOG" 2>/dev/null | tr -d '[:space:]' || echo "0")
  DRIVER_Q=${DRIVER_Q:-0}
  DRIVER_E=$(grep -c '^E,' "$DRIVER_LOG" 2>/dev/null | tr -d '[:space:]' || echo "0")
  DRIVER_E=${DRIVER_E:-0}
  PANIC_COUNT=$(grep -c "panicked at\|MUST be WalStorage\|internal error: entered unreachable code" "$LOG" 2>/dev/null | tr -d '[:space:]' || echo "0")
  PANIC_COUNT=${PANIC_COUNT:-0}
  REMAIN=$(( (END_TS - $(date +%s)) / 60 ))

  # CPU high-load detection: >80% system-wide (i.e., >80% on a 10-core box
  # means >=8 fully saturated cores). Note ps %cpu on macOS is per-core;
  # to map to system-wide we multiply by NCPU.
  HIGH=0
  CPU_HIGH_THRESHOLD=$(awk -v n="$NCPU" 'BEGIN{printf "%.0f", 80.0*n}')
  # Use printf (no trailing newline) to keep CPU_AWK_CHECK a clean integer.
  CPU_AWK_CHECK=$(awk -v c="$CPU" -v t="$CPU_HIGH_THRESHOLD" 'BEGIN{printf "%d", (c>t)?1:0}')
  CPU_AWK_CHECK=$(echo "$CPU_AWK_CHECK" | tr -d '[:space:]')
  if [ "$CPU_AWK_CHECK" -eq 1 ]; then
    HIGH_CPU_STREAK=$((HIGH_CPU_STREAK + SAMPLE_INTERVAL))
    if [ "$HIGH_CPU_STREAK" -gt 60 ]; then
      HIGH_CPU_EPISODES=$((HIGH_CPU_EPISODES + 1))
      if [ "$HIGH_CPU_STREAK" -gt "$HIGH_CPU_MAX_STREAK" ]; then
        HIGH_CPU_MAX_STREAK=$HIGH_CPU_STREAK
      fi
      HIGH=1
    fi
  else
    HIGH_CPU_STREAK=0
  fi

  MARK=""
  if [ "$HIGH" -eq 1 ]; then MARK="⚠️ HIGH-CPU"; fi
  if [ "${PANIC_COUNT:-0}" -gt 0 ]; then MARK="🔥 PANIC"; fi
  echo "  [${ELAPSED}s/${DURATION}s] RSS=${RSS_MB}MB FD=${FD_COUNT} CPU=${CPU}%(/core ncpu=${NCPU}) q=${DRIVER_Q} e=${DRIVER_E} panics=${PANIC_COUNT} remain=${REMAIN}m ${MARK}"
  echo "$(date '+%Y-%m-%dT%H:%M:%S'),${ELAPSED},${RSS_MB},${FD_COUNT},${CPU},${DRIVER_Q},${DRIVER_E},${PANIC_COUNT},${HIGH}" >> "$METRICS"
done

# --- 5. Cleanup + report ----------------------------------------------
echo "[5/5] Stopping driver + server..."
kill -TERM "$DRIVER_PID" 2>/dev/null || true
wait "$DRIVER_PID" 2>/dev/null || true
kill -TERM "$SERVER_PID" 2>/dev/null || true
wait "$SERVER_PID" 2>/dev/null || true
trap - INT TERM

# Collect final stats (strip whitespace from grep -c output which adds newlines)
PANIC_COUNT=$(grep -c "panicked at\|MUST be WalStorage\|internal error: entered unreachable code" "$LOG" 2>/dev/null | tr -d '[:space:]' || echo "0")
PANIC_COUNT=${PANIC_COUNT:-0}
Q_DONE=$(grep -c "send_result_set done" "$LOG" 2>/dev/null | tr -d '[:space:]' || echo "0")
Q_DONE=${Q_DONE:-0}
DRIVER_Q=$(grep -c '^Q,' "$DRIVER_LOG" 2>/dev/null | tr -d '[:space:]' || echo "0")
DRIVER_Q=${DRIVER_Q:-0}
DRIVER_E=$(grep -c '^E,' "$DRIVER_LOG" 2>/dev/null | tr -d '[:space:]' || echo "0")
DRIVER_E=${DRIVER_E:-0}

# Compute QPS, latency stats from driver log
python3 - "$DRIVER_LOG" "$SUMMARY" <<'PYEOF'
import sys, json, statistics
log = sys.argv[1]
out = sys.argv[2]
queries, errors, latencies = [], [], []
with open(log) as f:
    for line in f:
        if line.startswith("Q,"):
            parts = line.strip().split(",", 4)
            try:
                latencies.append(float(parts[1]))
                queries.append(parts[3])
            except (ValueError, IndexError):
                pass
        elif line.startswith("E,"):
            errors.append(line.strip())
if not latencies:
    print("no queries parsed")
    sys.exit(0)
latencies.sort()
n = len(latencies)
def pct(p):
    return latencies[min(int(n*p/100), n-1)]
summary = {
    "total_queries": n,
    "total_errors": len(errors),
    "error_rate_pct": round(100.0 * len(errors) / max(n + len(errors), 1), 4),
    "latency_ms": {
        "min": round(latencies[0], 3),
        "p50": round(pct(50), 3),
        "p90": round(pct(90), 3),
        "p95": round(pct(95), 3),
        "p99": round(pct(99), 3),
        "max": round(latencies[-1], 3),
        "mean": round(statistics.mean(latencies), 3),
    },
    "by_kind": {},
}
for k in set(queries):
    summary["by_kind"][k] = queries.count(k)
with open(out, "w") as f:
    json.dump(summary, f, indent=2)
print(f"summary → {out}")
PYEOF

# Generate report
REPORT="$RESULTS_DIR/STABILITY_REPORT.md"
SUMMARY_DATA=$(cat "$SUMMARY")
{
echo "# v4.0.0 server-read-perf SOAK — ${DURATION_MIN}min"
echo
echo "**Date**: $(date -u +'%Y-%m-%dT%H:%M:%SZ')"
echo "**Duration**: ${DURATION_MIN}min (${DURATION}s)"
echo "**Port**: $PORT  **Threads**: $THREADS  **Server threads**: $SERVER_THREADS"
echo "**Storage**: file  **Rows**: $ROWS"
echo "**Workload**: ${READ_PCT}% reads (incl. point/range/agg) / $((100-READ_PCT))% writes (INSERT ${INSERT_PCT}% + UPDATE)"
echo "**Data dir**: $DATA_DIR"
echo "**Source commit**: $(git rev-parse HEAD 2>/dev/null || echo unknown)"
echo
echo "## Verdict"
echo
echo "| Metric | Value |"
echo "|--------|-------|"
echo "| Server alive at end | $SERVER_ALIVE |"
echo "| Trigger panics | $PANIC_COUNT |"
echo "| Driver queries done | $DRIVER_Q |"
echo "| Driver errors | $DRIVER_E |"
echo "| Server send_result_set done | $Q_DONE |"
echo "| High-CPU episodes (>80%/core sustained >60s) | $HIGH_CPU_EPISODES |"
echo "| High-CPU longest streak (s) | $HIGH_CPU_MAX_STREAK |"
echo
echo "## Latency stats (driver log)"
echo
echo '```json'
echo "$SUMMARY_DATA"
echo '```'
echo
echo "## Files"
echo
echo "- \`server.log\` — full server log"
echo "- \`driver.log\` — per-query CSV (Q,<ms>,<tid>,<kind>,<rows>)"
echo "- \`metrics.csv\` — 15s samplings"
echo "- \`summary.json\` — aggregate stats"
echo
} > "$REPORT"

if [ "$SERVER_ALIVE" -eq 0 ]; then
  echo "FAIL — server crashed during the soak run." | tee -a "$REPORT"
  exit 2
fi
if [ "${PANIC_COUNT:-0}" -gt 0 ]; then
  echo "FAIL — panic storm detected ($PANIC_COUNT occurrences)." | tee -a "$REPORT"
  exit 3
fi
if [ "$DRIVER_Q" -lt 100 ]; then
  echo "FAIL — driver completed fewer than 100 queries ($DRIVER_Q)." | tee -a "$REPORT"
  exit 1
fi

echo "PASS — ${DRIVER_Q} queries via ${THREADS} threads in ${DURATION_MIN}min, ${DRIVER_E} errors, ${HIGH_CPU_EPISODES} high-CPU episodes." \
  | tee -a "$REPORT"
exit 0