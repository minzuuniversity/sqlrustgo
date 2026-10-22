#!/usr/bin/env bash
# =============================================================================
# check_pr_template_sections.sh — PR Body "5-类文档" Section Gate (issue #5458)
# =============================================================================
# Purpose: Verify that an actual PR body carries all five required document
#          sections, using the `<!-- section:NAME -->` anchors defined in
#          .gitea/pull_request_template.md.
# Coverage: AGENTS.md §7 step 10 ("PR body 必填 5 类文档 + Test Plan + Evidence")
# Principle: P3 (测试必审 — a declaration must be checkable) /
#            ADR-008 Policy 1 (a PASS claim must be limited to what was checked)
# Verify:   reads a PR body and confirms the 5 anchors are present.
#
# Why this gate exists (issue #5458):
#   AGENTS.md required every PR body to carry "5 类文档 (PR_TEMPLATE.md)",
#   but:
#     1. the cited path did not exist, and
#     2. the "5 类文档" had no machine-readable definition, and
#     3. nothing checked whether an ACTUAL PR body carried them.
#   The template listed the five names, and
#   tests/integration/sql/pr_template_and_full_gate_test.rs asserted the
#   TEMPLATE contains those names — but a template containing a name says
#   nothing about whether a given PR body filled it in.
#
#   This gate closes gap 3: it reads a real PR body and checks it.
#
# Usage:
#   bash scripts/gate/check_pr_template_sections.sh --pr-body <file>
#   cat body.md | bash scripts/gate/check_pr_template_sections.sh
#
# Exit codes:
#   0 = PASS — all five sections present
#   1 = FAIL — one or more sections missing (each is named)
#   2 = ENV_ERROR — the PR body is empty or cannot be read
#
# DRIFT is NOT PASS (ADR-006 P14).
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# --- ADR-014 5 evidence fields -------------------------------------------------
SOURCE_AGENT="${SOURCE_AGENT:-opencode-z440}"
SOURCE_RUN="${SOURCE_RUN:-opencode-z440-20261022-pr-sections}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

REQUIRED_SECTIONS=(SPEC TEST_PLAN TEST_DESIGN REVIEW ACCEPTANCE)

usage() {
    cat <<'EOF'
Usage: bash scripts/gate/check_pr_template_sections.sh [--pr-body <file>]

With no --pr-body, the PR body is read from stdin.
EOF
}

PR_BODY_FILE=""
while [ $# -gt 0 ]; do
    case "$1" in
        --pr-body) PR_BODY_FILE="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown option: $1"; usage; exit 2 ;;
    esac
done

echo "=== PR Body 5-类文档 Section Check (issue #5458) ==="
echo "  source_agent: $SOURCE_AGENT"
echo "  source_run:   $SOURCE_RUN"
echo "  timestamp:    $TIMESTAMP"
echo

BODY=""
if [ -n "$PR_BODY_FILE" ]; then
    if [ ! -f "$PR_BODY_FILE" ]; then
        echo "❌ PR body file not found: $PR_BODY_FILE"
        echo "   Cannot judge — DRIFT, not PASS."
        exit 2
    fi
    BODY=$(cat "$PR_BODY_FILE")
else
    BODY=$(cat)
fi

if [ -z "$BODY" ]; then
    echo "❌ PR body is empty."
    echo "   Cannot judge — DRIFT, not PASS."
    exit 2
fi

MISSING=()
PRESENT=0
for section in "${REQUIRED_SECTIONS[@]}"; do
    if printf '%s' "$BODY" | grep -qF "<!-- section:${section} -->"; then
        PRESENT=$((PRESENT + 1))
        echo "[PASS] section:${section}"
    else
        MISSING+=("$section")
        echo "[FAIL] section:${section} — marker not found in the PR body"
    fi
done

echo
echo "--- Summary ---"
echo "  required sections: ${#REQUIRED_SECTIONS[@]}"
echo "  present:           $PRESENT"
echo "  missing:           ${#MISSING[@]}"

if [ "${#MISSING[@]}" -gt 0 ]; then
    echo
    echo "❌ FAIL: missing section(s): ${MISSING[*]}"
    echo "   See .gitea/pull_request_template.md §一 for the required markers."
    exit 1
fi

echo
echo "✅ PASS: all ${#REQUIRED_SECTIONS[@]} required sections are present."
exit 0
