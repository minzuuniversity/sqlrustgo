#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：保留并改接（keep-rewire）
# 责任任务：P0-4
# 登记说明：判据内容仍在 v4.1.0 的声称面内(台账 open issue #5361/#5394)：LEFT JOIN 的 ON 表达式与左表行丢失仍是 v4.1.0 的 open P0 ⇒ 不能归档，改名重挂
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# RC-B5 - V312-RC-GA: Join/subquery correctness gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B5 Join/subquery correctness | bash scripts/gate/check_v312_join_subquery_semantics.sh
#    | JOIN USING/NATURAL JOIN, scalar subquery, ANY/ALL, HAVING multi-condition match oracle."
#
# STATUS (2026-09-04): SKELETON — Awaiting Phase 2 implementation.
# Linked issues: #4668 (NATURAL JOIN multi-col → Cartesian product — GA-blocker),
#                #4649 (LEFT JOIN USING → Cartesian product — closed via PR #4732),
#                #4636 (correlated scalar subquery — closed via PR #4731),
#                #4656 (> ALL / = ANY empty result — closed via PR #4737).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURES="$REPO_ROOT/tests/join_subquery_semantics"

EXPECTED_ARTIFACTS=(
  "$FIXTURES/manifest.yml"
  "$FIXTURES/join_using.sql"
  "$FIXTURES/natural_join.sql"
  "$FIXTURES/scalar_subquery.sql"
  "$FIXTURES/any_all_subquery.sql"
  "$FIXTURES/having_multi.sql"
  "$FIXTURES/oracle/{join_using,natural_join,scalar_subquery,any_all_subquery,having_multi}.expected"
)

echo "=== RC-B5 Join/subquery correctness (SKELETON) ==="
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
  echo "  - join_using.sql: LEFT/RIGHT/INNER JOIN USING(col) + USING(col1,col2) multi-col"
  echo "  - natural_join.sql: NATURAL JOIN on common-named column(s)"
  echo "  - scalar_subquery.sql: WHERE x = (SELECT ... FROM b WHERE b.id = a.id)"
  echo "  - any_all_subquery.sql: > ALL(...), = ANY(...), IN (SELECT ...) variants"
  echo "  - having_multi.sql: HAVING + multiple AND/OR conditions"
  echo "  - oracle/*.expected: SQLite-generated goldens (per-row SHA256 comparison)"
  echo "  - GA-blocker bypass must issue-link #4668 until fixed"
  exit 1
fi
exit 1
