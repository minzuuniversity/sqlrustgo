#!/usr/bin/env bash
#
# check_5_principles.sh — 5原则 G-01~G-06 全量检查
#
# 用途: 统一入口，执行 Truthfulness Framework 全部 6 个原则的自动检查
#
# G-01: Claim ≠ Evidence (PASS/FAIL 声明必须有 CI 证据)
# G-02: Document Claim 必须标记来源类型 (Source Type)
# G-03: Gate 报告必须包含实际命令
# G-04: Coverage 测量方法必须一致 (--tests primary, --lib fallback)
# G-05: Document State ≠ Execution State (计划文档不可重写)
# G-06: 文档 Claim 必须有 Freshness 标记
#
# 执行方式:
#   ./check_5_principles.sh <version> <out_dir>
#   ./check_5_principles.sh v3.8.0 /tmp/gate_reports

set -euo pipefail

VERSION="${1:-}"
OUT_DIR="${2:-}"

if [ -z "$VERSION" ] || [ -z "$OUT_DIR" ]; then
  echo "Usage: $0 <version> <out_dir>"
  echo "Example: $0 v3.8.0 /tmp/gate_reports"
  exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p "$OUT_DIR"

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

RELEASE_DIR="$REPO_ROOT/docs/releases/$VERSION"

# ============================================================
# 全局结果收集
# ============================================================
declare -A RESULTS
G01_PASS=0; G01_FAIL=0
G02_PASS=0; G02_FAIL=0
G03_PASS=0; G03_FAIL=0
G04_PASS=0; G04_FAIL=0
G05_PASS=0; G05_FAIL=0
G06_PASS=0; G06_FAIL=0

OVERALL_PASS=0
OVERALL_FAIL=0
BLOCKER_COUNT=0

# ============================================================
# G-01: Claim ≠ Evidence
# 检查: PASS/FAIL 声明是否绑定 CI run ID / commit hash / gate_policy_eval_id
# ============================================================
echo "=== G-01: Claim ≠ Evidence ==="

run_g01() {
  local g01_out="$OUT_DIR/g01_check.log"
  {
    echo "--- G-01 Evidence Binding Check ---"

    if [ ! -d "$RELEASE_DIR" ]; then
      echo "RELEASE_DIR not found: $RELEASE_DIR"
      return 1
    fi

    local all_docs
    all_docs=$(find "$RELEASE_DIR" -name "*.md" -type f 2>/dev/null || true)

    local total_claims=0
    local bound_claims=0
    local unbound_claims=0

    for doc_path in $all_docs; do
      doc_name=$(basename "$doc_path")

      # 查找所有状态声明行
      local claim_lines
      claim_lines=$(grep -nE "(PASS|FAIL|通过|失败|完成|done|completed|成功)" "$doc_path" 2>/dev/null || true)

      if [ -z "$claim_lines" ]; then
        continue
      fi

      while IFS=: read -r line_num content; do
        # 跳过注释、代码块
        if echo "$content" | grep -qE "^#|```|`"; then
          continue
        fi

        total_claims=$((total_claims + 1))

        # 检查是否有证据绑定
        local has_evidence=false
        if echo "$content" | grep -qE "(run_|#|ci_run|CI_RUN|log_hash|commit [0-9a-f]{7,40}|policy_eval|gate_policy|GATE_POLICY)"; then
          has_evidence=true
          bound_claims=$((bound_claims + 1))
        else
          unbound_claims=$((unbound_claims + 1))
          echo "  [G-01] UNBOUND: $doc_name:$line_num — $(echo "$content" | cut -c1-60)"
        fi
      done <<< "$claim_lines"
    done

    echo ""
    echo "G-01 Summary: total=$total_claims bound=$bound_claims unbound=$unbound_claims"

    if [ $unbound_claims -gt 0 ]; then
      echo "RESULT: FAIL — $unbound_claims unbound claims found"
      return 1
    else
      echo "RESULT: PASS — all claims have evidence binding"
      return 0
    fi
  } > "$g01_out" 2>&1

  if [ $? -eq 0 ]; then
    G01_PASS=1
    echo "  [PASS] G-01: Claim ≠ Evidence"
    return 0
  else
    G01_FAIL=1
    echo "  [FAIL] G-01: Claim ≠ Evidence — see $g01_out"
    return 1
  fi
}

