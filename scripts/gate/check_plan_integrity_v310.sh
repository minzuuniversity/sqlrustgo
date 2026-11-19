#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：归档候选（archive-candidate）
# 责任任务：P0-4
# 登记说明：头部绑定历史版本线，v4.1.0 的 STAGE_CONFIG 与 CI 都不引用它 ⇒ 归档候选；删除门禁脚本属门禁清单变更，须 owner 批准
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
#
# check_plan_integrity_v310.sh — v3.10.0 Plan Document Integrity Check
#
# Purpose: Ensure plan documents have not been rewritten to falsify history.
#          Plan documents are historical records — append-only, not rewritable.
#
# G-05 / B8-2 checks:
#   1. Plan docs not heavily rewritten (git line count comparison)
#   2. No suspicious status markers (GA Final, GA APPROVED, etc.)
#   3. No suspicious modification during gate period
#   4. TEST_PLAN.md has actual execution evidence (CI run IDs / commit hashes)
#   5. Development plan tasks reflect actual delivery
#
# Usage: ./check_plan_integrity_v310.sh <version> <out_dir>

set -euo pipefail

VERSION="${1:-}"
OUT_DIR="${2:-}"

if [ -z "$VERSION" ] || [ -z "$OUT_DIR" ]; then
  echo "Usage: $0 <version> <out_dir>" >&2
  exit 2
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

mkdir -p "$OUT_DIR"

RELEASE_DIR="$REPO_ROOT/docs/releases/$VERSION"

# Result counters (bash 3.2 compatible)
PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

add_pass() { PASS_COUNT=$((PASS_COUNT + 1)); echo "  [PASS] $1"; }
add_fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "  [FAIL] $1"; }
add_warn() { WARN_COUNT=$((WARN_COUNT + 1)); echo "  [WARN] $1"; }

# v3.10.0 plan documents to check
PLAN_DOCS_V310=(
  "VERSION_PLAN.md"
  "TEST_PLAN.md"
  "FEATURE_CHECKLIST.md"
  "CHANGELOG.md"
  "ARCHITECTURE.md"
  "plans/V310_DEVELOPMENT_PLAN.md"
  "plans/V310_ISSUES_PLAN.md"
)

echo "=== Plan Integrity Check for v$VERSION ==="
echo ""

# ============================================================
# Check 1: Plan doc not heavily rewritten
# Strategy: current line count >= 30% of historical max
# ============================================================
check_not_rewritten() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    add_warn "Plan doc not found (skipping rewrite check): $doc"
    return
  fi

  # Get max lines in git history for this file
  local max_lines=0
  max_lines=$(git log --follow --format= --"$path" 2>/dev/null | wc -l | tr -d ' ' || echo "0")

  local current_lines
  current_lines=$(wc -l < "$path" | tr -d ' ')

  if [ -n "$max_lines" ] && [ -n "$current_lines" ] && [ "$max_lines" -gt 0 ] 2>/dev/null; then
    local threshold
    threshold=$((max_lines * 30 / 100))
    if [ "$current_lines" -lt "$threshold" ]; then
      add_fail "Plan doc appears rewritten: $doc (current=${current_lines}, hist_max=${max_lines}, 30%_threshold=${threshold})"
    else
      add_pass "Line count healthy: $doc (${current_lines} lines, hist_max=${max_lines})"
    fi
  else
    add_warn "Cannot get git history for: $doc (no prior commits?)"
  fi
}

# ============================================================
# Check 2: No suspicious status markers
# GA Final / GA APPROVED / GA ✅ / Final Report in plan docs = suspicious
# ============================================================
check_no_suspicious_status() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # Suspicious patterns in plan documents
  # Note: "GA" alone is not suspicious — GA Final Report / GA APPROVED / GA ✅ are
  # Note 2: GA in table rows (requirements matrix) are OK — only prose claims are suspicious
  local patterns="GA.*Final.*Report|GA.*APPROVED|GA.*✅|Final Report|GA complete"

  # Use a temp file since we can't use process substitution on older bash
  local suspicious_tmp="$OUT_DIR/.suspicious_$$.tmp"
  grep -nE "$patterns" "$path" > "$suspicious_tmp" 2>/dev/null || true

  if [ -s "$suspicious_tmp" ]; then
    # Check if it's in the header (first 5 lines = allowed)
    # OR if it's inside a table row (line starts with |) — requirements table cells are OK
    local first_line
    first_line=$(head -1 "$suspicious_tmp" | cut -d: -f1)
    local first_content
    first_content=$(sed -n "${first_line}p" "$path" | grep -cE "^\s*\|" || echo "0")
    if [ "$first_line" -gt 5 ] && [ "$first_content" -eq 0 ]; then
      add_fail "Suspicious status marker in $doc: $(head -1 "$suspicious_tmp" | cut -c1-60)"
    else
      add_pass "Status marker in header/table (acceptable): $doc"
    fi
  else
    add_pass "No suspicious status markers: $doc"
  fi

  rm -f "$suspicious_tmp"
}

# ============================================================
# Check 3: No suspicious modification during gate period
# ============================================================
check_no_gate_period_modification() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  local last_commit_msg
  last_commit_msg=$(git log -1 --format="%s" --"$path" 2>/dev/null || echo "")

  if [ -n "$last_commit_msg" ]; then
    if echo "$last_commit_msg" | grep -qiE "GA|finalize|final|report|v3\.10\.0.*beta|v3\.10\.0.*release"; then
      add_warn "Gate-period modification: $doc — last commit: $last_commit_msg"
    fi
  fi
}

