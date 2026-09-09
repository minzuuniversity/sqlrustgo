#!/bin/bash
# scripts/gate/check_no_log_tbl_json.sh
#
# v4.0.0 commit hygiene gate (user rule 2026-09-08):
#   No *.log, *.tbl, *.json files in commits.
#   Single file < 100MB (GitHub compatibility).
#
# Usage:
#   scripts/gate/check_no_log_tbl_json.sh [<commit-ish>]
#       - Default: HEAD
#       - With <commit-ish>: diff against HEAD^..<commit-ish>
#
# Exit codes:
#   0 — PASS
#   1 — FAIL (file-extension violation or size violation)
#   2 — Misuse (wrong arguments)

set -e

if [ $# -gt 1 ]; then
    echo "Usage: $0 [<commit-ish>]"
    echo "  Default: HEAD"
    echo "  Examples:"
    echo "    $0               # check HEAD"
    echo "    $0 HEAD          # check HEAD explicitly"
    echo "    $0 HEAD~1..HEAD  # check last commit diff"
    echo "    $0 abc1234       # check single commit"
    exit 2
fi

TARGET="${1:-HEAD}"

# Ensure target is valid
if ! git rev-parse --verify "$TARGET^{commit}" >/dev/null 2>&1; then
    if [[ "$TARGET" == *".."* ]]; then
        # diff range — verify both ends
        LEFT=$(echo "$TARGET" | cut -d. -f1)
        RIGHT=$(echo "$TARGET" | cut -d. -f3)
        if ! git rev-parse --verify "$LEFT^{commit}" >/dev/null 2>&1 ||
           ! git rev-parse --verify "$RIGHT^{commit}" >/dev/null 2>&1; then
            echo "ERROR: $TARGET is not a valid commit reference"
            exit 2
        fi
    else
        echo "ERROR: $TARGET is not a valid commit reference"
        exit 2
    fi
fi

# Allowlist: small JSON config files that are legitimate
ALLOWLIST_REGEX='^(
    Cargo\.toml
  | package\.json
  | tsconfig\.json
  | .+\.config\.json
  | docs/governance/.*\.json
  | docs/releases/[^/]+/manifest\.json
  | docs/releases/[^/]+/evidence/[^/]+/summary\.json
)$'

# Extension whitelist regex (forbidden extensions)
FORBIDDEN_REGEX='\.log$|\.tbl$'

# Files to inspect
if [[ "$TARGET" == *".."* ]]; then
    FILES=$(git diff --name-only --diff-filter=AM "$TARGET" 2>/dev/null)
elif [ "$TARGET" = "HEAD" ]; then
    # Single commit: compare HEAD against HEAD^
    FILES=$(git diff --name-only --diff-filter=AM "HEAD^..HEAD" 2>/dev/null)
else
    # Single commit reference: compare against its parent
    FILES=$(git diff --name-only --diff-filter=AM "${TARGET}^..${TARGET}" 2>/dev/null)
fi

if [ -z "$FILES" ]; then
    echo "PASS: no files changed in $TARGET"
    exit 0
fi

VIOLATIONS=0
SIZE_VIOLATIONS=0
LOG_TBL_VIOLATIONS=0
JSON_VIOLATIONS=0

echo "Checking $TARGET for log/tbl/json violations and file size..."

while IFS= read -r file; do
    if [ -z "$file" ]; then continue; fi

    # Check 1: forbidden extensions .log and .tbl (NO allowlist)
    if [[ "$file" =~ $FORBIDDEN_REGEX ]]; then
        echo "  FAIL: $file (forbidden extension .log/.tbl)"
        LOG_TBL_VIOLATIONS=$((LOG_TBL_VIOLATIONS + 1))
        VIOLATIONS=$((VIOLATIONS + 1))
        continue
    fi

    # Check 2: .json files (with allowlist)
    if [[ "$file" =~ \.json$ ]]; then
        # Check if path is in allowlist
        # Strip leading "./" if present
        clean_file="${file#./}"
        if [[ "$clean_file" =~ $ALLOWLIST_REGEX ]]; then
            # Allowed small config file
            echo "  OK (allowlist): $file"
        else
            echo "  FAIL: $file (forbidden .json extension)"
            JSON_VIOLATIONS=$((JSON_VIOLATIONS + 1))
            VIOLATIONS=$((VIOLATIONS + 1))
            continue
        fi
    fi

    # Check 3: file size < 100MB
    if [ -f "$file" ]; then
        SIZE=$(stat -c %s "$file" 2>/dev/null || stat -f %z "$file" 2>/dev/null || echo 0)
        if [ "$SIZE" -gt 104857600 ]; then
            SIZE_MB=$(echo "scale=2; $SIZE / 1024 / 1024" | bc 2>/dev/null || echo "$((SIZE / 1024 / 1024))")
            echo "  FAIL: $file (size ${SIZE_MB}MB > 100MB GitHub limit)"
            SIZE_VIOLATIONS=$((SIZE_VIOLATIONS + 1))
            VIOLATIONS=$((VIOLATIONS + 1))
            continue
        fi
    fi

done <<< "$FILES"

echo ""
echo "=== Summary ==="
echo "  Files checked: $(echo "$FILES" | wc -l | tr -d ' ')"
echo "  log/tbl violations: $LOG_TBL_VIOLATIONS"
echo "  json violations: $JSON_VIOLATIONS"
echo "  size violations: $SIZE_VIOLATIONS"
echo "  total violations: $VIOLATIONS"

if [ "$VIOLATIONS" -gt 0 ]; then
    echo ""
    echo "❌ FAIL: $VIOLATIONS violation(s) detected"
    echo "User rule (2026-09-08): no *.log, *.tbl, *.json files in commits."
    echo "Single file size < 100MB required (GitHub compatibility)."
    echo "For evidence docs, use *.json.gz or external object storage."
    exit 1
fi

echo "✅ PASS"
exit 0