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
# Accept two formats:
#   Format A (HEAD):    status: active  / items:  / - id:
#   Format B (origin): exclusions:     / (top-level list) / - file:
if [ ! -f "$EXCLUSIONS" ]; then
  record_fail "exclusions.yml missing"
else
  HAS_STATUS_ACTIVE=$(grep -c "^status: active" "$EXCLUSIONS" 2>/dev/null || echo 0)
  HAS_EXCLUSIONS=$(grep -c "^exclusions:" "$EXCLUSIONS" 2>/dev/null || echo 0)
  HAS_ITEMS=$(grep -c "^items:" "$EXCLUSIONS" 2>/dev/null || echo 0)
  HAS_FILE_ITEMS=$(grep -c "^  - file:" "$EXCLUSIONS" 2>/dev/null || echo 0)

  if [ "$HAS_STATUS_ACTIVE" -eq 0 ] && [ "$HAS_EXCLUSIONS" -eq 0 ]; then
    record_fail "exclusions.yml: missing 'status: active' or 'exclusions:' (still seed?)"
  fi

  # Count items
  if [ "$HAS_ITEMS" -gt 0 ]; then
    EXCL_ITEMS=$(grep -c "^  - id:" "$EXCLUSIONS" 2>/dev/null || echo 0)
  elif [ "$HAS_FILE_ITEMS" -gt 0 ]; then
    EXCL_ITEMS=$(grep -c "^  - file:" "$EXCLUSIONS" 2>/dev/null || echo 0)
  else
    EXCL_ITEMS=0
  fi

  if [ "$EXCL_ITEMS" -eq 0 ]; then
    record_fail "exclusions.yml: no exclusion items found"
  fi

  # Validate required fields per item
  MISSING_FIELDS=0
  if [ "$HAS_ITEMS" -gt 0 ]; then
    # Format A: - id: ... root_cause/owner/expiry/follow_up_issue_or_openspec
    while IFS= read -r line; do
      item_id=$(echo "$line" | sed 's/^  - id: //')
      item_block=$(grep -A 10 "^  - id: ${item_id}" "$EXCLUSIONS" 2>/dev/null || echo "")
      for field in root_cause owner expiry follow_up_issue_or_openspec; do
        if ! echo "$item_block" | grep -q "^[ ]*${field}:"; then
          MISSING_FIELDS=$((MISSING_FIELDS + 1))
        fi
      done
    done < <(grep "^  - id:" "$EXCLUSIONS" 2>/dev/null)
  elif [ "$HAS_FILE_ITEMS" -gt 0 ]; then
    # Format B: - file: ... category/owner/expiry/follow_up
    while IFS= read -r line; do
      item_file=$(echo "$line" | sed 's/^  - file: //')
      item_block=$(grep -A 10 "^  - file: ${item_file}" "$EXCLUSIONS" 2>/dev/null || echo "")
      for field in category owner expiry follow_up; do
        if ! echo "$item_block" | grep -q "^[ ]*${field}:"; then
          MISSING_FIELDS=$((MISSING_FIELDS + 1))
        fi
      done
    done < <(grep "^  - file:" "$EXCLUSIONS" 2>/dev/null)
  fi

  if [ "$MISSING_FIELDS" -gt 0 ]; then
    record_fail "exclusions.yml: $MISSING_FIELDS missing field assignments"
  fi
fi

# ---- Manifest Validation ----
# `wc -l` on macOS emits leading whitespace (BSD wc); trim to digits-only so
# the string compare against MANIFEST_TOTAL is reliable across platforms.
if [ -f "$MANIFEST" ]; then
  MANIFEST_TOTAL=$(grep '"total_files"' "$MANIFEST" | grep -o '[0-9]\+' | head -1 || echo 0)
  MANIFEST_PASS=$(grep '"pass_files"' "$MANIFEST" | grep -o '[0-9]\+' | head -1 || echo 0)
  ACTUAL_TEST_FILES=$(find crates/sqlrustgo_sqllogictest/testdata -name "*.test" 2>/dev/null | wc -l | tr -d '[:space:]')
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

# Compute evidence_hash AFTER all log writes are done. We append more lines
# to $LOG below (the `tee -a "$LOG"` blocks) so the hash must be deferred
# until those writes finish.
LOG_HASH=$(sha256sum "$LOG" 2>/dev/null | cut -d' ' -f1 || echo unavailable)

cat >"$REPORT" <<EOF
# SQLRustGo v3.12 SQLLogicTest Smoke Baseline

> **provenance:** generated_by=check_sqllogictest_v312.sh, generated_at=$(date -Iseconds), commit=$(git rev-parse HEAD 2>/dev/null || echo unknown), source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, gate_policy_eval_id=v312-slt-smoke-001, evidence_hash=$LOG_HASH, log_path=$LOG

| Field | Value |
|---|---|
| source_agent | minimax-m2.7 |
| source_run | check_sqllogictest_v312 |
| timestamp | $(date -Iseconds) |
| commit | $(git rev-parse HEAD 2>/dev/null || echo unknown) |
| log | $LOG |
| evidence_hash | $LOG_HASH |
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


echo
SUMMARY_FILE="${LOG}.summary"
{
  echo
  echo "report: $REPORT"
  echo "manifest: $MANIFEST"
  echo "exclusions: $EXCLUSIONS"
  echo "summary: $PASS PASS, $FAIL FAIL"
} | tee "$SUMMARY_FILE"

if [ "$FAIL" -eq 0 ]; then
  exit 0
fi
exit 1
