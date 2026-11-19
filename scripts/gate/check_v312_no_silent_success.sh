#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：保留并改接（keep-rewire）
# 责任任务：P0-4
# 登记说明：判据内容仍在 v4.1.0 的声称面内(台账 open issue #5490/#5392)：『报告成功但实际少行/假成功』正是 v4.1.0 反复出现的失效形状 ⇒ 必须改名重挂，这是反虚假执行的判据
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# RC-B3 - V312-RC-GA: Semantic no-op guard gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B3 Semantic no-op guard | bash scripts/gate/check_v312_no_silent_success.sh
#    | Accepted DDL/DML has observable postcondition;
#      unsupported SQL returns explicit error."
#
# STATUS (2026-09-04): SKELETON — Awaiting Phase 2 implementation.
# Linked issues: #4652 (CREATE PROCEDURE/FUNCTION silently accepts without storage),
#                #4709 (CHECK constraint silently accepts invalid rows — closed PR #4737),
#                #4701 (expression index accepts but errors at execution).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURES="$REPO_ROOT/tests/no_silent_success"

EXPECTED_ARTIFACTS=(
  "$FIXTURES/manifest.yml"
  "$FIXTURES/ddl_noop_probes.sql"
  "$FIXTURES/dml_noop_probes.sql"
  "$FIXTURES/expected/accepted_must_have_postcondition.md"
  "$FIXTURES/expected/rejected_must_return_error.md"
)

echo "=== RC-B3 Semantic no-op guard (SKELETON) ==="
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
  echo "  - DDL_NOOP: CREATE PROCEDURE body without storage → must return error"
  echo "  - DML_NOOP: CHECK constraint that silently accepts out-of-range rows"
  echo "  - For each probe: assert that 'accepted SQL' produces observable side-effect"
  echo "                    OR returns 'unsupported' error (never silent no-op)"
  exit 1
fi
exit 1
