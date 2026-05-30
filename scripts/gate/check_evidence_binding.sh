#!/usr/bin/env bash
#
# check_evidence_binding.sh — 证据绑定检查脚本
#
# 用途: 检查文档中的声明是否绑定了有效的机器可验证证据
#
# Anti-Fabrication Policy 要求:
# - 每个 PASS/FAIL 声明必须绑定 CI run ID + log hash
# - 每个"完成"声明必须绑定 commit hash / PR 合并证明
# - 每个门禁结果必须绑定 gate_policy_eval_id
# - 无证据的声明必须标记为 UNVERIFIED CLAIM
#
# 违规类型:
# - Type A: 虚构执行（无 CI 证据声明测试通过）
# - Type B: 伪门禁（无 gate engine 输出声明门禁通过）
# - Type C: 伪证据（引用不存在的 CI run / log hash）
# - Type D: 伪任务完成（无 commit/PR 声明任务完成）

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
UNVERIFIED_CLAIMS=()

add_pass() { PASS_ITEMS+=("$1"); PASS_COUNT=$((PASS_COUNT + 1)); }
add_fail() { FAIL_ITEMS+=("$1"); FAIL_COUNT=$((FAIL_COUNT + 1)); }
add_warn() { WARN_ITEMS+=("$1"); WARN_COUNT=$((WARN_COUNT + 1)); }
add_unverified() { UNVERIFIED_CLAIMS+=("$1"); }

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

RELEASE_DIR="docs/releases/$VERSION"

# ============================================================
# 检查函数
# ============================================================

# 检查 PASS/FAIL 声明是否有 CI 证据
check_pass_fail_evidence() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    add_warn "文档不存在（跳过检查）：$doc"
    return
  fi

  # 查找所有 PASS/FAIL 声明
  local lines_with_pass_fail
  lines_with_pass_fail=$(grep -nE "PASS|FAIL|通过|失败|完成|done|completed|成功|失败" "$path" 2>/dev/null || true)

  if [ -z "$lines_with_pass_fail" ]; then
    add_pass "无状态声明（无需证据检查）：$doc"
    return
  fi

  # 对每个声明检查是否有证据绑定
  while IFS=: read -r line_num content; do
    # 跳过注释行和代码块
    if echo "$content" | grep -qE "^#|```|`"; then
      continue
    fi

    # 检查是否有 CI run ID / log hash 绑定
    local has_ci_ref=false
    local has_gate_ref=false

    # 检查是否有 CI run ID 格式（如 #19382, run_20260530_001）
    if echo "$content" | grep -qE "(run_|#|ci_run|CI_RUN|log_hash|log-hash)"; then
      has_ci_ref=true
    fi

    # 检查是否有 gate engine 输出引用（如 policy_eval_id, gate_output）
    if echo "$content" | grep -qE "(policy_eval|gate_output|gate_policy|GATE_)"; then
      has_gate_ref=true
    fi

    # 检查是否有 commit hash
    local has_commit_ref=false
    if echo "$content" | grep -qE "[0-9a-f]{7,40}"; then
      has_commit_ref=true
    fi

    # 判断声明类型并验证
    if echo "$content" | grep -qE "PASS|通过|成功|completed|done|PASS"; then
      if [ "$has_ci_ref" = false ] && [ "$has_gate_ref" = false ] && [ "$has_commit_ref" = false ]; then
        add_fail "Type A/B 违规：第 $line_num 行声明无 CI/gate/commit 证据: $(echo "$content" | cut -c1-60)"
        add_unverified "Line $line_num: $content"
      else
        add_pass "状态声明有证据绑定：第 $line_num 行"
      fi
    fi

    if echo "$content" | grep -qE "FAIL|失败|failed"; then
      if [ "$has_ci_ref" = false ] && [ "$has_gate_ref" = false ]; then
        add_warn "警告：第 $line_num 行 FAIL 声明可能无证据"
      fi
    fi
  done <<< "$lines_with_pass_fail"
}

# 检查是否存在伪证据（引用不存在的 CI run）
check_fabricated_evidence() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # 查找 CI run ID 引用（如 run_20260530_001 或 #19382）
  local ci_refs
  ci_refs=$(grep -oE "run_[0-9]{8}_[0-9]{3}|#[0-9]+|ci_run[_-]id:?[ ]*[0-9]+" "$path" 2>/dev/null || true)

  if [ -z "$ci_refs" ]; then
    return
  fi

  # 检查引用的 CI run 是否存在于 CI 系统或作为 artifact
  # 注意：这里只能检查格式是否正确，无法访问外部 CI 系统
  while read -r ref; do
    # 简单格式检查（实际应该查询 CI 系统）
    if ! echo "$ref" | grep -qE "^run_[0-9]{8}_[0-9]{3}$|^#[0-9]+$"; then
      add_warn "可疑 CI 引用格式：$ref 在 $doc"
    fi
  done <<< "$ci_refs"
}

