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

OUT_DIR="docs/releases/v3.12.0/evidence/sqllogictest"
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

# ---- Exclusion Registry Validation ----
if [ ! -f "$EXCLUSIONS" ]; then
  record_fail "exclusions.yml missing"
elif ! grep -q "^status: active" "$EXCLUSIONS" 2>/dev/null; then
  record_fail "exclusions.yml status is not active (still seed?)"
fi

EXCL_ITEMS=$(grep -c "^  - id:" "$EXCLUSIONS" 2>/dev/null || echo 0)
if [ "$EXCL_ITEMS" -eq 0 ]; then
  record_fail "exclusions.yml has no items"
fi

MISSING_FIELDS=0
while IFS= read -r line; do
  item_id=$(echo "$line" | sed 's/^  - id: //')
  item_block=$(grep -A 10 "^  - id: ${item_id}" "$EXCLUSIONS" 2>/dev/null || echo "")
  for field in root_cause owner expiry follow_up_issue_or_openspec; do
    if ! echo "$item_block" | grep -q "^[ ]*${field}:"; then
      MISSING_FIELDS=$((MISSING_FIELDS + 1))
    fi
  done
done < <(grep "^  - id:" "$EXCLUSIONS" 2>/dev/null)

if [ "$MISSING_FIELDS" -gt 0 ]; then
  record_fail "exclusions.yml: $MISSING_FIELDS missing field assignments"
fi

# ---- Manifest Validation ----
if [ -f "$MANIFEST" ]; then
  MANIFEST_TOTAL=$(grep '"total_files"' "$MANIFEST" | grep -o '[0-9]\+' | head -1 || echo 0)
  MANIFEST_PASS=$(grep '"pass_files"' "$MANIFEST" | grep -o '[0-9]\+' | head -1 || echo 0)
  ACTUAL_TEST_FILES=$(find crates/sqlrustgo_sqllogictest/testdata -name "*.test" 2>/dev/null | wc -l)
  if [ "$MANIFEST_TOTAL" != "$ACTUAL_TEST_FILES" ]; then
    record_fail "manifest total_files ($MANIFEST_TOTAL) != actual test files ($ACTUAL_TEST_FILES)"
  fi
  if [ "$MANIFEST_TOTAL" -gt 0 ] && [ "$EXCL_ITEMS" -gt 0 ]; then
    EXPECTED_EXCL=$((MANIFEST_TOTAL - MANIFEST_PASS))
    if [ "$EXPECTED_EXCL" != "$EXCL_ITEMS" ]; then
      record_fail "manifest pass_files ($MANIFEST_PASS) + exclusions ($EXCL_ITEMS) != total ($MANIFEST_TOTAL)"
    fi
  fi
fi

# ---- OpenSpec Follow-up Validation ----
MISSING_OPENSPC=0
while IFS= read -r line; do
  item_id=$(echo "$line" | sed 's/^  - id: //')
  follow_up_block=$(grep -A 8 "^  - id: ${item_id}" "$EXCLUSIONS" 2>/dev/null || echo "")
  op_path=$(echo "$follow_up_block" | grep "follow_up_issue_or_openspec:" | sed -n 's/.*openspec\/changes\/\([^ ]*\).*/\1/p' | tr -d ' ,')
  if [ -n "$op_path" ]; then
    op_dir=$(echo "$op_path" | sed 's/\/.*//')
    if [ -n "$op_dir" ] && [ ! -d "openspec/changes/${op_dir}" ] 2>/dev/null; then
      MISSING_OPENSPC=$((MISSING_OPENSPC + 1))
    fi
  fi
done < <(grep "^  - id:" "$EXCLUSIONS" 2>/dev/null)

if [ "$MISSING_OPENSPC" -gt 0 ]; then
  record_fail "exclusions.yml: $MISSING_OPENSPC OpenSpec follow-up directories missing"
fi
cat >"$REPORT" <<EOF
# SQLRustGo v3.12 SQLLogicTest Smoke Baseline

| Field | Value |
|---|---|
| source_agent | minimax-m2.7 |
| source_run | check_sqllogictest_v312 |
| timestamp | $(date -Iseconds) |
| commit | $(git rev-parse HEAD 2>/dev/null || echo unknown) |
| log | $LOG |
| evidence_hash | $(sha256sum "$LOG" 2>/dev/null | cut -d' ' -f1 || echo unavailable) |
| gate_status | $([ "$FAIL" -eq 0 ] && echo PASS || echo FAIL) |

## Runner Summary

\`\`\`text
${PASS_FAIL_LINE:-files: unavailable}
${PASS_RATE_LINE:-pass rate: unavailable}
\`\`\`

## Exclusion Registry

Exclusions are managed in: \`$EXCLUSIONS\`
Manifest is at: \`$MANIFEST\`

## Boundary

This report is a v3.12 smoke baseline. It does not claim the SQLite official corpus is integrated or that selected targets pass.
EOF


echo | tee -a "$LOG"
echo "report: $REPORT" | tee -a "$LOG"
echo "manifest: $MANIFEST" | tee -a "$LOG"
echo "exclusions: $EXCLUSIONS" | tee -a "$LOG"
echo "summary: $PASS PASS, $FAIL FAIL" | tee -a "$LOG"

if [ "$FAIL" -eq 0 ]; then
  exit 0
fi
exit 1
