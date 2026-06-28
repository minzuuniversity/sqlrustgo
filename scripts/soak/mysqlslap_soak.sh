#!/usr/bin/env bash
# mysqlslap_soak.sh — Standardized soak test runner using mariadb-slap / mysqlslap
# Usage:
#   bash scripts/soak/mysqlslap_soak.sh --level=30m
#   bash scripts/soak/mysqlslap_soak.sh --level=30m --auto-generate
#   bash scripts/soak/mysqlslap_soak.sh --level=4h --host=127.0.0.1 --port=3396
#
# Layer 2 (TPC-H):  Default mode — uses --query=tpch_queries.sql
# Layer 3 (Auto):  --auto-generate mode — uses --auto-generate-sql
#
# Produces:
#   <output-dir>/SoakReport.json   — standardized JSON report
#   <output-dir>/metrics.csv       — procfs RSS/FD samples
#   <output-dir>/mysqlslap.log    — mysqlslap stdout

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# ── Defaults ────────────────────────────────────────────────────────────────
LEVEL="${LEVEL:-30m}"          # 30m | 4h
HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-3396}"
USER="${USER:-root}"
PASSWORD="${PASSWORD:-}"
OUTPUT_DIR="${OUTPUT_DIR:-soak_results}"
AUTO_GENERATE=0
DRY_RUN=0

# Fixed parameters
CONCURRENCY=16
METRICS_INTERVAL=30

# ── CLI ────────────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --level=*)   LEVEL="${1#*=}";         shift ;;
        --host=*)    HOST="${1#*=}";          shift ;;
        --port=*)    PORT="${1#*=}";          shift ;;
        --user=*)    USER="${1#*=}";          shift ;;
        --password=*) PASSWORD="${1#*=}";    shift ;;
        --output-dir=*) OUTPUT_DIR="${1#*=}"; shift ;;
        --level)   LEVEL="$2";         shift 2 ;;
        --host)    HOST="$2";          shift 2 ;;
        --port)    PORT="$2";          shift 2 ;;
        --user)    USER="$2";          shift 2 ;;
        --password) PASSWORD="$2";    shift 2 ;;
        --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
        --auto-generate) AUTO_GENERATE=1; shift ;;
        --dry-run) DRY_RUN=1;         shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ── Level → iterations mapping ─────────────────────────────────────────────
case "$LEVEL" in
    30m)  ITERATIONS=60 ;;
    4h)   ITERATIONS=480 ;;
    *)    echo "ERROR: --level must be 30m or 4h, got '$LEVEL'"; exit 1 ;;
esac

# ── Probe for mysqlslap ─────────────────────────────────────────────────────
SLAP_BIN=""
for bin in mariadb-slap mysqlslap; do
    if command -v "$bin" &>/dev/null; then
        SLAP_BIN="$bin"
        break
    fi
done
if [[ -z "$SLAP_BIN" ]]; then
    echo "ERROR: neither mariadb-slap nor mysqlslap found in PATH"
    exit 1
fi

# ── Output dir ─────────────────────────────────────────────────────────────
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RUN_DIR="${OUTPUT_DIR}/${LEVEL}_${TIMESTAMP}"
mkdir -p "$RUN_DIR"

METRICS_CSV="${RUN_DIR}/metrics.csv"
SLAP_LOG="${RUN_DIR}/mysqlslap.log"

# ── Build mysqlslap command ─────────────────────────────────────────────────
SLAP_CMD=("$SLAP_BIN"
    --host="$HOST"
    --port="$PORT"
    --user="$USER"
    --concurrency="$CONCURRENCY"
    --iterations="$ITERATIONS"
    --verbose
)

if [[ "$AUTO_GENERATE" == "1" ]]; then
    # Layer 3: auto-generate schema
    SCHEMA_NAME="sbtest"
    SLAP_CMD+=(
        --auto-generate-sql
        --auto-generate-sql-load-type=mixed
        --auto-generate-sql-write-number=1000
        --number-of-queries=10000
        --create-schema="$SCHEMA_NAME"
    )
    MODE="autoschema"
else
    # Layer 2: TPC-H query file
    SCHEMA_NAME="tpch_sf01"
    QUERY_FILE="${SCRIPT_DIR}/tpch_queries.sql"
    if [[ ! -f "$QUERY_FILE" ]]; then
        echo "ERROR: TPC-H query file not found: $QUERY_FILE"
        exit 1
    fi
    SLAP_CMD+=(
        --query="$QUERY_FILE"
        --create-schema="$SCHEMA_NAME"
    )
    MODE="tpch"
