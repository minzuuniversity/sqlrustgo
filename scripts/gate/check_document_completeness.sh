#!/usr/bin/env bash
#
# check_document_completeness.sh — DOCUMENT_COMPLETENESS_CHECK 强制执行
#
# 用途: 强制执行 docs/governance/DOCUMENT_COMPLETENESS_CHECK.md 定义的
#       11 个 mandatory 文档 (QUICK_START/FEATURE_MATRIX/RELEASE_NOTES/
#       DEPLOYMENT_GUIDE/MIGRATION_GUIDE/INSTALL.md 等) 的存在性 + 大小
#       阈值 + 链接完整性检查.
#
# 跟 check_docs_consistency.sh 区别:
#   - check_docs_consistency.sh: VERSION_HISTORY/CHANGELOG/版本号一致性
#   - check_document_completeness.sh (本): mandatory 文档存在性 + 大小 +
#     内部链接 + non-empty
#
# 退出码:
#   0 — 全部 mandatory 文档存在且通过
#   1 — 缺失文档或大小不达标
#   2 — 用法错误

set -euo pipefail

VERSION="${1:-v3.8.0}"
OUT_DIR="${2:-artifacts/gate/${VERSION}/document_completeness}"

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

if [ -z "$VERSION" ]; then
  echo "Usage: $0 <version> [out_dir]"
  exit 2
fi

mkdir -p "$OUT_DIR"

# Mandatory docs per docs/governance/DOCUMENT_COMPLETENESS_CHECK.md
# Format: path:min_size_bytes
MANDATORY_DOCS=(
  "docs/releases/$VERSION/QUICK_START.md:5000"
  "docs/releases/$VERSION/FEATURE_MATRIX.md:15000"
  "docs/releases/$VERSION/RELEASE_NOTES.md:10000"
  "docs/releases/$VERSION/DEPLOYMENT_GUIDE.md:15000"
  "docs/releases/$VERSION/MIGRATION_GUIDE.md:10000"
  "docs/releases/$VERSION/INSTALL.md:5000"
  "docs/releases/$VERSION/CHANGELOG.md:1000"
  "docs/releases/$VERSION/EVALUATION_REPORT.md:1000"
)

# Optional docs (WARN if missing)
OPTIONAL_DOCS=(
  "docs/releases/$VERSION/oo/README.md"
  "docs/releases/$VERSION/oo/architecture/ARCHITECTURE_${VERSION}.md"
  "docs/releases/$VERSION/oo/user-guide/USER_MANUAL.md"
  "docs/releases/$VERSION/oo/reports/PERFORMANCE_ANALYSIS.md"
  "docs/releases/$VERSION/oo/reports/SQL92_COMPLIANCE.md"
  "docs/releases/$VERSION/TEST_PLAN.md"
  "docs/releases/$VERSION/TEST_MANUAL.md"
  "docs/releases/$VERSION/DOCUMENT_AUDIT.md"
  "docs/releases/$VERSION/SECURITY_ANALYSIS.md"
  "docs/releases/$VERSION/PERFORMANCE_TARGETS.md"
  "docs/releases/$VERSION/API_DOCUMENTATION.md"
  "docs/releases/$VERSION/DEVELOPMENT_GUIDE.md"
)

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

PASS_ITEMS=()
FAIL_ITEMS=()
WARN_ITEMS=()

add_pass() { PASS_ITEMS+=("$1"); PASS_COUNT=$((PASS_COUNT + 1)); }
add_fail() { FAIL_ITEMS+=("$1"); FAIL_COUNT=$((FAIL_COUNT + 1)); }
add_warn() { WARN_ITEMS+=("$1"); WARN_COUNT=$((WARN_COUNT + 1)); }

REPORT_FILE="$OUT_DIR/DOCUMENT_COMPLETENESS_REPORT.md"

echo "=================================================="
echo "Document Completeness Check"
echo "Version:  $VERSION"
echo "Out dir:  $OUT_DIR"
echo "=================================================="
echo ""

# ============================================================
# Check mandatory docs
# ============================================================
echo "=== Mandatory Documents ==="
for entry in "${MANDATORY_DOCS[@]}"; do
  doc="${entry%:*}"
  min_size="${entry#*:}"
  if [ ! -f "$doc" ]; then
    add_fail "MISSING: $doc (mandatory per DOC_CHECK_CORRECTION_RULES)"
    continue
  fi
  size=$(stat -c%s "$doc" 2>/dev/null || wc -c < "$doc")
  if [ "$size" -lt "$min_size" ]; then
    add_fail "TOO_SMALL: $doc (${size} bytes < ${min_size} bytes required)"
    continue
  fi
  # Check the file is not empty of content (e.g. all placeholder text)
  if grep -qE "^# (TODO|TBD|placeholder|Coming soon|待补充)$" "$doc" 2>/dev/null; then
    add_fail "PLACEHOLDER: $doc contains only placeholder heading"
    continue
  fi
  add_pass "OK: $doc (${size} bytes)"
done
echo ""

# ============================================================
# Check optional docs (WARN if missing)
# ============================================================
echo "=== Optional Documents ==="
for doc in "${OPTIONAL_DOCS[@]}"; do
  if [ ! -f "$doc" ]; then
    add_warn "OPTIONAL_MISSING: $doc"
  else
    add_pass "OK: $doc"
  fi
