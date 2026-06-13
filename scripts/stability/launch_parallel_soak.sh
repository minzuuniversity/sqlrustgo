#!/bin/bash
# launch_parallel_soak.sh - Run multiple wired-soak instances in parallel
#                            across distinct ports to cover the 0.5/1/2/4/8/12/16/24/48/72h
#                            duration ladder in minimum wall-clock time.
#
# v3.9.0 wired soak helper. NOT a "self-written stability program" — it
# orchestrates real run_wired_soak.sh processes (each with its own real
# sqlrustgo-mysql-server binary, sysbench process, TPC-H rotation).
#
# Companion scripts:
#   - test_integration_5min.sh (PR #3380): 60s architecture smoke
#   - run_wired_soak.sh: parameterized 0.5..72h soak driver
#   - load_tpch_fixture.sh, tpch_22_rotate.sh: TPC-H helpers
#
# Strategy:
#   - Each instance = separate port (auto-assigned from base)
#   - Each instance = separate data dir + results dir
#   - Each instance = separate binary process
#   - Standard set {0.5, 1, 2, 4, 8, 12, 16, 24, 48, 72} partitioned
#     into 4 phases for minimum wall-clock coverage:
#       PHASE_SHORT = "0.5 1 2"   (3 instances, ~2h wall)
#       PHASE_A     = "4 8 12"    (3 instances, ~12h wall)
#       PHASE_B     = "16 24"     (2 instances, ~24h wall)
#       PHASE_C     = "48 72"     (2 instances, ~72h wall)
#     Total wall-clock = max(72h) = 72h. Without parallelism = 184h.
#
# Usage:
#   PHASE_SHORT="0.5 1 2" BASE_PORT=3500 \
#       bash scripts/stability/launch_parallel_soak.sh
#
# Environment variables:
#   PHASE / PHASE_SHORT / PHASE_A / PHASE_B / PHASE_C
#                            Space-separated hours              (defaults: PHASE_SHORT="0.5 1 2")
#   BASE_PORT                 First port to allocate             (default 3500)
#   BINARY                    Override server binary             (default ./target/release/sqlrustgo-mysql-server)
#   FIXTURE                   none|tpch-tiny|tpch-sf001          (default tpch-sf001)
#   TPCH_ROTATE               0 or 1                              (default 1)
#   THREADS                   sysbench threads                    (default 4 — short runs need fewer)
#   STATUS_FILE               PID map output                      (default test_results/parallel_soak_status.json)
#   WAIT                      1 = wait for all to finish          (default 0)
#   STOP                      1 = stop all running instances      (mutually exclusive w/ WAIT)
#
# Maintainer: Hermes Agent
# Last touched: 2026-06-14

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# PHASE resolution: PHASE wins, else PHASE_SHORT (default "0.5 1 2"),
# else PHASE_A/B/C concatenation.
if [ -n "${PHASE:-}" ]; then
    PHASE_RESOLVED="$PHASE"
elif [ -n "${PHASE_SHORT:-}" ] || [ -n "${PHASE_A:-}" ] || [ -n "${PHASE_B:-}" ] || [ -n "${PHASE_C:-}" ]; then
    PHASE_RESOLVED="${PHASE_SHORT:-} ${PHASE_A:-} ${PHASE_B:-} ${PHASE_C:-}"
    # collapse runs of whitespace
    PHASE_RESOLVED=$(echo "$PHASE_RESOLVED" | tr -s ' ' | sed 's/^ *//; s/ *$//')
else
    PHASE_RESOLVED="0.5 1 2"
fi

BASE_PORT="${BASE_PORT:-3500}"
FIXTURE="${FIXTURE:-tpch-sf001}"
TPCH_ROTATE="${TPCH_ROTATE:-1}"
THREADS="${THREADS:-4}"
STATUS_FILE="${STATUS_FILE:-test_results/parallel_soak_status.json}"
WAIT="${WAIT:-0}"
STOP="${STOP:-0}"

if [ "$STOP" = "1" ]; then
    echo "[stop] reading $STATUS_FILE and killing all instances..."
    if [ ! -f "$STATUS_FILE" ]; then
        echo "  no status file; nothing to stop" >&2
        exit 0
    fi
    python3 -c "
import json, os, sys
try:
    with open('$STATUS_FILE') as f: st = json.load(f)
except Exception as e:
    print('  bad status file:', e); sys.exit(0)
for h, info in st.get('instances', {}).items():
    for label, pid in (info.get('pids') or {}).items():
        try:
            os.kill(int(pid), 15)
            print(f'  killed {h}/{label} pid={pid}')
        except ProcessLookupError:
            print(f'  already dead {h}/{label} pid={pid}')
" 2>&1 || true
    exit 0
fi

# Auto-detect binary
if [ -n "${BINARY:-}" ]; then
    SQLRUSTGO_BIN="$BINARY"
