#!/usr/bin/env bash
# RC E2E scenario: kill9_recovery
# Start sqlrustgo server, insert data, kill server with -9, restart,
# verify data survives via WAL recovery.
# Start sqlrustgo server, insert data, kill server, restart, verify data survives
set -euo pipefail

SRV_BIN="${1:-./target/release/sqlrustgo}"
DATA_DIR="/tmp/e2e_wal_$$"
SERVER_PORT="3399"
PASS=0
FAIL=0

cleanup() {
    echo "Cleanup..."
    kill "$SRV_PID" 2>/dev/null || true
    sleep 1
    rm -rf "$DATA_DIR" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== E2E-03: WAL Crash Recovery ==="

# Start server
echo "--- Step 1: Start server ---"
mkdir -p "$DATA_DIR"
"$SRV_BIN" --data-dir "$DATA_DIR" --port "$SERVER_PORT" &
SRV_PID=$!
sleep 2
if kill -0 "$SRV_PID" 2>/dev/null; then echo "  PASS: Server started (PID $SRV_PID)"; PASS=$((PASS+1)); else echo "  FAIL: Server start"; FAIL=$((FAIL+1)); exit 1; fi

# Create table and insert
echo "--- Step 2: INSERT committed data ---"
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE wal_test;" 2>/dev/null || true
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D wal_test -e "CREATE TABLE crash_test (id INTEGER PRIMARY KEY, val VARCHAR(100));" 2>/dev/null || true
mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D wal_test -e "BEGIN; INSERT INTO crash_test VALUES (1, 'survived'); INSERT INTO crash_test VALUES (2, 'also_survived'); COMMIT;" 2>/dev/null
ROWS=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D wal_test -e "SELECT COUNT(*) AS cnt FROM crash_test;" -N 2>/dev/null || echo "0")
if [ "$ROWS" -ge 1 ] 2>/dev/null; then echo "  PASS: Data inserted ($ROWS rows)"; PASS=$((PASS+1)); else echo "  FAIL: Insert"; FAIL=$((FAIL+1)); fi

# kill -9
echo "--- Step 3: kill -9 ---"
kill -9 "$SRV_PID" 2>/dev/null || true
sleep 1
if ! kill -0 "$SRV_PID" 2>/dev/null; then echo "  PASS: Server killed"; PASS=$((PASS+1)); else echo "  FAIL: Server still running"; FAIL=$((FAIL+1)); fi

# Restart
echo "--- Step 4: Restart and verify WAL recovery ---"
"$SRV_BIN" --data-dir "$DATA_DIR" --port "$SERVER_PORT" &
SRV_PID=$!
sleep 3

ROWS=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D wal_test -e "SELECT COUNT(*) AS cnt FROM crash_test;" -N 2>/dev/null || echo "0")
if [ "$ROWS" -ge 2 ] 2>/dev/null; then echo "  PASS: WAL recovery ($ROWS rows survived)"; PASS=$((PASS+1)); else echo "  FAIL: WAL recovery (got $ROWS, expected >=2)"; FAIL=$((FAIL+1)); fi

# Verify data integrity
echo "--- Step 5: Data integrity ---"
VAL1=$(mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -D wal_test -e "SELECT val FROM crash_test WHERE id=1;" -N 2>/dev/null || echo "")
if [ "$VAL1" = "survived" ]; then echo "  PASS: Data integrity (id=1: $VAL1)"; PASS=$((PASS+1)); else echo "  FAIL: Data integrity ($VAL1)"; FAIL=$((FAIL+1)); fi

echo ""
echo "=== E2E-03: $PASS PASS, $FAIL FAIL ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