fi

echo "=== Soak Test Runner ==="
echo "  Mode:        $MODE"
echo "  Level:       $LEVEL"
echo "  Concurrency: $CONCURRENCY"
echo "  Iterations:  $ITERATIONS"
echo "  Host:        $HOST:$PORT"
echo "  Schema:      $SCHEMA_NAME"
echo "  Slap bin:    $SLAP_BIN"
echo "  Output dir:  $RUN_DIR"
echo ""

# ── Find server PID ──────────────────────────────────────────────────────────
find_server_pid() {
    local port="$1"
    # Try lsof first, fall back to /proc scan
    if command -v lsof &>/dev/null; then
        lsof -ti ":$port" 2>/dev/null | head -1 || echo ""
    else
        for pid in /proc/[0-9]*; do
            if grep -q "sqlrustgo.*serve.*--port.*$port\|sqlrustgo.*$port" "$pid/cmdline" 2>/dev/null; then
                basename "$pid"
                break
            fi
        done
    fi
}

SERVER_PID=$(find_server_pid "$PORT")
if [[ -z "$SERVER_PID" ]]; then
    echo "WARNING: Could not find sqlrustgo server PID on port $PORT"
    echo "Metrics sampling will be skipped unless you start the server before running this script"
    SERVER_PID=""
fi

# ── Dry run ─────────────────────────────────────────────────────────────────
if [[ "$DRY_RUN" == "1" ]]; then
    echo "DRY RUN — command that would be executed:"
    echo "  ${SLAP_CMD[*]}"
    echo "  Metrics sampling: ${SERVER_PID:-<none>}"
    exit 0
fi

# ── Start metrics sampler ────────────────────────────────────────────────────
METRICS_PID=""
if [[ -n "$SERVER_PID" && -d "/proc/$SERVER_PID" ]]; then
    echo "Starting metrics sampler for PID=$SERVER_PID ..."
    bash "$SCRIPT_DIR/sample_metrics.sh" \
        --pid "$SERVER_PID" \
        --output "$METRICS_CSV" \
        --interval "$METRICS_INTERVAL" &
    METRICS_PID=$!
    echo "  Metrics PID=$METRICS_PID, interval=${METRICS_INTERVAL}s"
    echo "  → $METRICS_CSV"
else
    echo "Server PID not found; skipping metrics sampling"
    touch "$METRICS_CSV"
fi

# ── Run mysqlslap ────────────────────────────────────────────────────────────
echo ""
echo "Starting mysqlslap (this will run for ~${LEVEL}) ..."
echo "Command: ${SLAP_CMD[*]}"
echo ""

START_TIME=$(date +%s)
set +e
"${SLAP_CMD[@]}" 2>&1 | tee "$SLAP_LOG"
SLAP_EXIT=$?
set -e
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

echo ""
echo "mysqlslap finished (exit=$SLAP_EXIT) in ${ELAPSED}s"

# ── Stop metrics sampler ────────────────────────────────────────────────────
if [[ -n "$METRICS_PID" ]]; then
    kill "$METRICS_PID" 2>/dev/null || true
    wait "$METRICS_PID" 2>/dev/null || true
fi

# ── Generate SoakReport.json ─────────────────────────────────────────────────
REPORT_JSON="${RUN_DIR}/SoakReport.json"

python3 - "$METRICS_CSV" "$SLAP_LOG" "$LEVEL" "$CONCURRENCY" "$SCHEMA_NAME" "$ELAPSED" <<'PYEOF'
import sys
import json
import re
import os

metrics_csv = sys.argv[1]
slap_log = sys.argv[2]
level = sys.argv[3]
concurrency = int(sys.argv[4])
schema = sys.argv[5]
elapsed = int(sys.argv[6])

# Parse procfs CSV
rss_baseline = None
rss_final = None
fd_baseline = None
fd_final = None
num_samples = 0

if os.path.exists(metrics_csv):
    with open(metrics_csv) as f:
        lines = f.readlines()
    # Skip header
    data_lines = [l for l in lines if l.strip() and not l.startswith("timestamp")]
    num_samples = len(data_lines)
    if data_lines:
        # First data row = baseline
        parts0 = data_lines[0].strip().split(",")
        if len(parts0) >= 3:
            rss_baseline = int(parts0[1]) * 1024  # KB → bytes
            fd_baseline = int(parts0[2])
        # Last data row = final
        parts_last = data_lines[-1].strip().split(",")
        if len(parts_last) >= 3:
            rss_final = int(parts_last[1]) * 1024
            fd_final = int(parts_last[2])

