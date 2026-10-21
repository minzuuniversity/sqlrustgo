#!/usr/bin/env bash
# RC-B2-linked regression test for Issue #4652 (CREATE PROCEDURE/FUNCTION
# fake-success pattern in CLI batch mode).
#
# Per V312-RC-GA Path B §3 PR-A6 / WP-C, v3.12.0 GA adopts the **OR-downgrade**
# path: instead of silently accepting CREATE PROCEDURE / CREATE FUNCTION in CLI
# batch mode (which would store nothing, so subsequent CALL fails with
# "Stored procedure 'p1' not found"), the CLI returns an explicit "not supported"
# error per RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §3 entry for #4652:
#
#     "FIX via WP-C, OR downgrade: v3.12 GA rejects CREATE PROCEDURE/FUNCTION
#      with explicit error; never silently accepts."
#
# This test asserts the OR-downgrade path:
#   1. Feeding `CREATE PROCEDURE p1() BEGIN SELECT 1; END` via stdin to
#      `sqlrustgo-cli sqlite --batch --mode csv /tmp/...` MUST return a non-zero
#      exit code AND a stderr message that names the limitation explicitly
#      (NOT a fake "Stored procedure not found" downstream surprise).
#   2. Same for CREATE FUNCTION.
#
# Per Round-24 Anti-Pabrication: this is the honest-failure path; no
# SUBSTANTIALLY_COMPLETE or fake-PASS markers.

set -euo pipefail

# #5406 / #5477: the engine binary was located through a hardcoded path
# inside one developer's home directory. That path exists on exactly one
# machine, so everywhere else this script printed "SKIP" and exited 0 —
# which a caller reads as "the regression passed" while nothing was tested.
#
# Now: explicit env var -> CARGO_TARGET_DIR -> repo target/ -> PATH, and a
# missing binary is disclosed as NOT-RUN with exit code 77 (not 0).
source "$(dirname "$0")/../../common/resolve_bin.sh"

TARGET_NAME="$(basename "$0" .sh)"
BIN="$(resolve_sqlrustgo_bin "$TARGET_NAME")" || exit $?

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# CASE 1: CREATE PROCEDURE via stdin MUST fail with explicit message
BATCH_SQL=$(
  cat <<'SQL'
CREATE TABLE t(x int);
INSERT INTO t VALUES (1);
CREATE PROCEDURE p1() BEGIN SELECT * FROM t; END;
SQL
)

set +e
OUT=$("$BIN" sqlite --batch --mode csv "$TMP/db1" <<< "$BATCH_SQL" 2>&1)
EC=$?
set -e

echo "CASE 1 — CREATE PROCEDURE batch result:"
echo "  exit_code = $EC"
echo "  output    = $(echo "$OUT" | head -3 | tr '\n' '|')"

if [ "$EC" -eq 0 ]; then
  echo "FAIL: CREATE PROCEDURE batch succeeded but v3.12 GA OR-downgrade requires rejection"
  exit 1
fi
# Reject is honest only if stderr explicitly names the capability
if ! echo "$OUT" | grep -iqE "create procedure|create function|stored procedure|not supported|unsupported|v3\\.1[23]"; then
  echo "FAIL: CREATE PROCEDURE rejection does not name the limitation explicitly:"
  echo "$OUT" | head -5
  exit 1
fi
echo "CASE 1 PASS — explicit OR-downgrade rejection for CREATE PROCEDURE"

# CASE 2: CREATE FUNCTION via stdin MUST also reject explicitly
FUNC_SQL=$(
  cat <<'SQL'
CREATE TABLE t(x int);
CREATE FUNCTION f1(x INTEGER) RETURNS INTEGER RETURN x + 1;
SQL
)
set +e
OUT2=$("$BIN" sqlite --batch --mode csv "$TMP/db2" <<< "$FUNC_SQL" 2>&1)
EC2=$?
set -e

echo
echo "CASE 2 — CREATE FUNCTION batch result:"
echo "  exit_code = $EC2"
echo "  output    = $(echo "$OUT2" | head -3 | tr '\n' '|')"

if [ "$EC2" -eq 0 ]; then
  echo "FAIL: CREATE FUNCTION batch succeeded but v3.12 GA OR-downgrade requires rejection"
  exit 1
fi
if ! echo "$OUT2" | grep -iqE "create function|stored procedure|not supported|unsupported|v3\\.1[23]"; then
  echo "FAIL: CREATE FUNCTION rejection does not name the limitation explicitly:"
  echo "$OUT2" | head -5
  exit 1
fi
echo "CASE 2 PASS — explicit OR-downgrade rejection for CREATE FUNCTION"

echo
echo "RESULT: both cases match v3.12 GA OR-downgrade contract for Issue #4652"
exit 0
