#!/usr/bin/env bash
# SQLRustGo v3.12 SQLLogicTest gate entry.
#
# This gate establishes an executable baseline for V312-11. It does not claim
# the full SQLite official corpus is integrated or passing.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

OUT_DIR="docs/releases/v3.12.0/sqllogictest-baseline"
LOG_DIR="docs/releases/v3.12.0/logs"
mkdir -p "$OUT_DIR" "$LOG_DIR"

TS="$(date +%Y%m%d_%H%M%S)"
COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
LOG="$LOG_DIR/sqllogictest_${COMMIT}_${TS}.log"
REPORT="$OUT_DIR/smoke-report.md"
MANIFEST="$OUT_DIR/sqlite-corpus-manifest.json"
EXCLUSIONS="$OUT_DIR/exclusions.yml"

PASS=0
FAIL=0

record_pass() {
  local msg="[PASS] $1"
  echo "$msg"
  echo "$msg" >>"$LOG"
  PASS=$((PASS + 1))
}

record_fail() {
  local msg="[FAIL] $1"
  echo "$msg"
  echo "$msg" >>"$LOG"
  FAIL=$((FAIL + 1))
}

echo "=== SQLRustGo v3.12 SQLLogicTest Gate Entry ===" | tee "$LOG"
echo "timestamp: $(date -Iseconds)" | tee -a "$LOG"
echo "commit: $(git rev-parse HEAD 2>/dev/null || echo unknown)" | tee -a "$LOG"
echo | tee -a "$LOG"

if cargo build -p sqlrustgo_sqllogictest >>"$LOG" 2>&1; then
  record_pass "cargo build -p sqlrustgo_sqllogictest"
else
  record_fail "cargo build -p sqlrustgo_sqllogictest"
fi

if cargo run -p sqlrustgo_sqllogictest -- --help >>"$LOG" 2>&1; then
  record_pass "runner --help"
else
  record_fail "runner --help"
fi

if test -d crates/sqlrustgo_sqllogictest/testdata; then
  record_pass "local smoke testdata exists"
else
  record_fail "local smoke testdata exists"
fi

# The current runner returns 0 even with expected baseline failures. Keep this
# as a baseline collector and parse the output instead of pretending all tests pass.
cargo run -p sqlrustgo_sqllogictest -- \
  --test-dir crates/sqlrustgo_sqllogictest/testdata \
  --max-fail 20 >>"$LOG" 2>&1
RUN_STATUS=$?
if [ "$RUN_STATUS" -eq 0 ]; then
  record_pass "runner smoke execution completed"
else
  record_fail "runner smoke execution completed"
fi

SUMMARY="$(grep -A2 '^=== Summary ===' "$LOG" | tail -2 || true)"
PASS_FAIL_LINE="$(printf '%s\n' "$SUMMARY" | grep '^files:' || true)"
PASS_RATE_LINE="$(printf '%s\n' "$SUMMARY" | grep '^pass rate:' || true)"

cat >"$REPORT" <<EOF
# SQLRustGo v3.12 SQLLogicTest Smoke Baseline

| Field | Value |
|---|---|
| source_agent | Codex |
| source_run | check_sqllogictest_v312 |
| timestamp | $(date -Iseconds) |
| commit | $(git rev-parse HEAD 2>/dev/null || echo unknown) |
| log | $LOG |
| gate_status | $([ "$FAIL" -eq 0 ] && echo PASS || echo FAIL) |

## Runner Summary

\`\`\`text
${PASS_FAIL_LINE:-files: unavailable}
${PASS_RATE_LINE:-pass rate: unavailable}
\`\`\`

## Boundary

This report is a v3.12 smoke baseline. It does not claim the SQLite official corpus is integrated or that selected targets pass.
EOF

if [ ! -f "$MANIFEST" ]; then
  cat >"$MANIFEST" <<EOF
{
  "status": "not_integrated",
  "note": "SQLite official SQLLogicTest corpus is not yet integrated. V312-11 must replace this with a real manifest containing source, hash, file_count, selected_targets, and exclusions."
}
EOF
fi

if [ ! -f "$EXCLUSIONS" ]; then
  cat >"$EXCLUSIONS" <<EOF
# v3.12 SQLLogicTest exclusion registry
# V312-11 must replace this seed with issue-linked exclusions.
status: seed
items: []
EOF
fi

echo | tee -a "$LOG"
echo "report: $REPORT" | tee -a "$LOG"
echo "manifest: $MANIFEST" | tee -a "$LOG"
echo "exclusions: $EXCLUSIONS" | tee -a "$LOG"
echo "summary: $PASS PASS, $FAIL FAIL" | tee -a "$LOG"

if [ "$FAIL" -eq 0 ]; then
  exit 0
fi
exit 1
