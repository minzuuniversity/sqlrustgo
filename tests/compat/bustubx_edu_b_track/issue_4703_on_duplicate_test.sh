#!/usr/bin/env bash
# BASH regression test for Issue #4703 (V312-RC-GA PR-A4 / WP-A) — CLI batch
# OR-downgrade for INSERT ... ON DUPLICATE KEY UPDATE ... VALUES(col).
#
# Per V312-RC-GA Triage Plan §3 PR-A4 / WP-A entry for #4703:
#   "FIX via WP-A, OR downgrade: MySQL-style multi-column upsert excluded
#    from v3.12 GA claims."
#
# Adopted OR-downgrade path: sub-bug #1 (multi-col + VALUES()) is explicitly
# rejected in CLI batch mode with a named error.
#
# Sub-bug #2 (ON CONFLICT) and sub-bug #3 (CREATE TRIGGER ... UPDATE OF col_list)
# already work via earlier PR work; this test locks them in to prevent
# regression.

set -euo pipefail

BIN="${SQLRUSTGO_CLI:-/Users/liying/dev/sqlrustgo/target/debug/sqlrustgo}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [ ! -x "$BIN" ]; then
  echo "SKIP: $BIN not built (cargo build -p sqlrustgo-cli --all-features first)"
  exit 0
fi

# CASE 1 — sub-bug #1 (multi-col + VALUES()) MUST reject explicitly
SCRIPT1=$(
  cat <<'SQL'
CREATE TABLE m(id int, name text, val int);
INSERT INTO m VALUES (1, 'alice', 100);
INSERT INTO m VALUES (1, 'bob', 200) ON DUPLICATE KEY UPDATE name = VALUES(name), val = val + 50;
SQL
)

set +e
OUT1=$("$BIN" sqlite --batch "$TMP/db1" <<< "$SCRIPT1" 2>&1)
EC1=$?
set -e

echo "CASE 1 — INSERT ... ON DUPLICATE KEY UPDATE multi-col + VALUES():"
echo "  exit_code = $EC1"
echo "  output    = $(echo "$OUT1" | head -3 | tr '\n' '|')"

if [ "$EC1" -eq 0 ]; then
  echo "FAIL: multi-col + VALUES() must NOT silently succeed in CLI batch"
  exit 1
fi
if ! echo "$OUT1" | grep -iqE "4703.*OR-downgrade|VALUES"; then
  echo "FAIL: rejection does not name #4703 / OR-downgrade / VALUES limitation"
  echo "$OUT1" | head -5
  exit 1
fi
echo "CASE 1 PASS"

# CASE 2 — sub-bug #2 (ON CONFLICT) still works (no OR-downgrade)
SCRIPT2=$(
  cat <<'SQL'
CREATE TABLE o(id int, val int);
INSERT INTO o VALUES (1, 999) ON CONFLICT (id) DO UPDATE SET val = val + 1;
SQL
)

set +e
OUT2=$("$BIN" sqlite --batch "$TMP/db2" <<< "$SCRIPT2" 2>&1)
EC2=$?
set -e

echo
echo "CASE 2 — INSERT ... ON CONFLICT (id) DO UPDATE SET:"
echo "  exit_code = $EC2"

if [ "$EC2" -ne 0 ]; then
  echo "FAIL: ON CONFLICT path should NOT be OR-downgrade'd"
  echo "$OUT2" | head -5
  exit 1
fi
echo "CASE 2 PASS"

# CASE 3 — sub-bug #3 (CREATE TRIGGER ... UPDATE OF col_list) still works
SCRIPT3=$(
  cat <<'SQL'
CREATE TABLE t(id int, val int, note text);
CREATE TRIGGER tr AFTER UPDATE OF val, note ON t
  FOR EACH ROW BEGIN SELECT 1; END;
SQL
)

set +e
OUT3=$("$BIN" sqlite --batch "$TMP/db3" <<< "$SCRIPT3" 2>&1)
EC3=$?
set -e

echo
echo "CASE 3 — CREATE TRIGGER ... AFTER UPDATE OF multi-col:"
echo "  exit_code = $EC3"

if [ "$EC3" -ne 0 ]; then
  echo "FAIL: column-level UPDATE OF trigger should NOT be OR-downgrade'd"
  echo "$OUT3" | head -5
  exit 1
fi
echo "CASE 3 PASS"

echo
echo "RESULT: matches v3.12 GA OR-downgrade contract for Issue #4703"
exit 0
