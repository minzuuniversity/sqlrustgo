#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：归档候选（archive-candidate）
# 责任任务：P0-4
# 登记说明：头部绑定历史版本线，v4.1.0 的 STAGE_CONFIG 与 CI 都不引用它 ⇒ 归档候选；删除门禁脚本属门禁清单变更，须 owner 批准
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
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