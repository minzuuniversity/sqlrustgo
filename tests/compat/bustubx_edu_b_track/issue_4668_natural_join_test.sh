#!/usr/bin/env bash
# BASH regression test for Issue #4668 (V312-RC-GA PR-A3 / WP-D) —
# NATURAL JOIN + multi-column USING (col1, col2) OR-downgrade in CLI batch.
#
# Per `docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §3
# PR-A3 / WP-D entry for #4668:
#   "FIX via WP-D, OR downgrade: v3.12 GA only supports JOIN with explicit
#    ON; NATURAL JOIN and multi-column USING excluded."
#
# Adopted path:
# - NATURAL JOIN → CLI batch explicit OR-downgrade reject
# - multi-column USING (col1, col2) → CLI batch explicit OR-downgrade reject
# - single-column USING (col) → preserves GREEN (anti-regression lockdown)

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

# CASE 1 — NATURAL JOIN must reject (OR-downgrade)
SCRIPT1=$(
  cat <<'SQL'
CREATE TABLE a(id INTEGER, x INTEGER);
CREATE TABLE b(id INTEGER, y INTEGER);
INSERT INTO a VALUES (1, 10);
INSERT INTO b VALUES (1, 100);
SELECT a.id, x, y FROM a NATURAL JOIN b;
SQL
)

set +e
OUT1=$("$BIN" sqlite --batch --mode csv "$TMP/db1" <<< "$SCRIPT1" 2>&1)
EC1=$?
set -e

echo "CASE 1 — NATURAL JOIN (must reject per OR-downgrade):"
echo "  exit_code = $EC1"
if [ "$EC1" -eq 0 ]; then
  echo "FAIL: NATURAL JOIN must NOT silently succeed in CLI batch"
  exit 1
fi
if ! echo "$OUT1" | grep -iqE "4668.*OR-downgrade|NATURAL JOIN"; then
  echo "FAIL: rejection does not name #4668 / NATURAL JOIN"
  echo "$OUT1" | head -5
  exit 1
fi
echo "CASE 1 PASS"

# CASE 2 — multi-column USING must reject (OR-downgrade)
SCRIPT2=$(
  cat <<'SQL'
CREATE TABLE c(id INTEGER, x INTEGER);
CREATE TABLE d(id INTEGER, x INTEGER, y INTEGER);
INSERT INTO c VALUES (1, 10);
INSERT INTO d VALUES (1, 100, 200);
SELECT c.id, c.x, d.y FROM c JOIN d USING (id, x);
SQL
)

set +e
OUT2=$("$BIN" sqlite --batch --mode csv "$TMP/db2" <<< "$SCRIPT2" 2>&1)
EC2=$?
set -e

echo
echo "CASE 2 — multi-column USING (id, x) (must reject per OR-downgrade):"
echo "  exit_code = $EC2"
if [ "$EC2" -eq 0 ]; then
  echo "FAIL: multi-column USING must NOT silently succeed in CLI batch"
  exit 1
fi
if ! echo "$OUT2" | grep -iqE "4668.*OR-downgrade|multi-column USING"; then
  echo "FAIL: rejection does not name #4668 / multi-column USING"
  echo "$OUT2" | head -5
  exit 1
fi
echo "CASE 2 PASS"

# CASE 3 — single-column USING must still PASS (anti-regression)
SCRIPT3=$(
  cat <<'SQL'
CREATE TABLE e(id INTEGER);
CREATE TABLE f(id INTEGER);
INSERT INTO e VALUES (1);
INSERT INTO f VALUES (1);
INSERT INTO f VALUES (2);
SELECT * FROM e INNER JOIN f USING(id);
SQL
)

set +e
OUT3=$("$BIN" sqlite --batch --mode csv "$TMP/db3" <<< "$SCRIPT3" 2>&1)
EC3=$?
set -e

echo
echo "CASE 3 — single-column USING (id) (must PASS / anti-regression):"
echo "  exit_code = $EC3"

if [ "$EC3" -ne 0 ]; then
  echo "FAIL: single-column USING regressed (was GREEN before OR-downgrade)"
  echo "$OUT3" | head -5
  exit 1
fi

# Verify the join output: should produce 1 row matching id=1
if ! echo "$OUT3" | tail -1 | grep -q "^1$"; then
  echo "FAIL: single-column USING join returned unexpected output"
  echo "$OUT3" | head -5
  exit 1
fi
echo "CASE 3 PASS"

echo
echo "RESULT: matches v3.12 GA OR-downgrade contract for Issue #4668"
exit 0
