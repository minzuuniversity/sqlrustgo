#!/usr/bin/env bash
# BASH regression test for Issue #4708 (V312-RC-GA PR-A1 / WP-A) —
# non-ASCII identifier + MySQL backtick OR-downgrade in CLI batch.
#
# Per `docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §3 PR-A1:
#   "FIX via WP-A, OR explicit downgrade in release notes:
#    v3.12.0 GA does not support non-ASCII identifiers or comments;
#    B-track teaching corpora must use ASCII identifiers."
#
# Adopted path: sub-bugs #1 (Chinese identifier) + #3 (MySQL backtick)
# OR-downgrade rejected explicitly in CLI batch mode.
# Sub-bugs #2 (Chinese comment) + #4 (double-quote identifier) already
# GREEN from prior work — anti-regression lockdown.

set -euo pipefail

BIN="${SQLRUSTGO_CLI:-/Users/liying/dev/sqlrustgo/target/debug/sqlrustgo}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if [ ! -x "$BIN" ]; then
  echo "SKIP: $BIN not built (cargo build -p sqlrustgo-cli --all-features first)"
  exit 0
fi

# CASE 1 — sub-bug #1 (Chinese identifier) MUST reject explicitly
SCRIPT1='CREATE TABLE 用户(id int, 姓名 varchar(10));
INSERT INTO 用户(姓名) VALUES ('\''张三'\'');
SELECT * FROM 用户;'

set +e
OUT1=$("$BIN" sqlite --batch "$TMP/db1" <<< "$SCRIPT1" 2>&1)
EC1=$?
set -e

echo "CASE 1 — Chinese identifier (must reject per OR-downgrade):"
echo "  exit_code = $EC1"
echo "  output    = $(echo "$OUT1" | head -3 | tr '\n' '|')"

if [ "$EC1" -eq 0 ]; then
  echo "FAIL: Chinese identifier must NOT silently succeed in CLI batch"
  exit 1
fi
if ! echo "$OUT1" | grep -iqE "4708.*OR-downgrade|non-ASCII identifiers"; then
  echo "FAIL: rejection does not name #4708 / OR-downgrade / non-ASCII"
  echo "$OUT1" | head -5
  exit 1
fi
echo "CASE 1 PASS"

# CASE 2 — sub-bug #2 (Chinese comment only) MUST still PASS (anti-regression)
SCRIPT2=$'-- 这是中文注释\nSELECT 1;'
set +e
OUT2=$("$BIN" sqlite --batch "$TMP/db2" <<< "$SCRIPT2" 2>&1)
EC2=$?
set -e

echo
echo "CASE 2 — Chinese comment only (must PASS / no panic):"
echo "  exit_code = $EC2"

if [ "$EC2" -ne 0 ]; then
  echo "FAIL: Chinese comment regressed back to panic state"
  echo "$OUT2" | head -5
  exit 1
fi
echo "CASE 2 PASS"

# CASE 3 — sub-bug #3 (MySQL backtick) MUST reject explicitly
SCRIPT3=$'CREATE TABLE t(`col` int); INSERT INTO t(`col`) VALUES (1); SELECT * FROM t;'
set +e
OUT3=$("$BIN" sqlite --batch "$TMP/db3" <<< "$SCRIPT3" 2>&1)
EC3=$?
set -e

echo
echo "CASE 3 — MySQL backtick identifier (must reject per OR-downgrade):"
echo "  exit_code = $EC3"

if [ "$EC3" -eq 0 ]; then
  echo "FAIL: MySQL backtick must NOT silently succeed in CLI batch"
  exit 1
fi
if ! echo "$OUT3" | grep -iqE "4708.*OR-downgrade|MySQL backtick"; then
  echo "FAIL: rejection does not name #4708 / MySQL backtick tag"
  echo "$OUT3" | head -5
  exit 1
fi
echo "CASE 3 PASS"

# CASE 4 — sub-bug #4 (double-quoted identifier) MUST still PASS (anti-regression)
SCRIPT4=$'CREATE TABLE t("col" int); INSERT INTO t("col") VALUES (1); SELECT "col" FROM t;'
set +e
OUT4=$("$BIN" sqlite --batch "$TMP/db4" <<< "$SCRIPT4" 2>&1)
EC4=$?
set -e

echo
echo "CASE 4 — Double-quoted identifier (must PASS / anti-regression):"
echo "  exit_code = $EC4"

if [ "$EC4" -ne 0 ]; then
  echo "FAIL: double-quoted identifier regressed"
  echo "$OUT4" | head -5
  exit 1
fi
echo "CASE 4 PASS"

echo
echo "RESULT: matches v3.12 GA OR-downgrade contract for Issue #4708"
exit 0
