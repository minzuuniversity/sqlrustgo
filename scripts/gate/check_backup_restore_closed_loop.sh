#!/usr/bin/env bash
#
# scripts/gate/check_backup_restore_closed_loop.sh
#
# The reviewer-facing entry point of the #5525 closed loop.
#
# WHY THIS FILE EXISTS
# --------------------
# The Owner directive of 2026-10-25 (hub issue #5, comment 198163) says, for
# pull request #5587:
#
#   "#5587（备份/恢复真实化，**必须独立 Reviewer 实跑「备份→恢复→验证数据」
#    闭环**，仅「不报错」不算过）"
#
# and, for the P0 red line "#5525 备份假成功":
#
#   "验收=真实恢复/真实并发场景证据，禁止「仅不报错」"
#
# "The call returned Ok" is therefore not an acceptable result.  This
# script runs the closed loop and reports a **verdict** that is only PASS
# when the run produced the evidence line, and it is INCONCLUSIVE — never
# PASS — when the evidence line is missing, because a measurement that
# could not see its own result has measured nothing.
#
# Usage:
#   bash scripts/gate/check_backup_restore_closed_loop.sh [--json <out>]
#
# Exit codes:
#   0  PASS      — the closed loop ran and the evidence line is present
#   1  FAIL      — the closed loop ran and a check failed
#   2  INCONCLUSIVE — the closed loop could not produce a verdict
#
set -uo pipefail

TEST_TARGET="backup_restore_closed_loop_5525"
JSON_OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --json) JSON_OUT="${2:-}"; shift 2 ;;
    -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
    *) echo "usage: $0 [--json <out>]" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT" || { echo "FATAL: cannot enter the repository root" >&2; exit 2; }

COMMIT="$(git rev-parse HEAD 2>/dev/null || echo unknown)"
LOG="$(mktemp "${TMPDIR:-/tmp}/backup_closed_loop.XXXXXX")"
trap 'rm -f "$LOG"' EXIT

echo "=== #5525 backup → restore → verify-the-data closed loop ==="
echo "commit: ${COMMIT}"
echo "target: ${TEST_TARGET}"
echo ""

# --nocapture, because the evidence line is printed by the test itself and
# a captured stdout would hide exactly the thing this script has to look at.
cargo test -p sqlrustgo-tools --test "${TEST_TARGET}" -- --nocapture >"$LOG" 2>&1
TEST_RC=$?

sed -n '1,200p' "$LOG"

# The evidence line: backup id, table count, row count and the SHA-256 of
# the payload the backup actually wrote.
EVIDENCE="$(grep -E '^EVIDENCE closed_loop ' "$LOG" | head -1)"
BACKUP_ID="$(sed -n 's/.*backup_id=\([^ ]*\).*/\1/p' <<<"$EVIDENCE")"
TABLES="$(sed -n 's/.*tables=\([^ ]*\).*/\1/p' <<<"$EVIDENCE")"
ROWS="$(sed -n 's/.*rows=\([^ ]*\).*/\1/p' <<<"$EVIDENCE")"
PAYLOAD_SHA="$(sed -n 's/.*payload_sha256=\([^ ]*\).*/\1/p' <<<"$EVIDENCE")"

VERDICT="INCONCLUSIVE"
REASON="the closed loop produced no evidence line"
if [ "$TEST_RC" -ne 0 ]; then
  VERDICT="FAIL"
  REASON="the closed loop returned a non-zero exit code (${TEST_RC})"
elif [ -n "$EVIDENCE" ] && [ -n "$BACKUP_ID" ] && [ -n "$ROWS" ] && [ "${ROWS}" -gt 0 ] 2>/dev/null; then
  VERDICT="PASS"
  REASON="the backup, the artifacts, the deep comparison and the landing in a real storage engine all held"
fi

echo ""
echo "VERDICT: ${VERDICT}"
echo "REASON:  ${REASON}"
[ -n "${BACKUP_ID}" ] && echo "backup_id: ${BACKUP_ID}"
[ -n "${TABLES}" ] && echo "tables: ${TABLES}"
[ -n "${ROWS}" ] && echo "rows: ${ROWS}"
[ -n "${PAYLOAD_SHA}" ] && echo "payload_sha256: ${PAYLOAD_SHA}"

if [ -n "$JSON_OUT" ]; then
  cat >"$JSON_OUT" <<JSON
{
  "verdict": "${VERDICT}",
  "reason": "${REASON}",
  "commit": "${COMMIT}",
  "backup_id": "${BACKUP_ID}",
  "tables": "${TABLES}",
  "rows": "${ROWS}",
  "payload_sha256": "${PAYLOAD_SHA}",
  "test_exit_code": ${TEST_RC}
}
JSON
  echo "wrote ${JSON_OUT}"
fi

case "$VERDICT" in
  PASS) exit 0 ;;
  FAIL) exit 1 ;;
  *) exit 2 ;;
esac
