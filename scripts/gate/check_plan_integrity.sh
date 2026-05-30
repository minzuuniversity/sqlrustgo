#!/usr/bin/env bash
#
# check_plan_integrity.sh — 开发/测试计划完整性检查
#
# 用途: 检查计划文档是否被非法重写（伪造历史记录）
#
# Truthfulness 原则：
# - 开发计划、测试计划是历史记录，禁止重写
# - 计划文档只追加、不改写
# - 禁止为通过门禁而修改原始计划状态
#
# 常见违规行为：
# - 将 VERSION_PLAN.md 从 "Alpha 阶段" 重写为 "GA Final"
# - 将 TEST_PLAN.md 从 "测试进行中" 重写为 "GA Final Report"
# - 将 DEVELOPMENT_PLAN.md 改为 "最终报告" 以掩盖未完成的工作
# - 修改计划状态（Alpha/Beta/RC/GA）以通过门禁
#
# 合规做法：
# - 状态变更是执行结果，在门禁检查后记录为新文档
# - 原始计划文档只追加版本，不改写
# - 测试结果以命令输出为准，不在计划文档中伪造

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

PASS_ITEMS=()
FAIL_ITEMS=()
WARN_ITEMS=()

add_pass() { PASS_ITEMS+=("$1"); PASS_COUNT=$((PASS_COUNT + 1)); }
add_fail() { FAIL_ITEMS+=("$1"); FAIL_COUNT=$((FAIL_COUNT + 1)); }
add_warn() { WARN_ITEMS+=("$1"); WARN_COUNT=$((WARN_COUNT + 1)); }

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

RELEASE_DIR="docs/releases/$VERSION"

# ============================================================
# 检查 1: 计划文档是否被大幅重写
# ============================================================
# 策略: 对比计划文档在历史 commit 中的行数变化
# 如果某次 commit 使计划文档行数大幅减少（>30%），可能是重写

check_plan_rewrite() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    add_warn "计划文档不存在（跳过重写检查）：$doc"
    return
  fi

  # 获取该文件在历史中最大的行数变化
  local max_lines
  max_lines=$(git log --follow --format= --"$path" 2>/dev/null | wc -l | tr -d ' ' || echo "0")
  local current_lines
  current_lines=$(wc -l < "$path" | tr -d ' ')

  # 如果当前行数 < 历史最大行数的 30%，可能存在重写
  if [ -n "$max_lines" ] && [ -n "$current_lines" ] && [ "$max_lines" -gt 0 ] 2>/dev/null; then
    local threshold
    threshold=$((max_lines * 30 / 100))
    if [ "$current_lines" -lt "$threshold" ]; then
      add_fail "计划文档疑似被重写: $doc (当前 $current_lines 行，历史最大 $max_lines 行，阈值 $threshold)"
    else
      add_pass "计划文档行数正常: $doc ($current_lines 行)"
    fi
  else
    add_warn "无法获取历史记录（无历史数据）：$doc"
  fi
}

# ============================================================
# 检查 2: 计划文档中是否存在可疑状态标识
# ============================================================
# 策略: 检查 "GA Final"、"GA APPROVED"、"GA ✅" 等不应该出现在计划文档中的状态

check_suspicious_status() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # 可疑模式: 计划文档中出现 GA Final/GA APPROVED 等
  local suspicious_patterns=(
    "GA.*Final.*Report"
    "GA.*APPROVED"
    "GA.*✅"
    "Final Report"
    "GA complete"
  )

  for pattern in "${suspicious_patterns[@]}"; do
    if grep -qE "$pattern" "$path" 2>/dev/null; then
      # 检查这是否是标题行（计划文档的正常状态）还是内容行（可能是伪造）
      local lines
      lines=$(grep -nE "$pattern" "$path" 2>/dev/null || true)
      if [ -n "$lines" ]; then
        # 如果出现在文档头部（< 10 行），可能是状态标注（可接受）
        # 如果出现在正文，可能是伪造
        local first_match_line
        first_match_line=$(echo "$lines" | head -1 | cut -d: -f1)
        if [ "$first_match_line" -gt 5 ]; then
          add_fail "计划文档疑似伪造状态: $doc 中发现 '$pattern' 在第 $first_match_line 行"
        else
          add_pass "计划文档状态标注正常: $doc (标题行)"
        fi
      fi
    fi
  done

  # 检查 VERSION_PLAN.md 是否显示 "GA" 状态（计划文档不应该显示 GA）
  if [ "$doc" = "VERSION_PLAN.md" ] || [ "$doc" = "DEVELOPMENT_PLAN.md" ] || [ "$doc" = "TEST_PLAN.md" ]; then
    if grep -qE "^\>.*GA|状态.*GA|GA.*✅|GA APPROVED" "$path" 2>/dev/null; then
      add_fail "计划文档显示 GA 状态（疑似伪造）: $doc"
    fi
  fi
}