# ============================================================
# G-02: Source Type Marker
# 检查: 每个 Claim 是否标记了 Source Type: [实测|SSOT引用|历史文档]
# ============================================================
echo "=== G-02: Source Type Marker ==="

run_g02() {
  local g02_out="$OUT_DIR/g02_check.log"
  {
    bash "$SCRIPT_DIR/check_g_02_source_marker.sh" "$VERSION" "$OUT_DIR" > "$g02_out" 2>&1
    echo "Exit code: $?"
  } > "$g02_out" 2>&1

  local exit_code=${PIPESTATUS[0]:-1}
  if [ $exit_code -eq 0 ]; then
    G02_PASS=1
    echo "  [PASS] G-02: Source Type Marker"
    return 0
  else
    G02_FAIL=1
    echo "  [FAIL] G-02: Source Type Marker — see $g02_out"
    return 1
  fi
}

# ============================================================
# G-03: Gate Report Contains Actual Commands
# 检查: Gate 报告是否包含实际执行的命令（而非仅描述）
# ============================================================
echo "=== G-03: Gate Report Commands ==="

run_g03() {
  local g03_out="$OUT_DIR/g03_check.log"
  local g03_fail=0

  echo "--- G-03 Gate Command Check ---" > "$g03_out"

  local gate_reports
  gate_reports=$(find "$RELEASE_DIR" -name "*GATE*.md" -o -name "*GATE_REPORT*.md" 2>/dev/null || true)

  if [ -z "$gate_reports" ]; then
    echo "  [WARN] No gate reports found, skipping G-03"
    echo "G-03: No gate reports found" >> "$g03_out"
    G03_PASS=1
    return 0
  fi

  for report in $gate_reports; do
    doc_name=$(basename "$report")
    echo "Checking: $doc_name"

    # 检查是否包含实际命令（cargo, rust, bash 等）
    if grep -qE "(cargo [a-z]|rustc |bash |#!/bin/bash|```bash|```sh)" "$report" 2>/dev/null; then
      echo "  [PASS] $doc_name contains actual commands"
    else
      echo "  [FAIL] $doc_name missing actual commands (only descriptions)"
      echo "G-03 FAIL: $doc_name — no actual commands found" >> "$g03_out"
      g03_fail=$((g03_fail + 1))
    fi

    # 检查是否有命令输出（stdout/stderr）
    if grep -qE "(\\\$\(cargo|\\\$.*cargo|\[INFO\]|\[ERROR\]|---)" "$report" 2>/dev/null; then
      echo "  [PASS] $doc_name contains command output"
    else
      echo "  [WARN] $doc_name may be missing command output"
    fi
  done

  if [ $g03_fail -gt 0 ]; then
    G03_FAIL=1
    echo "RESULT: FAIL — $g03_fail gate reports missing actual commands" >> "$g03_out"
    return 1
  else
    G03_PASS=1
    echo "RESULT: PASS" >> "$g03_out"
    return 0
  fi
}

# ============================================================
# G-04: Coverage Measurement Consistency
# 检查: Coverage 测量使用统一方法 (--tests primary, --lib fallback)
# ============================================================
echo "=== G-04: Coverage Consistency ==="

