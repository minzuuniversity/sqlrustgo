#!/usr/bin/env bash
# v400_prepare_soak.sh — Prepare sbtest1 dataset for v4.0.0 SOAK validation.
#
# Connects to running sqlrustgo-mysql-server (file storage) and:
#   - Drops+creates sbtest1 (id PK, k INT, c TEXT, pad TEXT)
#   - Bulk-inserts 10_000 rows via individual INSERTs (sufficient for
#     triggering D.1 PK-column and C.1 storage write_lock paths)
#
# Usage:
#   bash scripts/soak/v400_prepare_soak.sh [PORT=3411] [ROWS=10000]
#
# Exit codes:
#   0  PASS — table ready, rows loaded
#   1  precondition failure
set -euo pipefail

PORT="${1:-3411}"
ROWS="${2:-10000}"
HOST="127.0.0.1"
USER="root"
MYSQL_CMD="mysql -h ${HOST} -P ${PORT} -u ${USER} --silent --skip-column-names"

echo "=== v400_prepare_soak: port=$PORT rows=$ROWS ==="

# Verify server reachable
if ! ${MYSQL_CMD} -e "SELECT 1+1" >/dev/null 2>&1; then
  echo "FAIL: server not reachable at ${HOST}:${PORT}" >&2
  exit 1
fi

# Create table
echo "[1/3] Creating sbtest1 table..."
${MYSQL_CMD} <<'SQL'
DROP TABLE IF EXISTS sbtest1;
CREATE TABLE sbtest1 (
    id INTEGER PRIMARY KEY,
    k INTEGER NOT NULL DEFAULT 0,
    c TEXT NOT NULL DEFAULT '',
    pad TEXT NOT NULL DEFAULT ''
);
SQL

# Bulk insert: use deterministic alphanumeric strings (no single-quote risk)
echo "[2/3] Loading $ROWS rows into sbtest1..."
START=$(date +%s)
SQL_FILE=$(mktemp)
python3 - "$ROWS" > "$SQL_FILE" <<'PYEOF'
import sys
n = int(sys.argv[1])
print("INSERT INTO sbtest1 (id, k, c, pad) VALUES", end="")
for i in range(1, n + 1):
    sep = "," if i > 1 else " "
    c = ("c" + str(i).zfill(8))[:((i * 7) % 30 + 5)]
    p = ("p" + str(i).zfill(8))[:((i * 11) % 30 + 5)]
    print(f"{sep}({i},{i % 100},'{c}','{p}')", end="")
print(";")
PYEOF
${MYSQL_CMD} < "$SQL_FILE" 2>&1 | tail -5
rm -f "$SQL_FILE"
END=$(date +%s)
echo "  Loaded $ROWS rows in $((END - START))s"

# Verify
echo "[3/3] Verifying..."
COUNT=$(${MYSQL_CMD} -e "SELECT COUNT(*) FROM sbtest1" 2>&1 | tail -1)
echo "  Row count: $COUNT"
if [ "$COUNT" != "$ROWS" ]; then
  echo "FAIL: expected $ROWS rows, got $COUNT" >&2
  exit 1
fi

# Print PK column info (sanity for D.1 fix)
echo "  Sample PK lookups:"
${MYSQL_CMD} -e "SELECT id, k, CHAR_LENGTH(c) AS c_len FROM sbtest1 WHERE id IN (1, 100, 5000, 9999)" 2>&1

echo "PASS"