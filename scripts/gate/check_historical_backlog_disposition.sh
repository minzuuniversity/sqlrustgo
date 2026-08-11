#!/usr/bin/env bash
#
# check_historical_backlog_disposition.sh — wrapper for V312-20 / #3907
#
# Calls check_historical_backlog_disposition.py (with PyYAML preprocessing
# for date/hex tokens that confuse Python 3.14's stricter tokenizer)
# plus git reachability checks for claimed closing_merge_commits.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
pass() { echo "[PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "[FAIL] $1"; FAIL=$((FAIL + 1)); }

YAML="docs/releases/v3.12.0/historical-backlog-disposition.yml"
PY="scripts/gate/check_historical_backlog_disposition.py"

if ! command -v python3 >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/python3" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

if ! command -v python3 >/dev/null 2>&1; then
  fail "python3 not found in PATH"
  exit 1
fi
pass "python3 available"

if ! python3 -c "import yaml" 2>/dev/null; then
  fail "PyYAML not available; install: pip3 install pyyaml"
  exit 1
fi
pass "PyYAML available"

# Run the Python schema validator
python3 "$PY" "$YAML"
PY_STATUS=$?

if [ $PY_STATUS -eq 0 ]; then
  pass "YAML schema validation passed (89 items, all dispositions in allowed)"
else
  fail "YAML schema validation failed"
fi

# round-9: each closing_merge_commit must be reachable from HEAD
# FIX-TLS-WRITE-BLOCK closure evidence
for sha in \
  6bec8b4e69feb43d838c6f23db3bd41a30da0e11 \
  3adbfa597bf14687f224c4ede152eabd6eb0f807 \
  ecff2f10503959b8715e8ecba055985c8696c92a
do
  if git merge-base --is-ancestor "$sha" HEAD 2>/dev/null; then
    pass "FIX-TLS closing commit reachable from HEAD: ${sha:0:7}..."
  else
    fail "FIX-TLS closing commit NOT reachable from HEAD: ${sha:0:7}..."
  fi
done

echo
echo "Results: PASS=$PASS, FAIL=$FAIL"
if [ $FAIL -eq 0 ]; then
  echo "[PASS] V312-20 Historical Backlog Disposition gate PASSED"
  exit 0
fi
echo "[FAIL] V312-20 Historical Backlog Disposition gate FAILED"
exit 1