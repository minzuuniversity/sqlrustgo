#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：需外部主机（needs-external-host）
# 责任任务：P0-4
# 登记说明：判据内容仍在 v4.1.0 的声称面内(台账 open issue #5226)：B-track oracle 语料是 SQL 语义差分的对照源 ⇒ 改名重挂
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# RC-B1 - V312-RC-GA: BustubX-EDU B-track oracle corpus gate
# Source: docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md §5
#   "RC-B1 B-track oracle corpus | bash scripts/gate/check_bustubx_b_track_v312.sh
#    | Core B-track seed + exercises pass against SQLite oracle or issue-linked exclusion."
#
# Phase 2 (2026-11-14, #5114): the runner below executes every manifest case
# against SQLite and SQLRustGo and compares exit + normalized rows + SHA256.
# It also checks the committed oracle against a live SQLite run (a stale
# oracle is FAIL, not PASS). Missing engine/sqlite => exit 2 INCOMPLETE.
# The previous SKELETON only checked artifact presence and always exited 1.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$REPO_ROOT/tests/compat/bustubx_edu_b_track/manifest.yml"
BTRACK_ROOT="$REPO_ROOT/tests/compat/bustubx_edu_b_track"
EXPECTED_ARTIFACTS=(
  "$MANIFEST"
  "$BTRACK_ROOT/seed.sql"
  "$BTRACK_ROOT/exercises/week01_setup.sql"
  "$BTRACK_ROOT/oracle/week01_setup.expected"
)

echo "=== RC-B1 BustubX-EDU B-track oracle corpus (SKELETON) ==="
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
  echo "  - Author B-track seed.sql from bustubx_edu week01-week06 curriculum"
  echo "  - Generate SQLite oracle via 'sqlite3 < seed.sql' for each exercise"
  echo "  - Populate oracle/week{N}-{name}.expected from SQLite output"
  echo "  - Re-run this gate; manifest pass-rate must reach 6/6 week coverage"
  exit 1
fi

echo
echo "RESULT: artifacts present — running Phase 2 differential runner."
RUNNER="$REPO_ROOT/scripts/gate/run_bustubx_b_track.py"
if [ ! -f "$RUNNER" ]; then
  echo "FAIL: runner missing: $RUNNER" >&2
  exit 2
fi
python3 "$RUNNER" "$@"
rc=$?
case "$rc" in
  0) echo "RESULT: PASS (all B-track cases match SQLite oracle)";;
  2) echo "RESULT: INCOMPLETE (engine/sqlite/oracle unavailable — not PASS)";;
  *) echo "RESULT: FAIL (see per-case output above)";;
esac
exit "$rc"
