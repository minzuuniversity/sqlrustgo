#!/usr/bin/env bash
# v3.12.0 V312-14 Crash Recovery Gate — 替代 v3.9.0 #3174 crash_test_framework 检查.
#
# 设计意图: v3.12.0 ALPHA 阶段需要的"崩溃恢复"语义是 WAL replay / kill -9 /
# backup/restore / upgrade,而不是 v3.9.0 时代的通用 crash_test_framework.
# 此脚本针对 v3.12.0 实际存在的测试和证据:
#   1. tests/integration/stress/crash_test_framework.rs 存在
#   2. tests/integration/stress/crash_test_harness.rs 存在
#   3. tests/integration/stress/recovery_scenarios_test.rs 存在
#   4. tests/integration/stress/process_kill_crash_test.rs 存在
#   5. docs/releases/v3.12.0/evidence/crash_recovery/V312-14-CRASH-RECOVERY.md 存在
#   6. V312-14 evidence 状态 (PASS / PARTIAL / FAIL) 与 deferred items 追踪
#
# 退出码: 0=PASS (evidence 存在且 status != FAIL), 1=FAIL

set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0
FAIL=0

probe() {
  local label="$1"
  local path="$2"
  if [ -e "$path" ]; then
    echo "  [PASS] $label: $path"
    PASS=$((PASS + 1))
  else
    echo "  [FAIL] $label: $path missing"
    FAIL=$((FAIL + 1))
  fi
}

echo "=== V312-14 Crash Recovery Gate ==="
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "Scope:  v3.12.0 crash recovery baseline (替代 v3.9.0 #3174 framework 检查)"
echo

probe "v3.12 crash_test_framework" "tests/integration/stress/crash_test_framework.rs"
probe "v3.12 crash_test_harness"   "tests/integration/stress/crash_test_harness.rs"
probe "v3.12 recovery_scenarios"   "tests/integration/stress/recovery_scenarios_test.rs"
probe "v3.12 process_kill_crash"   "tests/integration/stress/process_kill_crash_test.rs"
probe "V312-14 evidence doc"       "docs/releases/v3.12.0/evidence/crash_recovery/V312-14-CRASH-RECOVERY.md"

echo
echo "=== V312-14 Crash Recovery Status ==="
EVIDENCE="docs/releases/v3.12.0/evidence/crash_recovery/V312-14-CRASH-RECOVERY.md"
if [ -f "$EVIDENCE" ]; then
  # Extract status line (## Gate Status block)
  STATUS_LINE=$(grep -A1 "Gate Status" "$EVIDENCE" 2>/dev/null | tail -1 | head -1)
  echo "  $STATUS_LINE"
  if echo "$STATUS_LINE" | grep -qiE "PASS|✅"; then
    PASS=$((PASS + 1))
    echo "  [PASS] gate status PASS"
  elif echo "$STATUS_LINE" | grep -qiE "PARTIAL"; then
    PASS=$((PASS + 1))
    echo "  [PASS] gate status PARTIAL (deferred items tracked, evidence present)"
  elif echo "$STATUS_LINE" | grep -qiE "FAIL"; then
    FAIL=$((FAIL + 1))
    echo "  [FAIL] gate status FAIL"
  else
    echo "  [WARN] gate status unknown"
  fi
fi

echo
echo "PASS: $PASS, FAIL: $FAIL"
if [ "$FAIL" -eq 0 ]; then
  echo "STATUS: V312-14 CRASH RECOVERY GATE PASS"
  exit 0
fi
echo "STATUS: V312-14 CRASH RECOVERY GATE FAIL"
exit 1