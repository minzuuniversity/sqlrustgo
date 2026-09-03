#!/usr/bin/env bash
# BASH regression test for Issue #4626 (V312-RC-GA PR-A7) — CLI batch
# `BEGIN; SELECT FOR UPDATE; ROLLBACK` chain.
#
# Per V312-RC-GA Triage Plan §3 PR-A7 / WP-C entry for #4626:
#   "SELECT FOR UPDATE 后 ROLLBACK 报 transaction already aborted (隐式 abort)"
#
# Actual reproducer surfaces as: prior INSERT/UPDATE/DELETE in CLI batch
# opens an implicit tx that is not visible to `dispatch_one`'s tx_depth
# tracker, so explicit `BEGIN` fails with
#   "Transaction already in progress"
# and explicit `ROLLBACK` (after SELECT FOR UPDATE) fails with
#   "transaction already aborted".
#
# Fix (sqlite_mode.rs::dispatch_one): pre-flush any implicit-tx by issuing
# a no-op `engine.execute("COMMIT")` before each explicit BEGIN at
# top-level (tx_depth == 0). COMMIT is a no-op when current_tx_id is None.

set -euo pipefail

BIN="${SQLRUSTGO_CLI:-/Users/liying/dev/sqlrustgo/target/debug/sqlrustgo}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [ ! -x "$BIN" ]; then
  echo "SKIP: $BIN not built (cargo build -p sqlrustgo-cli --all-features first)"
  exit 0
fi

# CASE 1 — INSERT then BEGIN then SELECT FOR UPDATE then ROLLBACK
SCRIPT1=$(
  cat <<'SQL'
CREATE TABLE t(id int, val int);
INSERT INTO t VALUES (1, 10);
BEGIN;
SELECT * FROM t WHERE id = 1 FOR UPDATE;
ROLLBACK;
SQL
)

set +e
OUT1=$("$BIN" sqlite --batch --mode csv "$TMP/db1" <<< "$SCRIPT1" 2>&1)
EC1=$?
set -e

echo "CASE 1 — INSERT + BEGIN + SELECT FOR UPDATE + ROLLBACK:"
echo "  exit_code = $EC1"
echo "  output    = $(echo "$OUT1" | head -3 | tr '\n' '|')"

if [ "$EC1" -ne 0 ]; then
  echo "FAIL: tx-state corruption — BEGIN/SELECT FOR UPDATE/ROLLBACK chain failed"
  echo "  detail: $OUT1"
  exit 1
fi
echo "CASE 1 PASS"

# CASE 2 — empty table variant: no INSERT in batch
SCRIPT2=$(
  cat <<'SQL'
CREATE TABLE t2(id int, val int);
BEGIN;
SELECT * FROM t2 WHERE id = 1 FOR UPDATE;
ROLLBACK;
SQL
)

set +e
OUT2=$("$BIN" sqlite --batch --mode csv "$TMP/db2" <<< "$SCRIPT2" 2>&1)
EC2=$?
set -e

echo
echo "CASE 2 — BEGIN + SELECT FOR UPDATE + ROLLBACK (empty table):"
echo "  exit_code = $EC2"

if [ "$EC2" -ne 0 ]; then
  echo "FAIL: empty-table chain failed"
  echo "  detail: $OUT2"
  exit 1
fi
echo "CASE 2 PASS"

# CASE 3 — multi-statement with UPDATE before tx-control (extra implicit-tx case)
SCRIPT3=$(
  cat <<'SQL'
CREATE TABLE t3(id int, val int);
INSERT INTO t3 VALUES (1, 10), (2, 20);
UPDATE t3 SET val = val + 1 WHERE id = 1;
BEGIN;
SELECT * FROM t3 WHERE id = 1 FOR UPDATE;
ROLLBACK;
SQL
)

set +e
OUT3=$("$BIN" sqlite --batch --mode csv "$TMP/db3" <<< "$SCRIPT3" 2>&1)
EC3=$?
set -e

echo
echo "CASE 3 — INSERT + UPDATE + BEGIN + SELECT FOR UPDATE + ROLLBACK:"
echo "  exit_code = $EC3"

if [ "$EC3" -ne 0 ]; then
  echo "FAIL: tx-state corruption after UPDATE-then-BEGIN"
  echo "  detail: $OUT3"
  exit 1
fi
echo "CASE 3 PASS"

echo
echo "RESULT: all 3 cases match v3.12 GA PR-A7 contract for Issue #4626"
exit 0
