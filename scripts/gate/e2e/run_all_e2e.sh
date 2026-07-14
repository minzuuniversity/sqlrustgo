#!/usr/bin/env bash
# Run all RC E2E scenarios (8 RC scenarios in tests/e2e/, named by scenario)
# Usage: ./scripts/gate/e2e/run_all_e2e.sh [server_port] [srv_bin]
set -euo pipefail

SERVER_PORT="${1:-3307}"
SRV_BIN="${2:-./target/release/sqlrustgo}"
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)/tests/e2e"
PASS=0
FAIL=0
TOTAL=0

echo "========================================"
echo "v3.10.0 E2E Scenario Suite"
echo "Date: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "Server port: $SERVER_PORT"
echo "========================================"
echo ""

for script in "$DIR"/startup_connect.sh "$DIR"/tpch_sf01.sh "$DIR"/kill9_recovery.sh \
              "$DIR"/alter_rename.sh "$DIR"/rollback_mvcc.sh "$DIR"/union_set_ops.sh \
              "$DIR"/backup_restore.sh "$DIR"/sysbench_wired.sh; do
    NAME=$(basename "$script" .sh)
    echo ""
    echo "--- Running $NAME ---"
    TOTAL=$((TOTAL + 1))
    
    if bash "$script" "$SERVER_PORT" "$SRV_BIN"; then
        echo "  ✅ $NAME PASS"
        PASS=$((PASS + 1))
    else
        echo "  ❌ $NAME FAIL"
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "========================================"
echo "E2E Suite Results: $PASS PASS, $FAIL FAIL, $TOTAL total"
echo "========================================"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