elif [ -x ./target/release/sqlrustgo-mysql-server ]; then
    SQLRUSTGO_BIN=./target/release/sqlrustgo-mysql-server
else
    echo "WARN: no release binary; will be built by first run_wired_soak.sh" >&2
    SQLRUSTGO_BIN=./target/release/sqlrustgo-mysql-server
fi

# Pre-flight: at least N free ports from BASE_PORT
PORT_LIST=()
for h in $PHASE_RESOLVED; do
    PORT_LIST+=( "$((BASE_PORT + ${#PORT_LIST[@]}))" )
done
echo "=========================================="
echo "Parallel wired-soak launcher"
echo "=========================================="
echo "PHASE='$PHASE_RESOLVED'"
echo "BASE_PORT=$BASE_PORT  → allocated: ${PORT_LIST[*]}"
echo "FIXTURE=$FIXTURE  TPCH_ROTATE=$TPCH_ROTATE  THREADS=$THREADS"
echo "BINARY=$SQLRUSTGO_BIN"
echo "STATUS_FILE=$STATUS_FILE"
echo "WAIT=$WAIT"
echo "=========================================="

# Pre-check ports free
for p in "${PORT_LIST[@]}"; do
    if lsof -i ":$p" >/dev/null 2>&1; then
        echo "FAIL: port $p already in use (maybe prior instance)" >&2
        lsof -i ":$p" >&2
        exit 1
    fi
done

# Build STATUS scaffolding
mkdir -p "$(dirname "$STATUS_FILE")"
cat > "$STATUS_FILE" <<EOF
{
  "phase": "$PHASE_RESOLVED",
  "base_port": $BASE_PORT,
  "fixture": "$FIXTURE",
  "binary": "$SQLRUSTGO_BIN",
  "launched_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "instances": {}
}
EOF

INSTANCES_JSON_TMP="$(mktemp)"
echo "{}" > "$INSTANCES_JSON_TMP"

i=0
for h in $PHASE_RESOLVED; do
    port="${PORT_LIST[$i]}"
    i=$((i+1))
    # Sanitize HOURS for use in path: 0.5 -> 0_5
    h_safe=$(echo "$h" | tr '.' '_')
    results_dir="test_results/wired_soak_${h_safe}h_$(date +%Y%m%d_%H%M%S)_p${port}"
    log_file="$results_dir/launcher.out"
    mkdir -p "$results_dir"
    echo ""
    echo "[$h h @ port $port] launching..."
    env \
        HOURS="$h" \
        PORT="$port" \
        FIXTURE="$FIXTURE" \
        TPCH_ROTATE="$TPCH_ROTATE" \
        THREADS="$THREADS" \
        SQLRUSTGO_BIN="$SQLRUSTGO_BIN" \
        RESULTS_DIR="$results_dir" \
        nohup bash "$SCRIPT_DIR/run_wired_soak.sh" > "$log_file" 2>&1 &
    LAUNCHER_PID=$!
    echo "  launcher PID=$LAUNCHER_PID  log=$log_file"
    # Wait briefly so server's port claim doesn't race
    sleep 2
    # Update JSON
    python3 -c "
import json
with open('$INSTANCES_JSON_TMP') as f: st = json.load(f)
st.setdefault('instances', {})['$h'] = {
    'port': $port,
    'launcher_pid': $LAUNCHER_PID,
    'results_dir': '$results_dir',
    'log_file': '$log_file',
    'pids': {}
}
with open('$INSTANCES_JSON_TMP', 'w') as f: json.dump(st, f, indent=2)
"
done

# Merge into STATUS_FILE
python3 -c "
import json
with open('$STATUS_FILE') as f: top = json.load(f)
with open('$INSTANCES_JSON_TMP') as f: inst = json.load(f)
top['instances'] = inst['instances']
with open('$STATUS_FILE', 'w') as f: json.dump(top, f, indent=2)
"
rm -f "$INSTANCES_JSON_TMP"

echo ""
echo "=========================================="
echo "All instances launched. Status: $STATUS_FILE"
echo "=========================================="
python3 -c "
import json
with open('$STATUS_FILE') as f: st = json.load(f)
for h, info in st['instances'].items():
    print(f\"  {h:>5}h @ port {info['port']:<5} launcher_pid={info['launcher_pid']:<6} log={info['log_file']}\")
"

if [ "$WAIT" = "1" ]; then
    echo ""
    echo "[WAIT=1] blocking until all launchers exit (this may take hours)..."
    PIDS=()
    while read -r pid; do PIDS+=("$pid"); done < <(python3 -c "
import json
with open('$STATUS_FILE') as f: st = json.load(f)
for h, info in st['instances'].items():
    print(info['launcher_pid'])
")
    for pid in "${PIDS[@]}"; do
        echo "  waiting on launcher pid=$pid ..."
        wait "$pid" 2>/dev/null || true
    done
    echo "  all launchers exited"
fi