done
echo ""

# ============================================================
# Internal link integrity (sample)
# ============================================================
echo "=== Internal Link Integrity (sample) ==="
LINK_BROKEN=0
LINK_BROKEN_DETAILS=""
CHECKED_LINKS=0
# Sample: check first 20 internal links across mandatory docs
for entry in "${MANDATORY_DOCS[@]}"; do
  doc="${entry%:*}"
  if [ ! -f "$doc" ]; then continue; fi
  # Extract relative markdown links: [text](path) where path doesn't start with http
  while IFS= read -r link_path; do
    if [ -z "$link_path" ]; then continue; fi
    CHECKED_LINKS=$((CHECKED_LINKS + 1))
    if [ "$CHECKED_LINKS" -gt 20 ]; then break; fi
    # Resolve relative to doc directory
    doc_dir=$(dirname "$doc")
    # Strip leading ./ and #
    target="$doc_dir/$link_path"
    target="${target%#*}"  # remove #anchor
    if [ ! -e "$target" ] && [ ! -e "${target}.md" ]; then
      LINK_BROKEN=$((LINK_BROKEN + 1))
      LINK_BROKEN_DETAILS+="  - $doc → $link_path
"
    fi
  done < <(grep -oE '\]\(([^)]+)\)' "$doc" 2>/dev/null | sed 's/^](//;s/)$//' | grep -vE '^https?://' | grep -vE '^[#]' | head -5 || true)
done
if [ "$LINK_BROKEN" -gt 0 ]; then
  add_fail "Internal link integrity: 抽查 $CHECKED_LINKS 个链接, $LINK_BROKEN 个断裂:
$LINK_BROKEN_DETAILS"
elif [ "$CHECKED_LINKS" -gt 0 ]; then
  add_pass "Internal link integrity: 抽查 $CHECKED_LINKS 个链接全部有效"
else
  add_warn "Internal link integrity: 未发现 internal 链接 (或文档无 mandatory docs)"
fi
echo ""

# ============================================================
# Report output
# ============================================================
cat > "$REPORT_FILE" <<EOF
# Document Completeness Check Report

> **Version**: ${VERSION}
> **Check tool**: \`scripts/gate/check_document_completeness.sh\`
> **Reference**: \`docs/governance/DOCUMENT_COMPLETENESS_CHECK.md\` v1.1.0
> **Generated**: $(date -u '+%Y-%m-%dT%H:%M:%SZ')

---

## Summary

| Result | Count |
|--------|-------|
| ✅ PASS | ${PASS_COUNT} |
| ⚠️  WARN | ${WARN_COUNT} |
| ❌ FAIL | ${FAIL_COUNT} |

EOF

if [ "$FAIL_COUNT" -gt 0 ]; then
  cat >> "$REPORT_FILE" <<EOF
## ❌ Failures

EOF
  for f in "${FAIL_ITEMS[@]}"; do
    echo "$f" | sed 's/^/- /' >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
  done
fi

if [ "$WARN_COUNT" -gt 0 ]; then
  cat >> "$REPORT_FILE" <<EOF
## ⚠️ Warnings

EOF
  for w in "${WARN_ITEMS[@]}"; do
    echo "$w" | sed 's/^/- /' >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
  done
fi

cat >> "$REPORT_FILE" <<EOF
## ✅ Passes

EOF
for p in "${PASS_ITEMS[@]}"; do
  echo "$p" | sed 's/^/- /' >> "$REPORT_FILE"
  echo "" >> "$REPORT_FILE"
done

cat >> "$REPORT_FILE" <<EOF

---

## Mandatory Documents (per DOC_CHECK_CORRECTION_RULES.md / DOCUMENT_COMPLETENESS_CHECK.md)

| Path | Min Size | Purpose |
|------|----------|---------|
| \`docs/releases/${VERSION}/QUICK_START.md\` | 5K | 5 分钟快速开始 |
| \`docs/releases/${VERSION}/FEATURE_MATRIX.md\` | 15K | 完整功能矩阵 |
| \`docs/releases/${VERSION}/RELEASE_NOTES.md\` | 10K | 发布说明 |
| \`docs/releases/${VERSION}/DEPLOYMENT_GUIDE.md\` | 15K | 部署指南 |
| \`docs/releases/${VERSION}/MIGRATION_GUIDE.md\` | 10K | 升级路径 |
| \`docs/releases/${VERSION}/INSTALL.md\` | 5K | 安装说明 |
| \`docs/releases/${VERSION}/CHANGELOG.md\` | 1K | 变更日志 |
| \`docs/releases/${VERSION}/EVALUATION_REPORT.md\` | 1K | 版本评估 |

---

*Report: \`${REPORT_FILE}\`*
*Tool version: check_document_completeness.sh v1.0.0*
EOF

echo "=================================================="
echo "Report: $REPORT_FILE"
echo "PASS=$PASS_COUNT  WARN=$WARN_COUNT  FAIL=$FAIL_COUNT"
echo "=================================================="

if [ "$FAIL_COUNT" -gt 0 ]; then
  echo ""
  echo "❌ Document completeness check FAILED — $FAIL_COUNT 个 mandatory docs 缺失或不达标"
  exit 1
else
  echo ""
  echo "✅ Document completeness check PASSED"
  exit 0
fi
