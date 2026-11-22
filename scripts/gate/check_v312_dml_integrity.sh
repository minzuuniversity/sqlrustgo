#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：保留并改接（keep-rewire）
# 责任任务：P0-4
# 登记说明：判据内容仍在 v4.1.0 的声称面内(台账 open issue #5409/#5462/#5463/#5527)：INSERT 列名校验/GROUP BY 语义/重复键静默追加仍是 v4.1.0 的 open P0 ⇒ 不能归档，改名重挂执行点
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# RC-B6 - V312-RC-GA: DML/integrity correctness gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B6 DML/integrity correctness | bash scripts/gate/check_v312_dml_integrity.sh
#    | CHECK, AUTO_INCREMENT/AUTOINCREMENT, RETURNING,
#      UPDATE without WHERE are correct or scoped out."
#
# STATUS (2026-09-04): SKELETON — Awaiting Phase 2 implementation.
# Linked issues: #4709 (CHECK multi-condition — closed PR #4737),
#                #4672 (AUTOINCREMENT — closed),
#                #4696 (UPDATE w/o WHERE — closed),
#                #4626 (SELECT FOR UPDATE + ROLLBACK abort — still open, GA-blocker).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURES="$REPO_ROOT/tests/dml_integrity"

EXPECTED_ARTIFACTS=(
  "$FIXTURES/manifest.yml"
  "$FIXTURES/check_constraint.sql"
  "$FIXTURES/auto_increment.sql"
  "$FIXTURES/returning.sql"
  "$FIXTURES/update_without_where.sql"
  "$FIXTURES/transaction_for_update_rollback.sql"
  "$FIXTURES/oracle/transaction_for_update_rollback.expected"
)

echo "=== RC-B6 DML/integrity correctness (SKELETON) ==="
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
  echo "  - check_constraint.sql: INSERT VALUES (200) ON CHECK (col < 100) → reject"
  echo "  - auto_increment.sql: INSERT INTO autoinc(id) VALUES(NULL) → id=N+1"
  echo "  - returning.sql: INSERT ... RETURNING col1, col2 → result rows"
  echo "  - update_without_where.sql: UPDATE t SET x=y → updates ALL rows (no parse error)"
  echo "  - transaction_for_update_rollback.sql: SELECT FOR UPDATE then ROLLBACK"
  echo "    MUST release lock + revert state — fail closed if abort (issue #4626)"
  exit 1
fi
exit 1
