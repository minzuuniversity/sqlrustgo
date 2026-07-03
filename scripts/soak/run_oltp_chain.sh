#!/usr/bin/env bash
# run_oltp_chain.sh — Run all 4 OLTP workload modes in sequence
# Usage:
#   bash scripts/soak/run_oltp_chain.sh --level=small
#   bash scripts/soak/run_oltp_chain.sh --level=medium --host=127.0.0.1 --port=3396
#
# Runs:
#   1. oltp_read_only  (15 min) — read baseline
#   2. oltp_read_write (15 min) — realistic OLTP mix
#   3. oltp_update_index (15 min) — hot row updates
#   4. oltp_write_only (15 min) — WAL/redo pressure
#
# Total: 1h for small, 2h for medium, 4h for large
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Defaults
HOST="127.0.0.1"
PORT=3396
USER="root"
PASSWORD=""
LEVEL="small"
CONCURRENCY=16
OUTPUT_DIR="soak_results"

# Duration per mode (seconds)
case "$LEVEL" in
    small)
        PER_MODE=900    # 15 min each = 1h total
        ;;
    medium)
        PER_MODE=1800   # 30 min each = 2h total
        ;;
    large)
        PER_MODE=3600   # 1h each = 4h total
        ;;
    *) echo "ERROR: --level must be small|medium|large, got '$LEVEL'"; exit 1 ;;
esac

while [[ $# -gt 0 ]]; do
    case "$1" in
        --host=*)    HOST="${1#*=}"; shift ;;
        --host)      HOST="$2"; shift 2 ;;
        --port=*)    PORT="${1#*=}"; shift ;;
        --port)      PORT="$2"; shift 2 ;;
        --user=*)    USER="${1#*=}"; shift ;;
        --user)      USER="$2"; shift 2 ;;
        --password=*) PASSWORD="${1#*=}"; shift ;;
        --password)  PASSWORD="$2"; shift 2 ;;
        --level=*)   LEVEL="${1#*=}"; shift ;;
        --level)     LEVEL="$2"; shift 2 ;;
        --concurrency=*) CONCURRENCY="${1#*=}"; shift ;;
        --concurrency)   CONCURRENCY="$2"; shift 2 ;;
        --output-dir=*)  OUTPUT_DIR="${1#*=}"; shift ;;
        --output-dir)    OUTPUT_DIR="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo "=== OLTP Chain Soak Test ==="
echo "  Level:        $LEVEL"
echo "  Per-mode:     ${PER_MODE}s ($((PER_MODE/60)) min)"
echo "  Total:        $((PER_MODE*4/60)) min"
echo "  Concurrency:  $CONCURRENCY"
echo "  Target:       $HOST:$PORT/$USER"
echo "  Output:       $OUTPUT_DIR"
echo ""

# Ensure data is seeded
echo "=== Checking data ==="
ROW_COUNT=$(mysql -h "$HOST" -P "$PORT" -u "$USER" ${PASSWORD:+-p$PASSWORD} -N -e "SELECT COUNT(*) FROM customers" 2>/dev/null || echo "0")
if [[ "$ROW_COUNT" -lt 100 ]]; then
    echo "  Insufficient data ($ROW_COUNT rows). Seeding..."
    bash "$SCRIPT_DIR/prepare_oltp_data.sh" --host="$HOST" --port="$PORT" --user="$USER" ${PASSWORD:+--password=$PASSWORD} --level="$LEVEL"
else
    echo "  Data OK ($ROW_COUNT customers)"
fi
echo ""

# Run each mode
MODES=("oltp_read_only" "oltp_read_write" "oltp_update_index" "oltp_write_only")
PASS=0
FAIL=0

for mode in "${MODES[@]}"; do
    echo ""
    echo "============================================"
    echo "  Running mode: $mode (${PER_MODE}s)"
    echo "============================================"
    echo ""

    if python3 "$SCRIPT_DIR/oltp_soak_driver.py" \
        --host="$HOST" --port="$PORT" --user="$USER" ${PASSWORD:+--password=$PASSWORD} \
        --mode="$mode" --concurrency="$CONCURRENCY" \
        --duration="$PER_MODE" --level="$LEVEL" \
        --output-dir="$OUTPUT_DIR"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        echo "  [WARN] Mode $mode exited with non-zero status"
    fi
done

echo ""
echo "============================================"
echo "  Chain Summary"
echo "============================================"
echo "  Modes passed: $PASS / ${#MODES[@]}"
echo "  Modes failed: $FAIL / ${#MODES[@]}"
echo "  Output:       $OUTPUT_DIR"
echo "============================================"

exit $FAIL
