#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_executable() {
  local path="$1"
  test -x "$path" || fail "$path must exist and be executable"
}

assert_executable scripts/gate/check_alpha_entry_v3.12.0.sh
assert_executable scripts/gate/check_alpha_quality_v3.12.0.sh
assert_executable scripts/gate/check_v312_deferred_followups.sh

deferred_log="$(mktemp)"
if ! bash scripts/gate/check_v312_deferred_followups.sh >"$deferred_log" 2>&1; then
  cat "$deferred_log"
  fail "deferred follow-up check should pass when every exclusion is bound to a Gitea issue"
fi
grep -q "Gitea issue references" "$deferred_log" || {
  cat "$deferred_log"
  fail "deferred follow-up output must disclose Gitea issue binding"
}
grep -q "OpenSpec-only references: 0" "$deferred_log" || {
  cat "$deferred_log"
  fail "deferred follow-up output must reject OpenSpec-only closure bindings"
}

quality_log="$(mktemp)"
if ALPHA_QUALITY_FAST_TEST=1 bash scripts/gate/check_alpha_quality_v3.12.0.sh >"$quality_log" 2>&1; then
  cat "$quality_log"
  fail "alpha quality gate must fail while current hard governance gates are failing"
fi
grep -q "STATUS: ALPHA QUALITY BLOCKED" "$quality_log" || {
  cat "$quality_log"
  fail "alpha quality gate must use an explicit BLOCKED status"
}
grep -q "Q1_SQLLOGICTEST_GATE" "$quality_log" || {
  cat "$quality_log"
  fail "alpha quality gate must include SQLLogicTest hard gate"
}
grep -q "Q2_P12_IGNORE_COUNT" "$quality_log" || {
  cat "$quality_log"
  fail "alpha quality gate must include P12 ignore registry gate"
}
grep -q "Q3_P16_GATE_TEST_INTEGRITY" "$quality_log" || {
  cat "$quality_log"
  fail "alpha quality gate must include P16 gate test integrity"
}

rm -f "$deferred_log" "$quality_log"
echo "PASS: v312 alpha gate governance regression tests"
