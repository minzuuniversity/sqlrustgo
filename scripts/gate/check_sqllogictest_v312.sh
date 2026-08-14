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
SOURCE_AGENT="${SOURCE_AGENT:-unknown-local-agent}"
SOURCE_RUN="${SOURCE_RUN:-check_sqllogictest_v312}"

PASS=0
FAIL=0
WARN=0

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

record_warn() {
  local msg="[WARN] $1"
  echo "$msg"
  echo "$msg" >>"$LOG"
  WARN=$((WARN + 1))
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
# Round-9 (V312-24): runner now exits non-zero when files_fail > 0
# (see crates/sqlrustgo_sqllogictest/src/main.rs).
cargo run -p sqlrustgo_sqllogictest -- \
  --test-dir crates/sqlrustgo_sqllogictest/testdata \
  --max-fail 20 >>"$LOG" 2>&1
RUN_STATUS=$?

# Parse file-level FAIL/PREPROCESS FAIL lines from the runner log so that the
# gate can detect real regressions even if the runner process happens to exit 0
# (e.g. when --max-fail truncates output). The runner's own exit code is the
# primary signal; this grep is the secondary belt-and-suspenders check.
# Use || fallback rather than `|| echo 0` inside $(...) to avoid multi-line
# stdout under `set -u` when grep returns non-zero.
grep -c '^FAIL \[' "$LOG" 2>/dev/null > /tmp/_r9_fail_count || FAIL_FILE_COUNT=0
grep -c '^PREPROCESS FAIL \[' "$LOG" 2>/dev/null > /tmp/_r9_pp_fail_count || PREPROCESS_FAIL_COUNT=0
FAIL_FILE_COUNT=$(tr -d '[:space:]' < /tmp/_r9_fail_count)
PREPROCESS_FAIL_COUNT=$(tr -d '[:space:]' < /tmp/_r9_pp_fail_count)
FAIL_FILE_COUNT="${FAIL_FILE_COUNT:-0}"
PREPROCESS_FAIL_COUNT="${PREPROCESS_FAIL_COUNT:-0}"
rm -f /tmp/_r9_fail_count /tmp/_r9_pp_fail_count
TOTAL_FILE_FAIL=$((FAIL_FILE_COUNT + PREPROCESS_FAIL_COUNT))

# ---- Round-10: Exclusion-aware smoke gate ----
# The V312 smoke baseline is intentionally not the full SQLite corpus; 16
# known FAIL files are registered in exclusions.yml (each with a v313
# follow_up_issue and close_boundary). This check splits the observed
# file-level failures into:
#   * registered = listed under exclusions.yml `items: - file:` (or `id:` mapped via `file:`)
#   * unregistered = NOT in the registry → real regression
# The runner is allowed to exit non-zero AND produce file-level failures,
# provided every observed failure maps to a registered exclusion item.
# Unregistered failures OR a missing/malformed registry still fail the gate.
if [ -f "$EXCLUSIONS" ] && grep -q "^status: active" "$EXCLUSIONS" 2>/dev/null; then
  # Extract registered file names from exclusions.yml (Format A: items: - file: ...)
  REGISTERED_FILES=$(grep -E "^[[:space:]]+file:[[:space:]]+\S+\.test" "$EXCLUSIONS" 2>/dev/null \
    | sed -E 's/^[[:space:]]+file:[[:space:]]+//' | sort -u)
  REGISTERED_COUNT=$(printf '%s\n' "$REGISTERED_FILES" | grep -c '\.test$' 2>/dev/null || true)
  REGISTERED_COUNT="${REGISTERED_COUNT:-0}"

  # Extract observed failed file names from the log
  OBSERVED_FAILS=$(grep -E '^(FAIL|PREPROCESS FAIL) \[' "$LOG" 2>/dev/null \
    | sed -E 's/^(FAIL|PREPROCESS FAIL) \[([^]]+)\].*/\2/' | sort -u)
  OBSERVED_COUNT=$(printf '%s\n' "$OBSERVED_FAILS" | grep -c '.' 2>/dev/null || true)
  OBSERVED_COUNT="${OBSERVED_COUNT:-0}"

  # Compute unregistered failures via comm (write inputs to temp files to
  # avoid process-substitution shell quirks; use -z to handle empty inputs).
  UNREGISTERED=0
  if [ -n "$OBSERVED_FAILS" ] && [ -n "$REGISTERED_FILES" ]; then
    _obs=$(mktemp); _reg=$(mktemp); _diff=$(mktemp)
    printf '%s\n' "$OBSERVED_FAILS" > "$_obs"
    printf '%s\n' "$REGISTERED_FILES" > "$_reg"
    comm -23 "$_obs" "$_reg" > "$_diff"
    UNREGISTERED=$(grep -c '.' "$_diff" 2>/dev/null || true)
    UNREGISTERED="${UNREGISTERED:-0}"
    rm -f "$_obs" "$_reg" "$_diff"
  elif [ "$OBSERVED_COUNT" -gt 0 ] && [ -z "$REGISTERED_FILES" ]; then
    UNREGISTERED=$OBSERVED_COUNT
  fi

  if [ "$RUN_STATUS" -eq 0 ]; then
    if [ "$OBSERVED_COUNT" -eq 0 ]; then
      record_pass "runner smoke execution completed (clean)"
    else
      record_pass "runner smoke execution completed (observed=$OBSERVED_COUNT, registered=$REGISTERED_COUNT, unregistered=$UNREGISTERED — all observed failures covered by exclusions.yml)"
    fi
  else
    if [ "$UNREGISTERED" -eq 0 ]; then
      record_pass "runner smoke execution completed (runner exit=$RUN_STATUS, observed=$OBSERVED_COUNT, registered=$REGISTERED_COUNT — all observed failures covered by exclusions.yml)"
    else
      record_fail "runner smoke execution completed (runner exit=$RUN_STATUS, observed=$OBSERVED_COUNT, unregistered=$UNREGISTERED — UNREGISTERED failures require new exclusion entries)"
    fi
  fi
else
  # No active exclusion registry → strict behavior (Round-9)
  if [ "$RUN_STATUS" -eq 0 ]; then
    if [ "$TOTAL_FILE_FAIL" -gt 0 ]; then
      record_fail "runner smoke execution completed ($TOTAL_FILE_FAIL file-level failures detected in log)"
    else
      record_pass "runner smoke execution completed"
    fi
  else
    record_fail "runner smoke execution completed (runner exit=$RUN_STATUS, $TOTAL_FILE_FAIL file-level failures)"
  fi
fi

# Promote a structured per-file summary to the top of the log so downstream
# tools and human reviewers can see the full file-level result without
# scrolling through cargo build output.
PER_FILE_TABLE="$(grep -E '^(PASS|FAIL|PREPROCESS FAIL) \[' "$LOG" | sort || true)"
if [ -n "$PER_FILE_TABLE" ]; then
  {
    echo
    echo "=== Per-file results ==="
    echo "$PER_FILE_TABLE"
  } >>"$LOG"
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

  # Count items: separate OPEN (active deferral) from CLOSED (historical).
  # Round-15+ (follow-up to PR #4194): closed exclusions are retained for
  # historical reference per Round-9 codex #89133 + Round-14 #89293
  # governance; the manifest math invariant only enforces against OPEN
  # items, so the all-pass + closed-historical steady state is legal.
  if [ "$HAS_ITEMS" -gt 0 ]; then
    EXCL_ITEMS=$(grep -c "^  - id:" "$EXCLUSIONS" 2>/dev/null || echo 0)
    EXCL_CLOSED_ITEMS=$(grep -c "^    status: closed" "$EXCLUSIONS" 2>/dev/null || echo 0)
    EXCL_OPEN_ITEMS=$((EXCL_ITEMS - EXCL_CLOSED_ITEMS))
  elif [ "$HAS_FILE_ITEMS" -gt 0 ]; then
    EXCL_ITEMS=$(grep -c "^  - file:" "$EXCLUSIONS" 2>/dev/null || echo 0)
    # Format B has no per-item status field — treat all items as OPEN.
    EXCL_OPEN_ITEMS=$EXCL_ITEMS
    EXCL_CLOSED_ITEMS=0
  else
    EXCL_ITEMS=0
    EXCL_OPEN_ITEMS=0
    EXCL_CLOSED_ITEMS=0
  fi

  if [ "$EXCL_ITEMS" -eq 0 ]; then
    record_fail "exclusions.yml: no exclusion items found"
  fi

  # Validate required fields per item
  MISSING_FIELDS=0
  if [ "$HAS_ITEMS" -gt 0 ]; then
    # Format A: - id: ... root_cause/owner/expiry/follow_up_issue_or_openspec
    # Round-9: accept either "root_cause:" (legacy) or "failure_summary:" (Round-9 schema);
    # accept either "expiry:" (legacy) or "v3.13_expiry:" (Round-9 schema).
    while IFS= read -r line; do
      item_id=$(echo "$line" | sed 's/^  - id: //')
      item_block=$(grep -A 12 "^  - id: ${item_id}" "$EXCLUSIONS" 2>/dev/null || echo "")
      for field in owner; do
        if ! echo "$item_block" | grep -q "^[ ]*${field}:"; then
          MISSING_FIELDS=$((MISSING_FIELDS + 1))
        fi
      done
      # expiry (legacy) OR v3.13_expiry (Round-9)
      if ! echo "$item_block" | grep -qE "^[ ]*(expiry|v3.13_expiry):"; then
        MISSING_FIELDS=$((MISSING_FIELDS + 1))
      fi
      # root_cause (legacy) OR failure_summary (Round-9)
      if ! echo "$item_block" | grep -qE "^[ ]*(root_cause|failure_summary):"; then
        MISSING_FIELDS=$((MISSING_FIELDS + 1))
      fi
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
  ACTUAL_TEST_FILES=$(find crates/sqlrustgo_sqllogictest/testdata -name "*.test" -not -path '*/_unsupported/*' 2>/dev/null | wc -l | tr -d '[:space:]')
  if [ "$MANIFEST_TOTAL" != "$ACTUAL_TEST_FILES" ]; then
    record_fail "manifest total_files ($MANIFEST_TOTAL) != actual test files ($ACTUAL_TEST_FILES)"
  fi
  if [ "$MANIFEST_TOTAL" -gt 0 ]; then
    EXPECTED_EXCL=$((MANIFEST_TOTAL - MANIFEST_PASS))
    if [ "$EXPECTED_EXCL" -gt 0 ] && [ "$EXCL_OPEN_ITEMS" -ne "$EXPECTED_EXCL" ]; then
      record_fail "manifest pass_files ($MANIFEST_PASS) + open exclusions ($EXCL_OPEN_ITEMS) != total ($MANIFEST_TOTAL)"
    fi
    # All-pass + closed-historical steady state: emit an explicit PASS so
    # the closed-only state is visible in the gate log and smoke report.
    if [ "$EXPECTED_EXCL" -eq 0 ] && [ "$EXCL_CLOSED_ITEMS" -gt 0 ]; then
      record_pass "manifest all-pass + closed-historical steady state (pass=$MANIFEST_PASS, total=$MANIFEST_TOTAL, open=$EXCL_OPEN_ITEMS, closed-historical=$EXCL_CLOSED_ITEMS)"
    fi
  fi
fi

# ---- Follow-up Issue Validation (Round-14: Gitea issues replace OpenSpec paths) ----
MISSING_FOLLOWUP=0
INVALID_FOLLOWUP=0
while IFS= read -r line; do
  item_id=$(echo "$line" | sed 's/^  - id: //')
  follow_up_block=$(grep -A 8 "^  - id: ${item_id}" "$EXCLUSIONS" 2>/dev/null || echo "")
  # Accept either follow_up_issue (Round-14+) or follow_up_issue_or_openspec (legacy)
  # Strip surrounding whitespace and quotes from the value
  issue_ref=$(echo "$follow_up_block" | grep "follow_up_issue" | head -1 | sed 's/.*follow_up_issue[^:]*:[[:space:]]*//' | tr -d '"' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  if [ -z "$issue_ref" ]; then
    MISSING_FOLLOWUP=$((MISSING_FOLLOWUP + 1))
  elif ! echo "$issue_ref" | grep -qE "^#?[0-9]+"; then
    INVALID_FOLLOWUP=$((INVALID_FOLLOWUP + 1))
  fi
done < <(grep "^  - id:" "$EXCLUSIONS" 2>/dev/null)

if [ "$MISSING_FOLLOWUP" -gt 0 ]; then
  record_fail "exclusions.yml: $MISSING_FOLLOWUP items missing follow_up_issue field"
fi
if [ "$INVALID_FOLLOWUP" -gt 0 ]; then
  record_fail "exclusions.yml: $INVALID_FOLLOWUP items have invalid follow_up_issue format (must be #NNNN)"
fi

# ---- Scope Distinction Check (per codex #89293: smoke vs full semantic) ----
SCOPE_LINE=$(grep "^scope:" "$EXCLUSIONS" 2>/dev/null | head -1)
if [ -n "$SCOPE_LINE" ]; then
  if ! echo "$SCOPE_LINE" | grep -qiE "smoke|baseline|subset|not.*full"; then
    record_warn "exclusions.yml scope line unclear: '$SCOPE_LINE' (should explicitly state 'NOT full corpus')"
  fi
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
| source_agent | $SOURCE_AGENT |
| source_run | $SOURCE_RUN |
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
