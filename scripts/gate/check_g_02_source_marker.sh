#!/usr/bin/env bash
#
# check_g_02_source_marker.sh — G-02 Document Claim 来源类型标记检查
#
# 用途: 检查文档中的 Claim 是否标记了来源类型
#
# G-02 要求:
# 所有文档中的 Claim 必须标记来源类型:
#   Claim: "Parser coverage 47%"
#   Source Type: [实测|SSOT引用|历史文档]
#   Source: ALPHA_GATE_REPORT.md §A5 / RC_GATE_REPORT.md commit aa830bcd
#
# 违规:
# - Claim 无 Source Type 标记
# - Source Type 为空或无效值
# - Source 引用不存在的文档

set -euo pipefail

VERSION="${1:-}"
OUT_DIR="${2:-}"

if [ -z "$VERSION" ] || [ -z "$OUT_DIR" ]; then
  echo "Usage: $0 <version> <out_dir>"
  exit 2
fi

mkdir -p "$OUT_DIR"

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

RELEASE_DIR="docs/releases/$VERSION"

# ============================================================
# 检查函数
# ============================================================

# 检查一个文档的 Claim 来源标记
check_doc_source_marker() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    echo "  [WARN] 文档不存在：$doc"
    return
  fi

  # 查找所有 Claim 行（包含数字或百分比的陈述性语句）
  # 匹配模式: "Claim:" 前缀 或 数字/百分比声明
  local claim_lines
  claim_lines=$(grep -nE "(Claim:|coverage|pass rate|%|[0-9]+\.[0-9]+%|passed|failed|通过率)" "$path" 2>/dev/null || true)

  if [ -z "$claim_lines" ]; then
    echo "  [PASS] 无 Claim 声明：$doc"
    PASS_COUNT=$((PASS_COUNT + 1))
    return
  fi

  local doc_fail=0
  local doc_pass=0

  while IFS=: read -r line_num content; do
    # 跳过注释行、代码块、表格行、空行
    if echo "$content" | grep -qE "^#|```|`|^[[:space:]]*\|"; then
      continue
    fi

    # 检查是否有 Source Type 标记（三种有效格式）
    local has_source_type=false
    if echo "$content" | grep -qE "Source Type:|Source-Type:|来源类型:|来源:"; then
      has_source_type=true
    fi

    # 检查是否是实质性 Claim（包含具体数值或状态声明）
    local is_substantive=false
    if echo "$content" | grep -qE "(%|passed|failed|通过|失败|[0-9]+\.[0-9]+)"; then
      is_substantive=true
    fi

    if [ "$is_substantive" = true ]; then
      if [ "$has_source_type" = false ]; then
        echo "  [FAIL] 第 $line_num 行 Claim 无 Source Type: $(echo "$content" | cut -c1-60)"
        doc_fail=$((doc_fail + 1))
      else
        # 进一步检查 Source 字段是否有值
        if echo "$content" | grep -qE "Source Type:[[:space:]]*$|Source-Type:[[:space:]]*$|来源类型:[[:space:]]*$|来源:[[:space:]]*$"; then
          echo "  [FAIL] 第 $line_num 行 Source Type 为空"
          doc_fail=$((doc_fail + 1))
        else
          echo "  [PASS] 第 $line_num 行有完整来源标记"
          doc_pass=$((doc_pass + 1))
        fi
      fi
    fi
  done <<< "$claim_lines"

  if [ $doc_fail -gt 0 ]; then
    FAIL_COUNT=$((FAIL_COUNT + doc_fail))
  else
    PASS_COUNT=$((PASS_COUNT + doc_pass))
  fi
}

# 检查 Source 引用是否指向存在的文档
check_source_reference() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # 查找所有 Source 引用（如 "Source: xxx.md" 或 "Source: commit xxx"）
  local source_refs
  source_refs=$(grep -oE "Source:[[:space:]]*[^\n]+" "$path" 2>/dev/null || true)

  if [ -z "$source_refs" ]; then
    return
  fi

  while read -r ref; do
    # 提取被引用的文档名（去除 commit hash 部分）
    local ref_content
    ref_content=$(echo "$ref" | sed 's/Source:[[:space:]]*//')

    # 检查是否引用 .md 文档
    local referenced_doc
    referenced_doc=$(echo "$ref_content" | grep -oE "[a-zA-Z0-9_-]+\.md" || true)
    if [ -n "$referenced_doc" ]; then
      if [ ! -f "$RELEASE_DIR/$referenced_doc" ] && [ ! -f "$REPO_ROOT/docs/$referenced_doc" ]; then
        echo "  [WARN] Source 引用不存在的文档：$referenced_doc (在 $doc 中)"
        WARN_COUNT=$((WARN_COUNT + 1))
      fi
    fi

    # 检查是否引用 commit hash（7-40 位 hex）
    local commit_ref
    commit_ref=$(echo "$ref_content" | grep -oE "[0-9a-f]{7,40}" || true)
    if [ -n "$commit_ref" ]; then
      if ! git -C "$REPO_ROOT" rev-parse --verify "$commit_ref" >/dev/null 2>&1; then
        echo "  [FAIL] Source 引用不存在的 commit：$commit_ref (在 $doc 中)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
      fi
    fi
  done <<< "$source_refs"
}

