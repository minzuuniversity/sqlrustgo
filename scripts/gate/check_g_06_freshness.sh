#!/usr/bin/env bash
#
# check_g_06_freshness.sh — G-06 Freshness 标记检查
#
# 用途: 检查文档 Claim 是否标注了数据年龄
#
# G-06 要求:
# 所有引用历史数据的 Claim 必须标注数据年龄:
#   Parser coverage: 47.16%
#   Freshness: v3.5.0 Alpha Gate Report (2026-05-28)
#   Current status: Unknown (未重新测量)
#
# 违规:
# - 历史数据（>30天）无 Freshness 标记
# - Freshness 标记日期早于实际测量日期

set -euo pipefail

VERSION="${1:-}"
OUT_DIR="${2:-}"

if [ -z "$VERSION" ] || [ -z "$OUT_DIR" ]; then
  echo "Usage: $0 <version> <out_dir>"
  exit 2
fi

# Freshness 阈值: 30天
FRESHNESS_THRESHOLD_DAYS=30

mkdir -p "$OUT_DIR"

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

RELEASE_DIR="docs/releases/$VERSION"

# ============================================================
# 辅助函数
# ============================================================

# 从 Freshness 标记中提取日期
extract_freshness_date() {
  local content="$1"
  # 支持格式: YYYY-MM-DD, YYYY/MM/DD, Mon DD YYYY, DD-Mon-YYYY
  echo "$content" | grep -oE "[0-9]{4}-[0-9]{2}-[0-9]{2}" | head -1
}

# 检查日期是否超过阈值
is_stale() {
  local freshness_date="$1"
  if [ -z "$freshness_date" ]; then
    return 1  # 无法判断，视为过期
  fi

  # 计算日期差异
  local freshness_ts
  freshness_ts=$(date -j -f "%Y-%m-%d" "$freshness_date" +%s 2>/dev/null || echo "0")
  local now_ts
  now_ts=$(date +%s)

  local age_days=$(( (now_ts - freshness_ts) / 86400 ))

  if [ $age_days -gt $FRESHNESS_THRESHOLD_DAYS ]; then
    return 0  # 过期
  else
    return 1  # 新鲜
  fi
}

# ============================================================
# 检查函数
# ============================================================

# 检查单个文档的 Freshness 标记
check_doc_freshness() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # 查找所有包含数值的声明行（潜在历史数据）
  local claim_lines
  claim_lines=$(grep -nE "(%|passed|failed|通过率|覆盖率|latency|throughput|exec time|coverage|pass rate)" "$path" 2>/dev/null || true)

  if [ -z "$claim_lines" ]; then
    return
  fi

  local doc_fail=0
  local doc_pass=0

  while IFS=: read -r line_num content; do
    # 跳过注释行、代码块、表格行
    if echo "$content" | grep -qE "^#|```|`|^[[:space:]]*\|"; then
      continue
    fi

    # 检查是否有 Freshness 标记（在同一行或后续几行）
    local has_freshness=false
    local freshness_value=""

    # 先在同一行查找
    if echo "$content" | grep -qE "Freshness:|新鲜度:|数据年龄:"; then
      has_freshness=true
      freshness_value=$(echo "$content" | grep -oE "Freshness:.*|新鲜度:.*|数据年龄:.*" | head -1)
    else
      # 在后续行查找（lookahead 3行）
      local following_lines
      following_lines=$(sed -n "${line_num},$((line_num + 3))p" "$path" 2>/dev/null || true)
      if echo "$following_lines" | grep -qE "Freshness:|新鲜度:|数据年龄:"; then
        has_freshness=true
        freshness_value=$(echo "$following_lines" | grep -oE "Freshness:.*|新鲜度:.*|数据年龄:.*" | head -1)
      fi
    fi

    # 提取日期
    local freshness_date=""
    if [ -n "$freshness_value" ]; then
      freshness_date=$(extract_freshness_date "$freshness_value")
    fi

    # 判断是否为历史数据（包含具体数值且非"当前"描述）
    local is_historical=false
    if echo "$content" | grep -qE "[0-9]+\.[0-9]+%|[0-9]+\.[0-9]+ |[0-9]+ passed|[0-9]+ failed"; then
      # 排除"实时测量"、"当前"、"now"等
      if ! echo "$content" | grep -qE "current|now|实时|当前|measured at|date:"; then
        is_historical=true
      fi
    fi

    if [ "$is_historical" = true ]; then
      if [ "$has_freshness" = false ]; then
        echo "  [FAIL] 第 $line_num 行历史数据无 Freshness 标记: $(echo "$content" | cut -c1-60)"
        doc_fail=$((doc_fail + 1))
      elif [ -n "$freshness_date" ] && is_stale "$freshness_date"; then
        local age_days=$(( ($(date +%s) - $(date -j -f "%Y-%m-%d" "$freshness_date" +%s 2>/dev/null || echo "0")) / 86400 ))
        echo "  [WARN] 第 $line_num 行 Freshness 过期 (${age_days}天前): $(echo "$content" | cut -c1-60)"
        WARN_COUNT=$((WARN_COUNT + 1))
      else
        echo "  [PASS] 第 $line_num 行有有效 Freshness 标记"
        doc_pass=$((doc_pass + 1))
      fi
    fi
  done <<< "$claim_lines"

  if [ $doc_fail -gt 0 ]; then
    FAIL_COUNT=$((FAIL_COUNT + doc_fail))
  else
    PASS_COUNT=$((PASS_COUNT + doc_pass))
  fi
}