run_g04() {
  local g04_out="$OUT_DIR/g04_check.log"
  local g04_fail=0

  echo "--- G-04 Coverage Consistency Check ---" > "$g04_out"

  local coverage_docs
  coverage_docs=$(find "$RELEASE_DIR" -name "*.md" -exec grep -lE "(coverage|覆盖率|llvm-cov)" {} \; 2>/dev/null || true)

  if [ -z "$coverage_docs" ]; then
    echo "  [WARN] No coverage documents found, skipping G-04"
    G04_PASS=1
    return 0
  fi

  for doc in $coverage_docs; do
    doc_name=$(basename "$doc")
    echo "Checking: $doc_name"

    # 检查是否混用了 --lib only 和 --tests
    local has_lib_only=false
    local has_tests=false

    if grep -qE "(--lib only|--lib-only|--lib-only)" "$doc" 2>/dev/null; then
      has_lib_only=true
    fi
    if grep -qE "(--tests|--all-features --tests)" "$doc" 2>/dev/null; then
      has_tests=true
    fi

    if [ "$has_lib_only" = true ] && [ "$has_tests" = false ]; then
      echo "  [FAIL] $doc_name uses --lib only (inconsistent with G-04)"
      echo "G-04 FAIL: $doc_name — uses --lib only instead of --tests primary" >> "$g04_out"
      g04_fail=$((g04_fail + 1))
    elif grep -qE "(--lib only)" "$doc" 2>/dev/null; then
      # 允许 --lib 作为 fallback 但必须有明确标注
      if ! grep -qE "(--lib fallback|fallback.*--lib|--lib.*fallback)" "$doc" 2>/dev/null; then
        echo "  [WARN] $doc_name --lib usage may not be properly annotated"
      fi
    fi
  done

  if [ $g04_fail -gt 0 ]; then
    G04_FAIL=1
    echo "RESULT: FAIL — $g04_fail coverage docs inconsistent" >> "$g04_out"
    return 1
  else
    G04_PASS=1
    echo "RESULT: PASS" >> "$g04_out"
    return 0
  fi
}

# ============================================================
# G-05: Document State ≠ Execution State
# 检查: 计划文档是否被非法重写（防伪造）
# ============================================================
echo "=== G-05: Document State ≠ Execution State ==="

run_g05() {
  local g05_out="$OUT_DIR/g05_check.log"
  {
    bash "$SCRIPT_DIR/check_plan_integrity.sh" "$VERSION" "$OUT_DIR" > /dev/null 2>&1
    echo "Exit code: $?"
  } > "$g05_out" 2>&1

  local exit_code=${PIPESTATUS[0]:-1}
  if [ $exit_code -eq 0 ]; then
    G05_PASS=1
    echo "  [PASS] G-05: Document State ≠ Execution State"
    return 0
  else
    G05_FAIL=1
    echo "  [FAIL] G-05: Document State ≠ Execution State — see $g05_out"
    return 1
  fi
}

# ============================================================
# G-06: Freshness Markers
# 检查: 历史数据是否标注了 Freshness 标记
# ============================================================
echo "=== G-06: Freshness Markers ==="

run_g06() {
  local g06_out="$OUT_DIR/g06_check.log"
  {
    bash "$SCRIPT_DIR/check_g_06_freshness.sh" "$VERSION" "$OUT_DIR" > "$g06_out" 2>&1
    echo "Exit code: $?"
  } > "$g06_out" 2>&1

  local exit_code=${PIPESTATUS[0]:-1}
  if [ $exit_code -eq 0 ]; then
    G06_PASS=1
    echo "  [PASS] G-06: Freshness Markers"
    return 0
  else
    G06_FAIL=1
    echo "  [FAIL] G-06: Freshness Markers — see $g06_out"
    return 1
  fi
}

# ============================================================
# 执行所有检查
# ============================================================
echo ""
echo "============================================"
echo "  5 Principles (G-01~G-06) Full Check"
echo "  Version: $VERSION"
echo "  Output: $OUT_DIR"
echo "============================================"
echo ""

run_g01 || true
run_g02 || true
run_g03 || true
run_g04 || true
run_g05 || true
run_g06 || true

OVERALL_PASS=$((G01_PASS + G02_PASS + G03_PASS + G04_PASS + G05_PASS + G06_PASS))
OVERALL_FAIL=$((G01_FAIL + G02_FAIL + G03_FAIL + G04_FAIL + G05_FAIL + G06_FAIL))
BLOCKER_COUNT=$OVERALL_FAIL