# ============================================================
# 主检查流程
# ============================================================
echo "=== G-02 Source Marker Check for v$VERSION ==="
echo ""

if [ ! -d "$RELEASE_DIR" ]; then
  echo "版本文档目录不存在：$RELEASE_DIR"
  exit 1
fi

# 重点检查文档类型
KEY_DOCS=(
  "RELEASE_NOTES.md"
  "RELEASE_GATE_CHECKLIST.md"
  "ALPHA_GATE_REPORT.md"
  "BETA_GATE_REPORT.md"
  "RC_GATE_REPORT.md"
  "GA_GATE_REPORT.md"
  "TEST_PLAN.md"
  "PERFORMANCE_REPORT.md"
  "ANALYSIS_REPORT.md"
  "MATURITY_ASSESSMENT.md"
)

echo "--- 重点文档检查 ---"
for doc in "${KEY_DOCS[@]}"; do
  if [ -f "$RELEASE_DIR/$doc" ]; then
    echo "检查: $doc"
    check_doc_source_marker "$doc"
    check_source_reference "$doc"
  fi
done

echo ""
echo "--- 全量文档扫描 ---"
ALL_DOCS=$(find "$RELEASE_DIR" -name "*.md" -type f 2>/dev/null || true)
for doc_path in $ALL_DOCS; do
  doc_name=$(basename "$doc_path")
  # 跳过已检查的文档
  skip=false
  for kd in "${KEY_DOCS[@]}"; do
    if [ "$doc_name" = "$kd" ]; then
      skip=true
      break
    fi
  done
  if [ "$skip" = true ]; then
    continue
  fi
  check_doc_source_marker "$doc_name"
done

# ============================================================
# 生成报告
# ============================================================
REPORT_FILE="$OUT_DIR/G02_SOURCE_MARKER_REPORT.md"

cat > "$REPORT_FILE" << EOF
# v$VERSION G-02 来源标记检查报告

> **检查日期**: $(date +%Y-%m-%d)
> **版本**: $VERSION
> **标准**: ADR-001 Truthfulness Framework G-02
> **检查工具**: check_g_02_source_marker.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | $PASS_COUNT |
| 失败 | $FAIL_COUNT |
| 警告 | $WARN_COUNT |

**总结**: $([ $FAIL_COUNT -gt 0 ] && echo "❌ 发现 $FAIL_COUNT 个 G-02 违规" || echo "✅ G-02 检查通过")

---

## G-02 合规要求

所有 Claim 必须包含:

```
Claim: "<具体声明内容>"
Source Type: [实测|SSOT引用|历史文档]
Source: <具体引用>
```

---

## 违规处理

EOF

if [ $FAIL_COUNT -gt 0 ]; then
  cat >> "$REPORT_FILE" << EOF
### 必须执行的修复

1. 为所有无 Source Type 标记的 Claim 补充标记
2. 确保 Source 引用真实存在的文档或 commit
3. Source Type 只允许: `实测`、`SSOT引用`、`历史文档`

EOF
else
  cat >> "$REPORT_FILE" << EOF
✅ 无 G-02 违规，所有 Claim 均有正确的来源标记

EOF
fi

cat >> "$REPORT_FILE" << EOF

---

*报告生成时间: $(date '+%Y-%m-%d %H:%M:%S')*
*检查工具版本: check_g_02_source_marker.sh v1.0.0*
*依据标准: ADR-001 Truthfulness Framework G-02*

EOF

echo ""
echo "Report: $REPORT_FILE"
echo "PASS=$PASS_COUNT, FAIL=$FAIL_COUNT, WARN=$WARN_COUNT"

if [ $FAIL_COUNT -gt 0 ]; then
  echo "❌ G-02 Source Marker check FAILED"
  exit 1
else
  echo "✅ G-02 Source Marker check PASSED"
  exit 0
fi
