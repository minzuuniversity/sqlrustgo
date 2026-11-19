#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：保留并改接（keep-rewire）
# 责任任务：P0-4
# 登记说明：判据内容仍在 v4.1.0 的声称面内(台账 open issue #5226)：解析器对真脚本的差分审计仍是 A04 的核心 ⇒ 改名重挂
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# RC-B2 - V312-RC-GA: Parser real-script gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B2 Parser real-script gate | bash scripts/gate/check_v312_parser_real_scripts.sh
#    | Multi-line DDL, standalone comments, quoted identifiers,
#      Chinese comments/identifiers, basic DML parse."
#
# STATUS (2026-09-04): SKELETON — Awaiting Phase 2 implementation.
# Linked issues: #4708 (中文标识符/comments), #4696 (UPDATE w/o WHERE), #4710 (TIMESTAMPDIFF),
#                #4649 (LEFT JOIN USING — closed via PR #4732 but regression test needed)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURES="$REPO_ROOT/tests/parser_real_scripts"
EXPECTED_ARTIFACTS=(
  "$FIXTURES/manifest.yml"
  "$FIXTURES/multi_line_ddl.sql"
  "$FIXTURES/chinese_identifiers.sql"
  "$FIXTURES/quoted_identifiers.sql"
  "$FIXTURES/standalone_comments.sql"
  "$FIXTURES/basic_dml.sql"
  "$FIXTURES/oracle/parser_real.expected"
)

echo "=== RC-B2 Parser real-script gate (SKELETON) ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Repo:      $REPO_ROOT"
echo "Branch:    $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')"
echo "HEAD:      $(git rev-parse --short HEAD 2>/dev/null || echo '?')"
echo

MISSING=0
for f in "${EXPECTED_ARTIFACTS[@]}"; do
  if [ ! -f "$f" ]; then
    echo "  MISSING: $f"
    MISSING=$((MISSING+1))
  fi
done

if [ "$MISSING" -gt 0 ]; then
  echo
  echo "RESULT: STUB — $MISSING required fixture(s) missing."
  echo "ACTION REQUIRED (Phase 2):"
  echo "  - Author multi-line DDL chunk (CREATE TABLE with comments/blank lines/Chinese ids)"
  echo "  - Run via 'sqlrustgo < fixture.sql' and verify expected output"
  echo "  - Cross-check using sqlite3 oracle on equivalent SQL"
  echo "  - Bypass cases must issue-link GA-blocker #4708 / #4696"
  exit 1
fi
exit 1