# 检查门禁报告中的 gate engine 输出
check_gate_output_evidence() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # 如果是门禁相关文档，检查是否有 gate engine 输出
  if echo "$doc" | grep -qE "GATE|GATE_CHECK|RELEASE_GATE"; then
    # 检查是否有 gate_policy_eval_id
    if ! grep -qE "policy_eval_id|gate_policy_eval|GATE_POLICY" "$path" 2>/dev/null; then
      add_fail "Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：$doc"
    fi

    # 检查是否有 PASS/FAIL 但无 gate engine 输出
    if grep -qE "GA.*PASS|Gate.*PASS|门禁.*通过" "$path" 2>/dev/null; then
      if ! grep -qE "gate_policy_eval|policy_eval_id" "$path" 2>/dev/null; then
        add_fail "Type B 违规：门禁声明 PASS 但无 gate engine 输出：$doc"
      fi
    fi
  fi
}

# 检查计划文档状态标识
check_plan_status_fabrication() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # VERSION_PLAN / DEVELOPMENT_PLAN / TEST_PLAN 不应出现 GA Final
  if echo "$doc" | grep -qE "VERSION_PLAN|DEVELOPMENT_PLAN|TEST_PLAN"; then
    if grep -qE "GA.*Final|GA APPROVED|GA.*✅|GA complete" "$path" 2>/dev/null; then
      # 检查是否在标题（前 10 行）
      local ga_final_line
      ga_final_line=$(grep -nE "GA.*Final|GA APPROVED|GA.*✅" "$path" 2>/dev/null | head -1 | cut -d: -f1)
      if [ -n "$ga_final_line" ] && [ "$ga_final_line" -le 10 ]; then
        # 标题行，可能是状态标注（可接受）
        add_pass "计划文档状态标注在标题行（可接受）：$doc"
      else
        add_fail "Type D 违规：计划文档显示 GA 状态（疑似伪造）：$doc 第 $ga_final_line 行"
      fi
    fi
  fi
}

# 检查 provenance 元数据
check_provenance_metadata() {
  local doc="$1"
  local path="$RELEASE_DIR/$doc"

  if [ ! -f "$path" ]; then
    return
  fi

  # 检查是否有 provenance 元数据
  if ! grep -qE "provenance:|generated_by:|generated_at:" "$path" 2>/dev/null; then
    # 对于 AI 生成的文档，这是警告
    add_warn "文档可能缺少 provenance 元数据：$doc"
  else
    add_pass "文档有 provenance 元数据：$doc"
  fi
}

# ============================================================
# 主检查流程
# ============================================================
echo "=== Evidence Binding Check for v$VERSION ==="

# 获取所有需要检查的文档
if [ -d "$RELEASE_DIR" ]; then
  ALL_DOCS=$(find "$RELEASE_DIR" -name "*.md" -type f 2>/dev/null || true)
else
  echo "版本文档目录不存在：$RELEASE_DIR"
  exit 1
fi

if [ -z "$ALL_DOCS" ]; then
  echo "未找到任何文档"
  exit 1
fi

for doc_path in $ALL_DOCS; do
  doc_name=$(basename "$doc_path")
  echo "检查: $doc_name..."

  check_pass_fail_evidence "$doc_name"
  check_fabricated_evidence "$doc_name"
  check_gate_output_evidence "$doc_name"
  check_plan_status_fabrication "$doc_name"
  check_provenance_metadata "$doc_name"
done

# ============================================================
# 生成报告
# ============================================================
REPORT_FILE="$OUT_DIR/EVIDENCE_BINDING_REPORT.md"

cat > "$REPORT_FILE" << EOF
# v$VERSION 证据绑定检查报告