# ============================================================
# Check 4: TEST_PLAN.md has actual execution evidence
# ============================================================
check_test_plan_evidence() {
  local test_plan="$RELEASE_DIR/TEST_PLAN.md"

  if [ ! -f "$test_plan" ]; then
    add_fail "TEST_PLAN.md not found"
    return
  fi

  # Count evidence bindings in TEST_PLAN.md
  # Evidence patterns: commit hash, CI run ID, test output, log file reference
  # Note: grep -cE exits 1 on no-match; use ||true to avoid pipefail; tr removes \n
  local evidence_count
  evidence_count=$(grep -cE \
    "(commit [0-9a-f]{7,40}|#[0-9]+|run_[0-9]+|ci_run|CI_RUN|log_|TEST_PLAN|test result)" \
    "$test_plan" 2>/dev/null | tr -d ' \n' || true)
  evidence_count=${evidence_count:-0}

  # Count total test entries
  local test_entry_count
  test_entry_count=$(grep -cE "^##?\s+test|TEST-|Test Case" "$test_plan" 2>/dev/null | tr -d ' \n' || true)
  test_entry_count=${test_entry_count:-0}

  if [ "$test_entry_count" -gt 0 ]; then
    local binding_ratio
    binding_ratio=$(( evidence_count * 100 / test_entry_count ))
    if [ "$binding_ratio" -ge 50 ]; then
      add_pass "TEST_PLAN.md evidence binding: ${evidence_count} bindings for ${test_entry_count} test entries (${binding_ratio}%)"
    else
      add_warn "TEST_PLAN.md low evidence binding: ${evidence_count} bindings for ${test_entry_count} entries (${binding_ratio}%, target >=50%)"
    fi
  else
    add_warn "TEST_PLAN.md: no test entries found to check"
  fi
}

# ============================================================
# Check 5: Development plan tasks match actual delivery
# ============================================================
check_dev_plan_delivery() {
  local dev_plan="$RELEASE_DIR/plans/V310_DEVELOPMENT_PLAN.md"

  if [ ! -f "$dev_plan" ]; then
    add_warn "V310_DEVELOPMENT_PLAN.md not found"
    return
  fi

  # Count tasks with status markers
  local done_tasks
  done_tasks=$(grep -cE "\[x\]|\[done\]|DONE|完成|CLOSE" "$dev_plan" 2>/dev/null || echo "0")
  local total_tasks
  total_tasks=$(grep -cE "\[.?\]|\[x\]|\[-?\]|TODO|DONE|WIP|进行中|完成|OPEN|CLOSED" "$dev_plan" 2>/dev/null || echo "0")

  if [ "$total_tasks" -gt 0 ]; then
    local done_ratio
    done_ratio=$(( done_tasks * 100 / total_tasks ))
    add_pass "V310_DEVELOPMENT_PLAN.md: ${done_tasks}/${total_tasks} tasks done (${done_ratio}%)"

    # Check for suspiciously high done ratio with no evidence
    if [ "$done_ratio" -gt 90 ]; then
      local high_evidence
      high_evidence=$(grep -cE "(commit [0-9a-f]{7,40}|#[0-9]+|PR|merge|merged)" "$dev_plan" 2>/dev/null || echo "0")
      if [ "$high_evidence" -lt 5 ]; then
        add_warn "High done ratio (${done_ratio}%) but low commit/PR evidence in plan"
      fi
    fi
  else
    add_warn "V310_DEVELOPMENT_PLAN.md: no task markers found"
  fi
}

# ============================================================
# Execute all checks
# ============================================================
echo "--- Check 1: Plan Rewrite Detection ---"
for doc in "${PLAN_DOCS_V310[@]}"; do
  check_not_rewritten "$doc"
done
echo ""

echo "--- Check 2: Suspicious Status Markers ---"
for doc in "${PLAN_DOCS_V310[@]}"; do
  check_no_suspicious_status "$doc"
done
echo ""

echo "--- Check 3: Gate-Period Modification ---"
for doc in "${PLAN_DOCS_V310[@]}"; do
  check_no_gate_period_modification "$doc"
done
echo ""

echo "--- Check 4: TEST_PLAN.md Evidence ---"
check_test_plan_evidence
echo ""

echo "--- Check 5: Development Plan Delivery ---"
check_dev_plan_delivery
echo ""

# ============================================================
# Report
# ============================================================
REPORT_FILE="$OUT_DIR/PLAN_INTEGRITY_V310_REPORT.md"
cat > "$REPORT_FILE" << EOF
# v$VERSION Plan Integrity Report

## Summary
- PASS: $PASS_COUNT
- WARN: $WARN_COUNT
- FAIL: $FAIL_COUNT

## Date
$(date -u +%Y-%m-%dT%H:%M:%SZ)

## Verdict
EOF

if [ $FAIL_COUNT -eq 0 ]; then
  echo "**PASS** — No plan integrity violations found" >> "$REPORT_FILE"
else
  echo "**FAIL** — $FAIL_COUNT integrity violation(s) found" >> "$REPORT_FILE"
fi

echo ""
echo "Report: $REPORT_FILE"
echo "PASS=$PASS_COUNT, WARN=$WARN_COUNT, FAIL=$FAIL_COUNT"

if [ $FAIL_COUNT -eq 0 ]; then
  echo ""
  echo "OVERALL: PASS — plan documents have integrity"
  exit 0
else
  echo ""
  echo "OVERALL: FAIL — $FAIL_COUNT plan integrity violation(s)"
  exit 1
fi