# ============================================================
# 检查 3: 计划文档修改时间和提交信息
# ============================================================
# 策略: 检查计划文档的最近修改是否在门禁期间

check_recent_modification() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # 获取最近一次修改该文件的 commit
  local last_commit_date
  last_commit_date=$(git log -1 --format="%ai" --"$path" 2>/dev/null || echo "")
  local last_commit_msg
  last_commit_msg=$(git log -1 --format="%s" --"$path" 2>/dev/null || echo "")

  if [ -n "$last_commit_date" ]; then
    # 提取日期部分
    local commit_date
    commit_date=$(echo "$last_commit_date" | cut -d' ' -f1)

    # 检查提交信息中是否有可疑关键词
    if echo "$last_commit_msg" | grep -qiE "GA|finalize|final|report"; then
      add_warn "计划文档在门禁期间被修改: $doc (commit: $last_commit_msg)"
    fi
  fi
}

# ============================================================
# 检查 4: 测试计划是否有实际执行证据
# ============================================================
# 策略: 检查 TEST_PLAN.md 中的测试结果是否有对应命令输出

check_test_result_evidence() {
  local doc="TEST_PLAN.md"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    add_warn "测试计划文档不存在：$doc"
    return
  fi

  # 检查 TEST_PLAN.md 中是否有测试结果
  if grep -qE "PASS|✅|失败|通过" "$path" 2>/dev/null; then
    # 检查是否有对应的输出文件或 evidence
    local has_evidence=false
    for evidence_file in "$OUT_DIR"/*.txt "$OUT_DIR"/*.log "$OUT_DIR"/*test*; do
      if [ -f "$evidence_file" ]; then
        has_evidence=true
        break
      fi
    done

    if [ "$has_evidence" = false ]; then
      add_warn "TEST_PLAN.md 中有测试结果但无对应执行证据文件"
    else
      add_pass "TEST_PLAN.md 测试结果有执行证据"
    fi
  else
    add_pass "TEST_PLAN.md 无伪造测试结果"
  fi
}

# ============================================================
# 检查 5: 开发计划中的任务状态是否与实际 commit 对应
# ============================================================
check_task_actual_delivery() {
  local doc="DEVELOPMENT_PLAN.md"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    add_warn "开发计划文档不存在：$doc"
    return
  fi

  # 检查是否有 "Done" 状态的任务
  if grep -qE "✅.*DONE|状态.*完成|✅ PASS" "$path" 2>/dev/null; then
    # 检查这些完成的任务是否对应实际 commit
    local todo_items
    todo_items=$(grep -E "^\|.*\|.*\|" "$path" 2>/dev/null | grep -cE "✅.*DONE|完成|GA" || echo "0")
    local commit_count
    commit_count=$(git log --oneline -20 -- "docs/releases/$VERSION/" 2>/dev/null | wc -l || echo "0")

    if [ "$todo_items" -gt 0 ] && [ "$commit_count" -lt "$todo_items" ]; then
      add_fail "开发计划显示 $todo_items 个任务完成，但近 20 个 commit 中只有 $commit_count 个文档更新"
    else
      add_pass "开发计划任务状态与实际 commit 数量基本一致"
    fi
  else
    add_pass "开发计划无异常完成状态"
  fi
}

# ============================================================
# 主检查流程
# ============================================================
echo "=== Plan Integrity Check for v$VERSION ==="

# 需要检查的计划文档
PLAN_DOCS=(
  "VERSION_PLAN.md"
  "DEVELOPMENT_PLAN.md"
  "TEST_PLAN.md"
)

for doc in "${PLAN_DOCS[@]}"; do
  echo "Checking $doc..."
  check_plan_rewrite "$doc"
  check_suspicious_status "$doc"
  check_recent_modification "$doc"
done

check_test_result_evidence
check_task_actual_delivery

# ============================================================
# 生成报告
# ============================================================
REPORT_FILE="$OUT_DIR/PLAN_INTEGRITY_REPORT.md"

cat > "$REPORT_FILE" << EOF
# v$VERSION 计划完整性检查报告

> **检查日期**: $(date +%Y-%m-%d)
> **版本**: $VERSION
> **Auditor**: Hermes Agent

---

## 检查结果

| 检查项 | 结果 |
|--------|------|
| 计划文档重写检查 | $([ $FAIL_COUNT -gt 0 ] && echo "❌ FAIL" || echo "✅ PASS") |
| 可疑状态标识检查 | $([ $FAIL_COUNT -gt 0 ] && echo "❌ FAIL" || echo "✅ PASS") |
| 文档修改时间检查 | $([ $WARN_COUNT -gt 0 ] && echo "⚠️  WARN" || echo "✅ PASS") |
| 测试结果证据检查 | $([ $WARN_COUNT -gt 0 ] && echo "⚠️  WARN" || echo "✅ PASS") |
| 任务实际交付检查 | $([ $FAIL_COUNT -gt 0 ] && echo "❌ FAIL" || echo "✅ PASS") |

**总结**: $PASS_COUNT 通过, $WARN_COUNT 警告, $FAIL_COUNT 失败

---

## 失败项详情

EOF

if [ ${#FAIL_ITEMS[@]} -eq 0 ]; then
  cat >> "$REPORT_FILE" << EOF
✅ 无失败项
EOF
else
  for item in "${FAIL_ITEMS[@]}"; do
    echo "- ❌ $item" >> "$REPORT_FILE"
  done
fi

cat >> "$REPORT_FILE" << EOF

---

## 警告项详情

EOF

if [ ${#WARN_ITEMS[@]} -eq 0 ]; then
  cat >> "$REPORT_FILE" << EOF
✅ 无警告项
EOF
else
  for item in "${WARN_ITEMS[@]}"; do
    echo "- ⚠️ $item" >> "$REPORT_FILE"
  done
fi

cat >> "$REPORT_FILE" << EOF

---

## 通过项详情

EOF

if [ ${#PASS_ITEMS[@]} -eq 0 ]; then
  cat >> "$REPORT_FILE" << EOF
无
EOF
else
  for item in "${PASS_ITEMS[@]}"; do
    echo "- ✅ $item" >> "$REPORT_FILE"
  done
fi

cat >> "$REPORT_FILE" << EOF

---

## Truthfulness 原则说明

本检查旨在防止以下违规行为：

1. **计划不可伪造**: 开发/测试计划是历史记录，禁止重写以通过门禁
2. **原始记录保留**: 计划文档只追加、不改写
3. **禁止形式主义**: 门禁是质量关卡，不是文档美化关卡

**正确做法**：
- 状态变更是执行结果 → 在门禁检查后记录为新文档（如 VERSION_PLAN_BETA_REPORT.md）
- 测试结果是命令输出的记录 → 不在计划文档中伪造

**违规示例**：
- ❌ 将 VERSION_PLAN.md 从 "Alpha 阶段" 重写为 "GA Final"
- ❌ 在 TEST_PLAN.md 写入 "93 tests PASS" 但未执行 cargo test

---

*报告生成时间: $(date '+%Y-%m-%d %H:%M:%S')*
EOF

echo ""
echo "Report: $REPORT_FILE"
echo "PASS=$PASS_COUNT, WARN=$WARN_COUNT, FAIL=$FAIL_COUNT"

if [ $FAIL_COUNT -gt 0 ]; then
  echo ""
  echo "❌ Plan integrity check FAILED"
  exit 1
else
  echo ""
  echo "✅ Plan integrity check PASSED"
  exit 0
fi