#!/bin/bash
# scripts/gate/check_tpch.sh — TPC-H SF0.1 regression gate
# Must pass before PR merge (except docs-only changes)
#
# Usage: bash scripts/gate/check_tpch.sh [--fast]
#   --fast  Skip data import, assume data already loaded

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BENCH_CLI="$REPO_DIR/target/release/sqlrustgo-bench-cli"
DDL="$REPO_DIR/scripts/tpch/tpch_schema.sql"
DATA_DIR="${DATA_DIR:-/tmp/tpch-sf01}"
BASELINE="$REPO_DIR/docs/releases/v3.6.0/TPCH_BASELINE.md"
RESULT_FILE="/tmp/tpch_gate_results.json"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_fail()  { echo -e "${RED}[FAIL]${NC}  $*"; }

# Check if bench binary exists
if [[ ! -x "$BENCH_CLI" ]]; then
    log_warn "Bench CLI not found at $BENCH_CLI, building..."
    cd "$REPO_DIR"
    cargo build --release -p sqlrustgo-bench-cli
fi

# Check data exists
if [[ ! -d "$DATA_DIR" ]] || [[ -z "$(ls -A "$DATA_DIR"/*.tbl 2>/dev/null)" ]]; then
    log_info "Generating SF0.1 TPC-H data..."
    mkdir -p "$DATA_DIR"
    cd /home/openclaw/tpch-dbgen-master
    printf 'y\ny\n' | ./dbgen -f -q -s 0.1 -T r 2>/dev/null || true
    printf 'y\ny\n' | ./dbgen -f -q -s 0.1 -T n 2>/dev/null || true
    printf 'y\ny\n' | ./dbgen -f -q -s 0.1 -T c 2>/dev/null || true
    printf 'y\ny\n' | ./dbgen -f -q -s 0.1 -T s 2>/dev/null || true
    printf 'y\ny\n' | ./dbgen -f -q -s 0.1 -T P 2>/dev/null || true
    printf 'y\ny\n' | ./dbgen -f -q -s 0.1 -T O 2>/dev/null || true
    printf 'y\ny\n' | ./dbgen -f -q -s 0.1 -T L 2>/dev/null || true
    mv *.tbl "$DATA_DIR/" 2>/dev/null || true
    cd "$REPO_DIR"
fi

log_info "Running TPC-H SF0.1 gate..."

# Run all 22 queries
cd "$REPO_DIR"
    "$BENCH_CLI" \
        tpch-bench \
        --ddl "$DDL" \
        --data "$DATA_DIR" \
        --queries all \
        --iterations 1 \
        --output "$RESULT_FILE" 2>&1

# Parse results
PASS=0
FAIL=0
TOTAL_TIME=0

for q in Q1 Q2 Q3 Q4 Q5 Q6 Q7 Q8 Q9 Q10 Q11 Q12 Q13 Q14 Q15 Q16 Q17 Q18 Q19 Q20 Q21 Q22; do
    elapsed=$(python3 -c "
import json, sys
try:
    with open('$RESULT_FILE') as f:
        d = json.load(f)
    qdata = d.get('queries', [])
    for item in qdata:
        if item.get('name') == '$q':
            print(item.get('avg_ms', 0))
            break
    else:
        print('ERROR')
except Exception as e:
    print('ERROR')
" 2>/dev/null || echo "ERROR")

    if [[ "$elapsed" != "ERROR" ]]; then
        echo -e "  ${GREEN}PASS${NC}  $q: ${elapsed}ms"
        PASS=$((PASS + 1))
        TOTAL_TIME=$(python3 -c "print($TOTAL_TIME + $elapsed)" 2>/dev/null || echo "$TOTAL_TIME")
    else
        echo -e "  ${RED}FAIL${NC}  $q"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "========================================"
echo "TPC-H Gate: $PASS passed, $FAIL failed"
echo "========================================"

if [[ $FAIL -gt 0 ]]; then
    log_fail "TPC-H gate FAILED — $FAIL queries failed"
    exit 1
else
    log_info "TPC-H gate PASSED — all 22 queries succeeded"
    exit 0
fi