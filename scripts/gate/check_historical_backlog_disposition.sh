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
#
# 2026-11-19 修正（逐条实测，不是推断）
# --------------------------------------------
# 这一段原先硬编码的三个 SHA，在当前 develop 上**全部不可达**，根因有两条：
#
#   ① GA-REWIND-1 那次 force-push 改写了 develop 历史 ⇒ 前两个 merge commit
#      的**对象本身已从仓库里消失**（git cat-file -t 报 no such object），
#      不是「可达性变了」而是「对象没了」。
#      但**修复的代码内容仍在当前树里**（crates/mysql-server/src/lib.rs 的
#      force_drain / drive_writes_only / WouldBlock 处理），
#      由历史重写后的等价提交承载。
#
#   ② 第三个 SHA **本身就是一个无效 SHA**：round-9 的闭合提交
#      （主题：fix(V312-20 round-9): FIX-TLS-WRITE-BLOCK 已修复 + 添加 V312-20 gate；
#      当前树可达副本 5890ec7de6…）在自己的提交说明里就写明
#      「发现 ecff2f105cf2a7d… 是无效 SHA」，并把 PR #3701 的真实 merge commit
#      更正为 ad68e5037…。
#
# 因此这里改为检查**当前树上真实承载这几项修复、且经 git merge-base --is-ancestor
# 实测为 HEAD 祖先**的提交：
#   04990a0d7e…  PR #3696 的内容提交：force-drain all TLS cipher records on STMT_PREPARE response
#   e94f877993…  PR #3700 的内容提交：restore blocking mode on accepted TcpStream
#   ad68e5037…   PR #3701 的**真实** merge commit（原门禁写的第三个是无效 SHA）
#   c03e76ee6…   commit：loop TlsStream write through all WouldBlock
#
# 这是**把判据指向真实存在的证据**，不是放宽判据：仍然逐条要求
# 「闭合提交必须是 HEAD 的祖先」，只是不再指向已被历史重写丢弃、
# 或当初就写错了的对象。
# 原始 SHA 与逐条对应关系登记在
# docs/releases/v3.12.0/historical-backlog-disposition.yml 的
# FIX-TLS-WRITE-BLOCK 条目（history_rewrite_note / reachable_equivalents 字段），
# 不丢失、不掩盖。
for sha in \
  04990a0d7e9d18f7f8a3d8b2b88d9c62062dc6bb \
  e94f877993ee404cf7a17c06cc58cab7c1d9796f \
  ad68e5037399cc608229e928a9966ff96b5e3295 \
  c03e76ee6833e4c0d4aa17ef41918f23bc0053de
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