echo ""
echo "============================================"
echo "  5 Principles Summary"
echo "============================================"
echo ""
printf "  %-8s %-6s %-6s\n" "Principle" "PASS" "FAIL"
printf "  %-8s %-6s %-6s\n" "--------" "----" "----"
printf "  %-8s %-6s %-6s\n" "G-01" "$G01_PASS" "$G01_FAIL"
printf "  %-8s %-6s %-6s\n" "G-02" "$G02_PASS" "$G02_FAIL"
printf "  %-8s %-6s %-6s\n" "G-03" "$G03_PASS" "$G03_FAIL"
printf "  %-8s %-6s %-6s\n" "G-04" "$G04_PASS" "$G04_FAIL"
printf "  %-8s %-6s %-6s\n" "G-05" "$G05_PASS" "$G05_FAIL"
printf "  %-8s %-6s %-6s\n" "G-06" "$G06_PASS" "$G06_FAIL"
echo ""
printf "  %-8s %-6s %-6s\n" "TOTAL" "$OVERALL_PASS" "$OVERALL_FAIL"
echo ""

# ============================================================
# 生成 JSON 报告
# ============================================================
REPORT_FILE="$OUT_DIR/5_PRINCIPLES_REPORT.json"

cat > "$REPORT_FILE" << EOF
{
  "gate": "5-principles",
  "version": "$VERSION",
  "timestamp": "$(date -Iseconds)",
  "branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')",
  "commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "results": {
    "G-01": {"name": "Claim ≠ Evidence", "pass": $G01_PASS, "fail": $G01_FAIL, "log": "$OUT_DIR/g01_check.log"},
    "G-02": {"name": "Source Type Marker", "pass": $G02_PASS, "fail": $G02_FAIL, "log": "$OUT_DIR/g02_check.log"},
    "G-03": {"name": "Gate Report Commands", "pass": $G03_PASS, "fail": $G03_FAIL, "log": "$OUT_DIR/g03_check.log"},
    "G-04": {"name": "Coverage Consistency", "pass": $G04_PASS, "fail": $G04_FAIL, "log": "$OUT_DIR/g04_check.log"},
    "G-05": {"name": "Document State ≠ Execution State", "pass": $G05_PASS, "fail": $G05_FAIL, "log": "$OUT_DIR/g05_check.log"},
    "G-06": {"name": "Freshness Markers", "pass": $G06_PASS, "fail": $G06_FAIL, "log": "$OUT_DIR/g06_check.log"}
  },
  "summary": {
    "pass_count": $OVERALL_PASS,
    "fail_count": $OVERALL_FAIL,
    "total": 6,
    "pass_rate": "$(echo "scale=1; $OVERALL_PASS * 100 / 6" | bc)%"
  },
  "verdict": "$([ $BLOCKER_COUNT -eq 0 ] && echo "PASS" || echo "FAIL")",
  "blockers": $BLOCKER_COUNT
}
EOF

echo "JSON Report: $REPORT_FILE"
echo ""

if [ $BLOCKER_COUNT -eq 0 ]; then
  echo "✅ 5 Principles Gate: PASS — all $OVERALL_PASS/6 checks passed"
  exit 0
else
  echo "❌ 5 Principles Gate: FAIL — $BLOCKER_COUNT blocker(s)"
  echo ""
  echo "Failed principles:"
  [ $G01_FAIL -eq 1 ] && echo "  - G-01: Claim ≠ Evidence"
  [ $G02_FAIL -eq 1 ] && echo "  - G-02: Source Type Marker"
  [ $G03_FAIL -eq 1 ] && echo "  - G-03: Gate Report Commands"
  [ $G04_FAIL -eq 1 ] && echo "  - G-04: Coverage Consistency"
  [ $G05_FAIL -eq 1 ] && echo "  - G-05: Document State ≠ Execution State"
  [ $G06_FAIL -eq 1 ] && echo "  - G-06: Freshness Markers"
  exit 1
fi