# ============================================================
# 主检查流程
# ============================================================
echo "=== G-06 Freshness Check for v$VERSION ==="
echo "Freshness 阈值: ${FRESHNESS_THRESHOLD_DAYS}天"
echo ""

if [ ! -d "$RELEASE_DIR" ]; then
  echo "版本文档目录不存在：$RELEASE_DIR"
  exit 1
fi

# 重点检查文档类型
KEY_DOCS=(
  "RELEASE_NOTES.md"
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
    check_doc_freshness "$doc"
  fi
done

echo ""
echo "--- 全量文档扫描 ---"
ALL_DOCS=$(find "$RELEASE_DIR" -name "*.md" -type f 2>/dev/null || true)
for doc_path in $ALL_DOCS; do
  doc_name=$(basename "$doc_path")
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
  check_doc_freshness "$doc_name"
done

# ============================================================
# 生成报告
# ============================================================
REPORT_FILE="$OUT_DIR/G06_FRESHNESS_REPORT.md"

cat > "$REPORT_FILE" << EOF
# v$VERSION G-06 Freshness 检查报告

> **检查日期**: $(date +%Y-%m-%d)
> **版本**: $VERSION
> **标准**: ADR-001 Truthfulness Framework G-06
> **Freshness 阈值**: ${FRESHNESS_THRESHOLD_DAYS}天
> **检查工具**: check_g_06_freshness.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | $PASS_COUNT |
| 失败 | $FAIL_COUNT |
| 警告 | $WARN_COUNT |

**总结**: $([ $FAIL_COUNT -gt 0 ] && echo "❌ 发现 $FAIL_COUNT 个 G-06 违规" || echo "✅ G-06 检查通过")

---

## G-06 合规要求

所有引用历史数据（>30天）的 Claim 必须标注:

```
Claim: "<具体数值声明>"
Freshness: <来源文档> (YYYY-MM-DD)
Current status: [Unknown/Stale/Verified]
```

---

## 违规处理

EOF

if [ $FAIL_COUNT -gt 0 ]; then
  cat >> "$REPORT_FILE" << EOF
### 必须执行的修复

1. 为所有无 Freshness 标记的历史数据补充标记
2. 过期数据（>30天）必须标注 "Current status: Unknown (未重新测量)"
3. 定期更新关键指标数据

EOF
else
  cat >> "$REPORT_FILE" << EOF
✅ 无 G-06 违规，所有历史数据均有 Freshness 标记

EOF
fi

cat >> "$REPORT_FILE" << EOF

---

*报告生成时间: $(date '+%Y-%m-%d %H:%M:%S')*
*检查工具版本: check_g_06_freshness.sh v1.0.0*
*依据标准: ADR-001 Truthfulness Framework G-06*

EOF

echo ""
echo "Report: $REPORT_FILE"
echo "PASS=$PASS_COUNT, FAIL=$FAIL_COUNT, WARN=$WARN_COUNT"

if [ $FAIL_COUNT -gt 0 ]; then
  echo "❌ G-06 Freshness check FAILED"
  exit 1
else
  echo "✅ G-06 Freshness check PASSED"
  exit 0
fi
