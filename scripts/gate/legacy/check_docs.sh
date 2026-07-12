#!/usr/bin/env bash
# STATUS: DEPRECATED — see scripts/gate/README.md
# Reason: not invoked by any active gate (audit 2026-06-04)
# Action:  do not add new callers; restore via git history if needed

# v3.8.0+ Documentation Gate Check
# Bash 3.2 compatible (macOS default)
#
# Checks:
#   1. Core documentation files (README, CHANGELOG, CONTRIBUTING)
#   2. v3.8.0 release docs presence
#   3. Governance docs presence
#   4. Markdown link validity
#
# v1.0 hardcoded checks (SECURITY_REPORT, INSTALL_TEST) were removed —
# v1.0 GA pre-dates this script by 2+ years. v1.0 docs are historical.
# See SPEC-010 for migration details.

set -e

echo "=== v3.8.0 Documentation Gate Check ==="
echo "Bash version: $BASH_VERSION"
echo ""

MISSING_DOCS=()

# Core documentation
CORE_DOCS=(
    "README.md"
    "CHANGELOG.md"
    "CONTRIBUTING.md"
    "docs/releases/VERSION_HISTORY.md"
)

# v3.8.0 release docs
V380_DOCS=(
    "docs/releases/v3.8.0/DEVELOPMENT_PLAN.md"
    "docs/releases/v3.8.0/ARCHITECTURE_DECISIONS.md"
    "docs/releases/v3.8.0/alpha/ALPHA_GATE_CONTRACT.md"
    "docs/releases/v3.8.0/alpha/ALPHA_GATE_REPORT.md"
    "docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md"
)

# Governance docs
GOV_DOCS=(
    "docs/governance/RELEASE_LIFECYCLE.md"
    "docs/governance/AI_COLLABORATION.md"
    "docs/governance/adr/ADR-001-truthfulness-framework.md"
)

echo "Checking core documentation files..."
for doc in "${CORE_DOCS[@]}" "${V380_DOCS[@]}" "${GOV_DOCS[@]}"; do
    if [ ! -f "$doc" ]; then
        MISSING_DOCS+=("$doc")
        echo "  ❌ $doc missing"
    else
        echo "  ✅ $doc exists"
    fi
done

if [ ${#MISSING_DOCS[@]} -gt 0 ]; then
    echo ""
    echo "❌ Missing documentation files:"
    for doc in "${MISSING_DOCS[@]}"; do
        echo "  - $doc"
    done
    exit 1
fi

# Check markdown links
echo ""
echo "Checking documentation links..."

MARKDOWN_FILES=$(find docs -name "*.md" -type f 2>/dev/null)

BROKEN_LINKS=()
for file in $MARKDOWN_FILES; do
    # Extract markdown links [text](url)
    LINKS=$(grep -oE '\[[^]]*\]\([^)]+\)' "$file" 2>/dev/null | \
            sed -E 's/.*\(([^)]+)\).*/\1/' | grep -v '^http' | grep -v '^#')
    for link in $LINKS; do
        # Skip anchors
        if [[ "$link" == \#* ]]; then continue; fi
        # Check if file/dir exists
        target="$(dirname "$file")/$link"
        if [ ! -e "$target" ]; then
            BROKEN_LINKS+=("$file -> $link")
        fi
    done
done

if [ ${#BROKEN_LINKS[@]} -gt 0 ]; then
    echo "  ⚠️  Potential broken links found:"
    for link in "${BROKEN_LINKS[@]}"; do
        echo "  - $link"
    done
else
    echo "  ✅ No broken links found"
fi

echo ""
echo "=== Documentation Gate Check Complete ==="
echo "Core docs: ✅ All present"
echo "Links: $(if [ ${#BROKEN_LINKS[@]} -eq 0 ]; then echo "✅ PASS"; else echo "⚠️  ${#BROKEN_LINKS[@]} warnings"; fi)"
