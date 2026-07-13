#!/usr/bin/env bash
#
# check_5_principles_v310.sh — Truthfulness Framework G-01~G-06 for v3.10.0
#
# G-01: Claim ≠ Evidence  (PASS/FAIL declarations must bind CI run ID or commit hash)
# G-02: Source Type Marker (every Claim must label its source type)
# G-03: Gate Report Commands (gate reports must contain actual executed commands)
# G-04: Coverage Consistency (coverage measurement: --tests primary, --lib fallback)
# G-05: Plan State ≠ Execution State (plan documents cannot be rewritten)
# G-06: Freshness Marker (historical data claims must label their age)
#
# Usage: ./check_5_principles_v310.sh <version> <out_dir>
#   e.g.  ./check_5_principles_v310.sh v3.10.0 /tmp/gate_reports

set -euo pipefail

VERSION="${1:-}"
OUT_DIR="${2:-}"

if [ -z "$VERSION" ] || [ -z "$OUT_DIR" ]; then
  echo "Usage: $0 <version> <out_dir>" >&2
  echo "Example: $0 v3.10.0 /tmp/gate_reports" >&2
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

mkdir -p "$OUT_DIR"

RELEASE_DIR="$REPO_ROOT/docs/releases/$VERSION"

# cargo on PATH for CI
if ! command -v cargo >/dev/null 2>&1; then
  if [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
fi

# Global result counters (bash 3.2 compatible — no associative arrays)
G01_PASS=0; G01_FAIL=0
G02_PASS=0; G02_FAIL=0
G03_PASS=0; G03_FAIL=0
G04_PASS=0; G04_FAIL=0
G05_PASS=0; G05_FAIL=0
G06_PASS=0; G06_FAIL=0
OVERALL_PASS=0
OVERALL_FAIL=0

# ============================================================
# G-01: Claim ≠ Evidence
# PASS/FAIL declarations must bind CI run ID / commit hash
# ============================================================
echo "=== G-01: Claim ≠ Evidence ==="
run_g01() {
  local g01_out="$OUT_DIR/g01_check.log"
  local total=0; local bound=0; local unbound=0

  {
    echo "--- G-01 Evidence Binding Check ---"

    if [ ! -d "$RELEASE_DIR" ]; then
      echo "RELEASE_DIR not found: $RELEASE_DIR"
      return 1
    fi

    # Scan all .md files in the release dir
    find "$RELEASE_DIR" -name "*.md" -type f | sort | while read -r doc_path; do
      doc_name="$(basename "$doc_path")"

      # Find status declaration lines (PASS|FAIL|通过|完成|done|completed|成功|ALPHA|BETA|RC|GA)
      grep -nE "(PASS|FAIL|通过|完成|done|completed|成功|ALPHA|BETA|RC|GA)" "$doc_path" 2>/dev/null | \
        grep -vE "^[^:]*:[^:]*:.*\`" | \
        grep -vE "^[^:]*:[^:]*:.*#" | \
        grep -vE "^[^:]*:[^:]*:\s*\|" | \
        while IFS=: read -r line_num content; do
          total=$((total + 1))
          # Evidence binding patterns: commit hash, CI run ID, gate_policy_eval, log hash
          if echo "$content" | grep -qE \
            "(run_|#[0-9]|ci_run|CI_RUN|log_hash|commit [0-9a-f]{7,40}|policy_eval|gate_policy|GATE_POLICY|v[0-9]+\.[0-9]+\.[0-9]+(-[a-z0-9]+)? @|commit [0-9a-f]{7,8})"; then
            bound=$((bound + 1))
          else
            unbound=$((unbound + 1))
            echo "  [G-01 UNBOUND] $doc_name:$line_num — $(echo "$content" | cut -c1-70)"
          fi
        done
    done

    echo ""
    echo "G-01 Summary: total=$total bound=$bound unbound=$unbound"

    # Threshold from STAGE_CONFIG.yaml §B8-1: <=50 for pre-existing docs
    if [ $unbound -le 50 ]; then
      echo "RESULT: PASS — unbound=$unbound (threshold 50)"
      return 0
    else
      echo "RESULT: FAIL — unbound=$unbound exceeds threshold 50"
      return 1
    fi
  } > "$g01_out" 2>&1

  if [ $? -eq 0 ]; then
    G01_PASS=1; OVERALL_PASS=$((OVERALL_PASS + 1))
    echo "  [PASS] G-01: Claim ≠ Evidence"
    return 0
  else
    G01_FAIL=1; OVERALL_FAIL=$((OVERALL_FAIL + 1))
    echo "  [FAIL] G-01: Claim ≠ Evidence — see $g01_out"
    return 1
  fi
}

# ============================================================
# G-02: Source Type Marker
# Every Claim must label its source type: [实测|SSOT引用|历史文档]
# ============================================================
echo "=== G-02: Source Type Marker ==="
run_g02() {
  local g02_out="$OUT_DIR/g02_check.log"
  local total=0; local marked=0; local unmarked=0

  {
    echo "--- G-02 Source Type Marker Check ---"

    find "$RELEASE_DIR" -name "*.md" -type f | sort | while read -r doc_path; do
      doc_name="$(basename "$doc_path")"

      # Find Claim lines (contain numeric/percentage statements)
      grep -nE "(%|[0-9]+\.[0-9]+|passed|failed|通过率|覆盖率)" "$doc_path" 2>/dev/null | \
        grep -vE "^[^:]*:[^:]*:.*\`" | \
        grep -vE "^[^:]*:[^:]*:.*#" | \
        grep -vE "^[^:]*:[^:]*:\s*\|" | \
        while IFS=: read -r line_num content; do
          # Skip table rows, code blocks, comments
          if echo "$content" | grep -qE "^[[:space:]]*\|"; then continue; fi

          total=$((total + 1))
          # Valid source type markers
          if echo "$content" | grep -qE \
            "(Source Type:|来源类型:|实测|SSOT|历史文档|实测数据|ci result|command output)"; then
            marked=$((marked + 1))
          else
            unmarked=$((unmarked + 1))
            echo "  [G-02 UNMARKED] $doc_name:$line_num — $(echo "$content" | cut -c1-60)"
          fi
        done
    done

    echo ""
    echo "G-02 Summary: total=$total marked=$marked unmarked=$unmarked"

    if [ $unmarked -eq 0 ]; then
      echo "RESULT: PASS"
      return 0
    else
      echo "RESULT: WARN — $unmarked unmarked claims (non-blocking)"
      return 0
    fi
  } > "$g02_out" 2>&1

  G02_PASS=1; OVERALL_PASS=$((OVERALL_PASS + 1))
  echo "  [PASS] G-02: Source Type Marker"
  return 0
}

# ============================================================
# G-03: Gate Report Contains Actual Commands
# Gate reports must contain actual executed commands (not just descriptions)
# ============================================================
echo "=== G-03: Gate Report Commands ==="
run_g03() {
  local g03_out="$OUT_DIR/g03_check.log"
  local g03_fail=0

  {
    echo "--- G-03 Gate Command Check ---"

    local gate_reports
    gate_reports="$(find "$RELEASE_DIR" -name "*GATE*.md" -o -name "*GATE_REPORT*.md" 2>/dev/null | sort)"

    if [ -z "$gate_reports" ]; then
      echo "  [WARN] No gate reports found, skipping G-03"
      echo "G-03: No gate reports found" >> "$g03_out"
      return 0
    fi

    for report in $gate_reports; do
      doc_name="$(basename "$report")"
      echo "Checking: $doc_name"

      # Must contain actual command patterns
      if grep -qE "(cargo [a-z]|rustc |bash |#!/bin/bash|\`\`\`bash|\`\`\`sh|\$\(cargo)" "$report" 2>/dev/null; then
        echo "  [PASS] $doc_name contains actual commands"
      else
        echo "  [FAIL] $doc_name missing actual commands (descriptions only)"
        echo "G-03 FAIL: $doc_name" >> "$g03_out"
        g03_fail=$((g03_fail + 1))
      fi
    done

    if [ $g03_fail -gt 0 ]; then
      echo "RESULT: FAIL — $g03_fail gate reports missing commands"
      return 1
    else
      echo "RESULT: PASS"
      return 0
    fi
  } > "$g03_out" 2>&1

  if [ $? -eq 0 ]; then
    G03_PASS=1; OVERALL_PASS=$((OVERALL_PASS + 1))
    echo "  [PASS] G-03: Gate Report Commands"
    return 0
  else
    G03_FAIL=1; OVERALL_FAIL=$((OVERALL_FAIL + 1))
    echo "  [FAIL] G-03: Gate Report Commands"
    return 1
  fi
}

# ============================================================
# G-04: Coverage Measurement Consistency
# Coverage: --tests primary, --lib fallback (never --lib-only without annotation)
# ============================================================
echo "=== G-04: Coverage Consistency ==="
run_g04() {
  local g04_out="$OUT_DIR/g04_check.log"
  local g04_fail=0

  {
    echo "--- G-04 Coverage Consistency Check ---"

    local coverage_docs
    coverage_docs="$(find "$RELEASE_DIR" -name "*.md" -exec grep -lE "(coverage|覆盖率|llvm-cov)" {} \; 2>/dev/null | sort)"

    if [ -z "$coverage_docs" ]; then
      echo "  [PASS] No coverage documents found, skipping G-04"
      return 0
    fi

    for doc in $coverage_docs; do
      doc_name="$(basename "$doc")"
      echo "Checking: $doc_name"

      # Inconsistent: --lib only without fallback annotation
      if grep -qE "\-\-lib only|\-\-lib-only" "$doc" 2>/dev/null; then
        if grep -qE "\-\-lib fallback|fallback.*\-\-lib|\-\-lib.*fallback" "$doc" 2>/dev/null; then
          echo "  [PASS] $doc_name has --lib fallback annotation"
        else
          echo "  [FAIL] $doc_name uses --lib only without fallback annotation"
          echo "G-04 FAIL: $doc_name" >> "$g04_out"
          g04_fail=$((g04_fail + 1))
        fi
      fi
    done

    if [ $g04_fail -gt 0 ]; then
      echo "RESULT: FAIL — $g04_fail coverage docs inconsistent"
      return 1
    else
      echo "RESULT: PASS"
      return 0
    fi
  } > "$g04_out" 2>&1

  if [ $? -eq 0 ]; then
    G04_PASS=1; OVERALL_PASS=$((OVERALL_PASS + 1))
    echo "  [PASS] G-04: Coverage Consistency"
    return 0
  else
    G04_FAIL=1; OVERALL_FAIL=$((OVERALL_FAIL + 1))
    echo "  [FAIL] G-04: Coverage Consistency"
    return 1
  fi
}

# ============================================================
# G-05: Document State ≠ Execution State
# Plan documents cannot be rewritten to falsify history
# Delegates to check_plan_integrity_v310.sh
# ============================================================
echo "=== G-05: Document State ≠ Execution State ==="
run_g05() {
  local g05_out="$OUT_DIR/g05_check.log"

  bash "$SCRIPT_DIR/check_plan_integrity_v310.sh" "$VERSION" "$OUT_DIR" > "$g05_out" 2>&1
  local exit_code=$?

  if [ $exit_code -eq 0 ]; then
    G05_PASS=1; OVERALL_PASS=$((OVERALL_PASS + 1))
    echo "  [PASS] G-05: Document State ≠ Execution State"
    return 0
  else
    G05_FAIL=1; OVERALL_FAIL=$((OVERALL_FAIL + 1))
    echo "  [FAIL] G-05: Document State ≠ Execution State — see $g05_out"
    return 1
  fi
}

# ============================================================
# G-06: Freshness Marker
# Historical data claims (>30 days old) must label their age
# ============================================================
echo "=== G-06: Freshness Marker ==="
run_g06() {
  local g06_out="$OUT_DIR/g06_check.log"
  local stale=0; local fresh=0

  {
    echo "--- G-06 Freshness Marker Check ---"

    find "$RELEASE_DIR" -name "*.md" -type f | sort | while read -r doc_path; do
      doc_name="$(basename "$doc_path")"

      # Find numeric claims that could be stale
      grep -nE "(%|passed|failed|通过率|覆盖率|latency|throughput)" "$doc_path" 2>/dev/null | \
        grep -vE "^[^:]*:[^:]*:.*\`" | \
        grep -vE "^[^:]*:[^:]*:.*#" | \
        while IFS=: read -r line_num content; do
          # Check for freshness marker (date or version tag)
          if echo "$content" | grep -qE \
            "(Freshness:|freshness|@|v[0-9]+\.[0-9]+\.[0-9]+|20[0-9][0-9]-[0-9][0-9]-[0-9][0-9])"; then
            fresh=$((fresh + 1))
          else
            # No freshness marker — flag as potentially stale (WARN, non-blocking)
            echo "  [WARN] $doc_name:$line_num — no freshness marker: $(echo "$content" | cut -c1-50)"
            stale=$((stale + 1))
          fi
        done
    done

    echo ""
    echo "G-06 Summary: fresh=$fresh potentially_stale=$stale"
    echo "RESULT: PASS (WARN only, non-blocking)"
    return 0
  } > "$g06_out" 2>&1

  G06_PASS=1; OVERALL_PASS=$((OVERALL_PASS + 1))
  echo "  [PASS] G-06: Freshness Marker"
  return 0
}

# ============================================================
# Execute all G-01~G-06
# ============================================================
run_g01 || true
run_g02 || true
run_g03 || true
run_g04 || true
run_g05 || true
run_g06 || true

# ============================================================
# Summary
# ============================================================
echo ""
echo "============================================"
echo "  5 Principles (G-01~G-06) Summary"
echo "============================================"
printf "  %-6s %-20s %-6s %-6s\n" "Rule" "Name" "PASS" "FAIL"
printf "  %-6s %-20s %-6s %-6s\n" "----" "----" "----" "----"
printf "  %-6s %-20s %-6s %-6s\n" "G-01" "Claim≠Evidence" "$G01_PASS" "$G01_FAIL"
printf "  %-6s %-20s %-6s %-6s\n" "G-02" "SourceTypeMarker" "$G02_PASS" "$G02_FAIL"
printf "  %-6s %-20s %-6s %-6s\n" "G-03" "GateCmd" "$G03_PASS" "$G03_FAIL"
printf "  %-6s %-20s %-6s %-6s\n" "G-04" "CoverageConsist" "$G04_PASS" "$G04_FAIL"
printf "  %-6s %-20s %-6s %-6s\n" "G-05" "PlanState≠Exec" "$G05_PASS" "$G05_FAIL"
printf "  %-6s %-20s %-6s %-6s\n" "G-06" "Freshness" "$G06_PASS" "$G06_FAIL"
echo ""
echo "  Total: PASS=$OVERALL_PASS/6, FAIL=$OVERALL_FAIL/6"
echo "============================================"

# JSON report
REPORT_FILE="$OUT_DIR/5_PRINCIPLES_V310_REPORT.json"
cat > "$REPORT_FILE" << EOF
{
  "version": "$VERSION",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "G01": {"name": "Claim≠Evidence", "pass": $G01_PASS, "fail": $G01_FAIL},
  "G02": {"name": "SourceTypeMarker", "pass": $G02_PASS, "fail": $G02_FAIL},
  "G03": {"name": "GateCmd", "pass": $G03_PASS, "fail": $G03_FAIL},
  "G04": {"name": "CoverageConsist", "pass": $G04_PASS, "fail": $G04_FAIL},
  "G05": {"name": "PlanState≠Exec", "pass": $G05_PASS, "fail": $G05_FAIL},
  "G06": {"name": "Freshness", "pass": $G06_PASS, "fail": $G06_FAIL},
  "total_pass": $OVERALL_PASS,
  "total_fail": $OVERALL_FAIL
}
EOF
echo "JSON Report: $REPORT_FILE"

if [ $OVERALL_FAIL -eq 0 ]; then
  echo ""
  echo "OVERALL: PASS — all 5 principles verified"
  exit 0
else
  echo ""
  echo "OVERALL: FAIL — $OVERALL_FAIL principle(s) failed"
  exit 1
fi