> **检查日期**: $(date +%Y-%m-%d)
> **版本**: $VERSION
> **Auditor**: Hermes Agent (Anti-Fabrication Policy v1.0)
> **检查工具**: check_evidence_binding.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | $PASS_COUNT |
| 警告 | $WARN_COUNT |
| 失败（违规） | $FAIL_COUNT |
| 未验证声明 | ${#UNVERIFIED_CLAIMS[@]} |

**总结**: $([ $FAIL_COUNT -gt 0 ] && echo "❌ 发现 $FAIL_COUNT 个违规（Type A/B/C/D）" || echo "✅ 无违规发现")

---

## 违规详情（按类型分类）

### Type A: 虚构执行（Execution Fabrication）

无 CI 证据声明"测试通过 / 编译成功"

EOF

if [ $FAIL_COUNT -gt 0 ]; then
  for item in "${FAIL_ITEMS[@]}"; do
    echo "- ❌ $item" >> "$REPORT_FILE"
  done
else
  echo "✅ 无 Type A/B 违规" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

---

### 警告项（需要人工复核）

EOF

if [ $WARN_COUNT -gt 0 ]; then
  for item in "${WARN_ITEMS[@]}"; do
    echo "- ⚠️ $item" >> "$REPORT_FILE"
  done
else
  echo "✅ 无警告" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

---

### 通过项

EOF

if [ $PASS_COUNT -gt 0 ]; then
  for item in "${PASS_ITEMS[@]}"; do
    echo "- ✅ $item" >> "$REPORT_FILE"
  done
else
  echo "（无）" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

---

## 未验证声明（UNVERIFIED CLAIMS）

EOF

if [ ${#UNVERIFIED_CLAIMS[@]} -gt 0 ]; then
  cat >> "$REPORT_FILE" << EOF
以下声明**无证据支撑**，不得用于门禁判断：

| 文档 | 行 | 声明内容 |
|------|-----|----------|
EOF
  for claim in "${UNVERIFIED_CLAIMS[@]}"; do
    echo "| - | $(echo "$claim" | cut -d: -f2-) |" >> "$REPORT_FILE"
  done
  echo "" >> "$REPORT_FILE"
  echo "**规则**: UnverifiedDoc 不得用于门禁判断" >> "$REPORT_FILE"
else
  echo "✅ 无未验证声明" >> "$REPORT_FILE"
fi

cat >> "$REPORT_FILE" << EOF

---

## Anti-Fabrication Policy 合规状态

| 要求 | 状态 |
|------|------|
| PASS/FAIL 声明绑定 CI 证据 | $([ $FAIL_COUNT -gt 0 ] && echo "❌ 违规" || echo "✅ 合规") |
| 门禁结果绑定 gate_policy_eval_id | $([ $FAIL_COUNT -gt 0 ] && echo "❌ 违规" || echo "✅ 合规") |
| 计划文档无 GA Final 伪造 | $([ $FAIL_COUNT -gt 0 ] && echo "❌ 违规" || echo "✅ 合规") |
| provenance 元数据存在 | $([ $WARN_COUNT -gt 0 ] && echo "⚠️  部分缺失" || echo "✅ 合规") |

---

## 后续行动

EOF

if [ $FAIL_COUNT -gt 0 ]; then
  cat >> "$REPORT_FILE" << EOF
### 必须执行的修复

1. **立即停止**当前门禁流程
2. **回退**到上一个 VerifiedDoc 状态
3. **补充**真实证据（CI run ID + log hash）
4. **重新**执行门禁检查

### 问责记录

- 违规次数: $FAIL_COUNT
- 违规类型: Type A（虚构执行）/ Type B（伪门禁）/ Type D（伪任务完成）
- 处理方式: 触发 Anti-Fabrication Policy 问责机制

EOF
else
  cat >> "$REPORT_FILE" << EOF
✅ 无需修复，所有检查通过

### 合规确认

- 所有 PASS/FAIL 声明有 CI 证据绑定
- 所有门禁结果有 gate_policy_eval_id
- 无计划文档状态伪造
- provenance 元数据存在

EOF
fi

cat >> "$REPORT_FILE" << EOF

---

*报告生成时间: $(date '+%Y-%m-%d %H:%M:%S')*
*检查工具版本: check_evidence_binding.sh v1.0.0*
*依据政策: Anti-Fabrication Policy v1.0.0*

---

## 参考：违规类型定义

| 类型 | 定义 | 严重程度 |
|------|------|----------|
| **Type A** | 虚构执行：AI 声称测试通过但无 CI 日志支撑 | P0 |
| **Type B** | 伪门禁：AI 生成门禁通过但无 gate engine 输出 | P0 |
| **Type C** | 伪证据：AI 引用不存在的 CI run / log hash | P1 |
| **Type D** | 伪任务完成：AI 标记任务完成但代码未合并 | P1 |
EOF

echo ""
echo "Report: $REPORT_FILE"
echo "PASS=$PASS_COUNT, WARN=$WARN_COUNT, FAIL=$FAIL_COUNT, UNVERIFIED=${#UNVERIFIED_CLAIMS[@]}"

if [ $FAIL_COUNT -gt 0 ]; then
  echo ""
  echo "❌ Evidence binding check FAILED - 发现 $FAIL_COUNT 个违规"
  exit 1
else
  echo ""
  echo "✅ Evidence binding check PASSED"
  exit 0
fi