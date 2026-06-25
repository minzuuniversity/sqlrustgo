#!/usr/bin/env bash
#
# scripts/wire_smoke_mysql_cli.sh
#
# Manual gate for openspec/changes/2026-06-18-wire-deprecate-eof.
# Boots sqlrustgo-mysql-server on an OS-assigned port, runs
# `mysql 8.0+ -e "SELECT 1"` against it, and asserts that the
# client returns a single row with no `ER_MALFORMED_PACKET` substring
# and no hang.
#
# Exits 0 on PASS, non-zero on any assertion failure.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$REPO_ROOT/target/release/sqlrustgo-mysql-server"
TMPDIR="$(mktemp -d)"
PORT=""

cleanup() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

# 1) Find a free port via a short-lived python listener.
PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(('127.0.0.1', 0))
print(s.getsockname()[1])
s.close()
PY
)"

# 2) Start the server in the background.
"$BIN" serve --port "$PORT" --data-dir "$TMPDIR" --verbose \
    > "$TMPDIR/server.log" 2>&1 &
SERVER_PID=$!

# 3) Wait up to 5s for the listener to come up.
for _ in $(seq 1 50); do
    if ss -lntp 2>/dev/null | grep -q ":$PORT "; then
        break
    fi
    sleep 0.1
done

if ! ss -lntp 2>/dev/null | grep -q ":$PORT "; then
    echo "FAIL: server did not start listening on port $PORT within 5s"
    echo "--- server log ---"
    cat "$TMPDIR/server.log"
    exit 1
fi

# 4) Run the mysql client.
OUT="$TMPDIR/mysql.out"
ERR="$TMPDIR/mysql.err"
set +e
timeout 10 mysql -h 127.0.0.1 -P "$PORT" -u root --ssl-mode=DISABLED \
    -e "SELECT 1" > "$OUT" 2> "$ERR"
RC=$?
set -e

if [[ $RC -ne 0 ]]; then
    echo "FAIL: mysql client exited with code $RC"
    echo "--- stdout ---"
    cat "$OUT"
    echo "--- stderr ---"
    cat "$ERR"
    exit 1
fi

# 5) Assert the output contains the row and no ER_MALFORMED_PACKET.
if grep -q "ER_MALFORMED_PACKET" "$OUT" "$ERR"; then
    echo "FAIL: ER_MALFORMED_PACKET substring present in mysql output"
    echo "--- stdout ---"
    cat "$OUT"
    echo "--- stderr ---"
    cat "$ERR"
    exit 1
fi

if ! grep -q "^1$" "$OUT"; then
    echo "FAIL: expected a row containing '1' (classic mode) or a '1' value (batch mode)"
    echo "--- stdout ---"
    cat "$OUT"
    exit 1
fi

echo "PASS: mysql 8.0+ received the SELECT 1 result without ER_MALFORMED_PACKET"
echo "--- captured row (first 5 lines) ---"
head -5 "$OUT"
exit 0