# Parse mysqlslap stdout for queries and errors
queries_executed = 0
errors = 0
p50 = 0.0
p99 = 0.0

if os.path.exists(slap_log):
    content = open(slap_log).read()

    # Extract query count from iteration report
    # Format: "Benchmark run: 60 iterations..." or "Iterations: 60"
    iter_match = re.search(r'Iterations:\s*(\d+)', content)
    if iter_match:
        iterations = int(iter_match.group(1))
        # Estimate total queries (concurrency × queries_per_iteration)
        # mysqlslap doesn't print exact query count in verbose mode
        # So we use iterations as proxy; this will be refined
        queries_executed = iterations * concurrency  # rough estimate

    # Extract avg latency
    avg_match = re.search(r'Average.*?(\d+\.?\d*)\s*ms', content)
    p50_match = re.search(r'p50.*?(\d+\.?\d*)\s*ms', content, re.IGNORECASE)
    p99_match = re.search(r'p99.*?(\d+\.?\d*)\s*ms', content, re.IGNORECASE)

    if avg_match:
        p50 = float(avg_match.group(1))
    if p50_match:
        p50 = float(p50_match.group(1))
    if p99_match:
        p99 = float(p99_match.group(1))

# Compute growth
memory_growth_pct = 0.0
fd_growth = 0
if rss_baseline and rss_final:
    memory_growth_pct = ((rss_final - rss_baseline) / rss_baseline) * 100.0 if rss_baseline > 0 else 0.0
if fd_baseline is not None and fd_final is not None:
    fd_growth = fd_final - fd_baseline

alert_triggered = (
    memory_growth_pct >= 10.0 or
    fd_growth >= 5
)

report = {
    "level": level,
    "duration_seconds": elapsed,
    "concurrency": concurrency,
    "schema": schema,
    "queries_executed": queries_executed,
    "errors": errors,
    "memory_baseline_bytes": rss_baseline or 0,
    "memory_final_bytes": rss_final or 0,
    "memory_growth_pct": round(memory_growth_pct, 4),
    "fd_baseline": fd_baseline or 0,
    "fd_final": fd_final or 0,
    "fd_growth": fd_growth,
    "p50_latency_ms": round(p50, 4),
    "p99_latency_ms": round(p99, 4),
    "alert_triggered": alert_triggered,
    "num_metrics_samples": num_samples,
}

print(json.dumps(report, indent=2))

# Write to REPORT_JSON env var path
report_path = os.environ.get("REPORT_JSON", "")
if report_path:
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)
PYEOF

REPORT_EXIT=$?

# Move report to canonical location
if [[ $REPORT_EXIT -eq 0 ]]; then
    echo ""
    echo "=== Soak Report ==="
    cat "$REPORT_JSON"
    echo ""
    echo "Report: $REPORT_JSON"
else
    echo "WARNING: report generation failed (exit=$REPORT_EXIT)"
fi

# ── Summary ─────────────────────────────────────────────────────────────────
echo ""
echo "=== Soak Run Summary ==="
echo "  Mode:        $MODE"
echo "  Level:       $LEVEL"
echo "  Duration:    ${ELAPSED}s (wall clock)"
echo "  Samples:     $num_samples procfs samples"
echo "  Exit:       mysqlslap=$SLAP_EXIT report=$REPORT_EXIT"

if [[ -f "$REPORT_JSON" ]]; then
    ALERT=$(python3 -c "import json; d=json.load(open('$REPORT_JSON')); print(d.get('alert_triggered','?'))" 2>/dev/null || echo "?")
    MEM=$(python3 -c "import json; d=json.load(open('$REPORT_JSON')); print(d.get('memory_growth_pct','?'))" 2>/dev/null || echo "?")
    FDG=$(python3 -c "import json; d=json.load(open('$REPORT_JSON')); print(d.get('fd_growth','?'))" 2>/dev/null || echo "?")
    echo "  Memory Δ:    ${MEM}% (alert threshold: 10%)"
    echo "  FD Δ:       ${FDG} (alert threshold: +5)"
    echo "  Alert:      $ALERT"
fi

echo "  Output dir:  $RUN_DIR"
echo ""
echo "DONE"
exit 0
